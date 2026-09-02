# Sweep api-audit: Public API surface audit of before and suanpan

## Method and coverage

This pass disputes the api-audit sweep's report finding by finding. For each of
the sweep's 23 findings I opened the cited lines with `awk`/`cat -n`, checked
the claim against the code, and looked for a recorded rationale in git history
(`git log -S`, `git show`, `git blame`), the before-prefixed `.agent-notes/`
directories, `crates/before/AGENTS.md`, and the header of `.cargo/mutants.toml`.
Mechanical checks run:

- Greps over `crates/before/src` and `crates/suanpan/src`: `must_use` (one hit,
  `party/ops/build.rs:39`), `non_exhaustive` (none), `# Errors`/`# Panics`
  (listed below), `Overlap` producers (only `clock.rs`), the six typos, and the
  words `mint`/`honest`.
- Dependency sources read, not run: `thiserror-impl-2.0.18/src/prop.rs`
  (the `source_field` rule), `dashu-int-0.5.0/src/ubig.rs` and `src/repr.rs`
  (`as_words` returns the normalized word slice), and the Rust 1.88 `core`
  sources for `Option`/`Result` (`#[must_use]` is on `Result` only).
- Rendered docs: the sweep's own `cargo doc` output under `target/doc/before/`
  (files dated Sep 1 22:43, produced at this commit) was re-inspected for
  `all.html`, `iter/index.html`, `struct.Clock.html`, `struct.Span.html`, and
  `struct.Version.html`. I did not re-run rustdoc.
- Git history: `crates/before/AGENTS.md` (blame and log), the
  `implementation` module (`67970b75` added, `22cdfbe1` deleted), the "Law of
  Disjointness" phrase (`46184a6a6` added, `a431eaf1d` removed), and the
  suanpan "no from-value constructor" sentence (`7ab518ce1`).

No cargo, just, test, or bench command was run; the two permitted test
invocations were not needed. No file under the repository was modified.

What this pass could not see: I did not audit the full private-doc footprint of
"mint" (it recurs across the skyline and meter modules, outside the public
surface this sweep covers), and I did not re-derive the surface-totality
pincer's item list beyond reading the rendered `all.html`.

Outcome: every sweep finding survives at its cited lines. Two are reframed with
factual corrections (api-audit-1: `Option` carries no `#[must_use]`, so
`Party::without` is not covered as the sweep said; api-audit-5: rustdoc does
form the `Clock::forks` link, with the stray backtick inside the link text).
One severity is lowered (api-audit-2). Nothing is dropped outright.

## Findings

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

### api-audit-2: Decode::Io interpolates the io::Error into Display and returns None from Error::source()
- Where: crates/before/src/error.rs:89-91 (related: party.rs:625, clock.rs:789, version.rs:1112, version/rank.rs:463, version/ranked.rs:273)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read error.rs in full: no `#[source]`, `#[from]`, or field named `source`; read `thiserror-impl-2.0.18/src/prop.rs:97-105`, the derive's source-field rule; `Cargo.lock` resolves before's thiserror to 2.0.18); executed: no
- Verification: confirmed; severity lowered from the sweep's medium: the variant's field is public and pattern-matchable, so structured access exists by `match`; only the `dyn Error` chain (anyhow/eyre reports, `ErrorKind` recovery through `source()`) is cut; history: no-rationale-found
- Owner-gated: no: the variant shape is unchanged; only `Display` text and `source()` change

thiserror implements `source()` as the field annotated `#[source]`/`#[from]` or
named `source`, else `None`. `Io(io::Error)` has none of these, so a reporter
walking the chain sees one flat string and loses the `io::Error`. Interpolating
`{0}` into `Display` while also exposing it as the source would duplicate the
text in chained reporters, which is why the idiomatic spelling separates them.

Evidence:

        89	    /// The underlying reader failed.
        90	    #[error("read error: {0}")]
        91	    Io(io::Error),

    thiserror-impl-2.0.18/src/prop.rs:
     97:fn source_field<'a, 'b>(fields: &'a [Field<'b>]) -> Option<&'a Field<'b>> {
     99:        if field.attrs.from.is_some() || field.attrs.source.is_some() {
    105:            MemberUnraw::Named(ident) if ident == "source" => return Some(field),

Resolution: `#[error("read error")] Io(#[source] io::Error)`, or `#[from]`, which also yields `From<io::Error>` and lets each `.map_err(Decode::Io)?` become `?` (party.rs:625, clock.rs:789, version.rs:1112, rank.rs:463, ranked.rs:273). Acceptance: a unit test constructs `Decode::Io(io::Error::new(ErrorKind::Other, "x"))` and asserts `std::error::Error::source(&e).is_some()`; `Display` no longer embeds the inner message.
Construction: in `crates/before/src/error/tests.rs` (or the existing error tests), `assert!(std::error::Error::source(&Decode::Io(io::Error::other("x"))).is_some())` fails at this commit and passes after the attribute lands.

### api-audit-3: The crate guidepost names an `implementation` module and a "Law of Disjointness" that no longer exist
- Where: crates/before/AGENTS.md:5-7 (related: crates/before/src/lib.rs:218-256)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn 'mod implementation' crates/before/src`: no hits; `grep 'Law of Disjointness' crates/before/src/lib.rs`: no hits; `git show --stat 22cdfbe1` lists `crates/before/src/implementation.rs | 270 -------------` and its lib.rs hunk removes `pub mod implementation;`; `git show a431eaf1d` removes the line `//! Interval tree clocks are correct only under the Law of Disjointness: no` and adds the Causal Singularity and Identity Linearity rules); executed: no
- Verification: confirmed; history: deliberate-but-expired: `f204638d` (2026-07-27, "fix ghost path in the crate guidepost") deliberately pointed the guidepost at the then-new public `implementation` module; `a431eaf1d` (2026-08-04) renamed the model section to "Safety rules" and `22cdfbe1` (2026-08-17) deleted the module, neither touching AGENTS.md
- Owner-gated: no

The first file a contributor reads sends them to a module that was deleted and
to a section heading that was renamed. Both the root and crate AGENTS.md forbid
references to code that no longer exists.

Evidence:

         5	crate docs for the model (`Party`/`Version`/`Clock`, the Law of
         6	Disjointness) and the public `implementation` module for the design essay,
         7	`version/skyline.rs` for the stored coding and its operation kernels, and

Resolution: point the model reference at the crate docs' "Safety rules" section (Causal Singularity, Identity Linearity), and either delete the `implementation` pointer or name where the design essay now lives (`version/skyline.rs` and `testing/validation_index.rs` are the surviving homes). Acceptance: every module and heading named in crates/before/AGENTS.md resolves to an existing item or heading.

### api-audit-4: Crate docs attribute the causal ordering to PartialEq instead of PartialOrd
- Where: crates/before/src/lib.rs:277-279 (related: lib.rs:30, version.rs:58-60, crates/before/README.md:281)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read lib.rs:25-35 and 275-279, version.rs:50-60, and the comparison matrix at version.rs:1713-1760; `grep 'describes a causal ordering' crates/before/README.md` hits line 281, so the derived README carries the same sentence); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`a <= b` is `PartialOrd`; the types table at lib.rs:30 and the `Version` docs at
version.rs:58 name it correctly. The crate deliberately splits equality (a byte
compare) from ordering (the causal sweep), so naming the wrong trait here
misdirects exactly the reader who goes looking for the impl.

Evidence:

       277	//! [`Version`]'s [`PartialEq`] describes a causal ordering: `a <= b` tests
       278	//! containment of history, and two versions with no containing order are
       279	//! [`concurrent`](Version::concurrent). Three tools extend it:

Resolution: replace `PartialEq` with `PartialOrd` in the sentence, then `just readme` to regenerate crates/before/README.md. Acceptance: lib.rs:277 and README.md:281 name `PartialOrd`.

### api-audit-5: iter module doc has an unclosed code span; the Clock::forks link text carries a literal backtick
- Where: crates/before/src/iter.rs:9-10
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read iter.rs; inspected the sweep's rendered `target/doc/before/iter/index.html`); executed: yes: python read of the rendered HTML, which contains `<a href="../struct.Clock.html#method.forks" title="method before::Clock::forks">`Clock::forks</a>`
- Verification: reframed: the sweep said the link does not form and only the first link is present; the rendered page shows rustdoc does form the second link, with the backtick as literal text inside it and no `<code>` formatting. The visible symptom (a stray backtick, unformatted name) stands; history: no-rationale-found
- Owner-gated: no

Evidence:

         9	//! See [`Party::forks`](crate::Party::forks) and
        10	//! [`Clock::forks](crate::Clock::forks).

Resolution: add the missing backtick. Acceptance: the rendered iter module docs show both names in code formatting with no literal backtick.

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

### api-audit-7: auto_traits.rs claims to pin every public API type but omits Limbs, TooWide, the shape types, and the polarity markers
- Where: crates/before/src/auto_traits.rs:1-32 (related: shape.rs:88, shape.rs:108, shape.rs:118, shape.rs:128, shape.rs:146, shape.rs:197, shape.rs:255, shape.rs:345, version/ticks.rs:125, error.rs:54, causally/polarity.rs:225-240, surfacecheck/src/extract.rs:21-24, surfacecheck/src/extract.rs:267-269)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (compared the `crate::...` names in auto_traits.rs against the Structs/Enums sections of the sweep's rendered `target/doc/before/all.html`, restricted to ungated items); executed: yes: python extraction of the item lists from all.html
- Verification: confirmed, and strengthened: `surfacecheck/src/extract.rs:21-24` excludes compiler-synthesized auto-trait impls from the surface census on the premise that "`src/auto_traits.rs` asserts `Send + Sync + Unpin` on every public API type at compile time", so the census's exclusion rests on a totality the roster does not have; history: no-rationale-found
- Owner-gated: no

Ungated public types with no `assert_impl_all!` line: `Limbs`, `error::TooWide`,
`shape::{Plateau, Rise, Region, Cell, Plateaus, Regions, Overlay, Cells}`, and
`causally::{Down, Up, Neutral}` (the last three are covered for `Send + Sync`
by the `Polarity: sealed::Sealed + Send + Sync + 'static` supertrait bound, not
for `Unpin`). The shape iterators hold `VersionWalk`/`PartyWalk` fields, whose
auto traits are exactly what a pin exists to guard.

Evidence:

         1	//! Compile-time pins on the auto traits of every public API type.

    surfacecheck/src/extract.rs:
        21	//!   [`crate::census::TRAIT_IMPLS`]. Compiler-synthesized auto-trait
        22	//!   impls are excluded here because `before` pins those guarantees
        23	//!   directly (`src/auto_traits.rs` asserts `Send + Sync + Unpin` on
        24	//!   every public API type at compile time); blanket impls are excluded

Resolution: add the missing `assert_impl_all!` lines (`shape::Cell<1>` and `shape::Cells<'static, 1>` for the const-generic pair), or derive the roster from the surface census so it cannot drift; otherwise narrow the module doc and the two surfacecheck comments to what is pinned. Acceptance: every struct and enum in rustdoc's `all.html` for the default feature set appears in an `assert_impl_all!` line, or a committed check compares the two lists.

### api-audit-8: `# Errors` sections cover about half the fallible public entries
- Where: crates/before/src/version.rs:1095-1110 (related: party.rs:558-574, party.rs:602-623, party.rs:763-789, party.rs:863-908, clock.rs:742-756, clock.rs:765-787, clock.rs:925-952, clock.rs:971, version.rs:1031-1047, version.rs:1071-1093, version.rs:1440-1511, version/ranked.rs:185, version/ranked.rs:234, version/ticks.rs:185-190, version/ticks.rs:199-213)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '# Errors' crates/before/src` excluding test and instrument modules: clock.rs:199,232,303,345; party.rs:280,310; span.rs:120; version/rank.rs:403,435; version/ranked.rs:241; span/wire.rs:52,79. Cross-checked against every `Result`-returning `pub fn`, `FromStr`, and `TryFrom` in the public defining files); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The section exists for `Span::new`, `Clock::join/join_all/sync/sync_all`,
`Party::join/join_all`, `Span::encode_to/decode`, `Rank::encode_to/decode`, and
`Ranked::decode`, and it is absent from `Version::decode`, `Party::decode`,
`Clock::decode`, the `encode_to` trio, `Version::encode_rank_to`,
`Ranked::encode_to/encode_rank_to`, `TryFrom<&Ticks> for u64`, and every
`FromStr`/`TryFrom` impl (which describe rejection in prose without the
section). The decode trio are the entries most likely to meet untrusted bytes.

Evidence:

      1095	    /// Decodes a [`Version`] from a reader of canonical bytes.
      1096	    ///
      1097	    /// # Complexity
      1098	    ///
      1099	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_decode.html"))]
      1100	    ///
      1101	    /// Strict validation is one pass over the stream, and the result reuses the read buffer.

Resolution: add `# Errors` to the listed sites, naming the `Decode`/`Parse`/`TooWide` variants each can return; `Rank::decode` (rank.rs:435-445) and `Span::decode` (span/wire.rs:79-85) are the variant-by-variant template, and `Rank::encode_to` (rank.rs:403-406) the one-line template for writers. Acceptance: every `pub fn` or trait impl returning `Result` carries a `# Errors` section.

### api-audit-9: Public docs point at arguments that live nowhere public (OwnSpan monotonicity; Query's missing Eq)
- Where: crates/before/src/span/own.rs:267-270 (related: span/own.rs:11-37, version/own.rs:12-28, version.rs:1669-1676, causally/query.rs:211-215, causally.rs:1-139)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both `Own*` type docs, version.rs:1660-1711, causally.rs in full; `grep -n 'Eq' crates/before/src/causally.rs` matches only `Ordering::Equal` at line 163; `grep -rn monoton` over the span/version files hits only own.rs:268, version.rs:356, version.rs:1676); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`OwnSpan::to_span` says the type's docs carry the monotonicity argument;
neither `OwnSpan`'s (span/own.rs:11-37) nor `OwnVersion`'s (version/own.rs:12-28)
does. The only nearby argument is a private comment whose closing sentence
reads "so it is not monotone under `<=`", with `min_ticks` as its true subject,
so a reader following the pointer meets an apparent contradiction. Separately,
query.rs says "there is deliberately no `Eq`; see the module docs", and the
`causally` module docs never mention it.

Evidence:

       267	    /// One [`OwnVersion::to_version`] per endpoint; the projection is
       268	    /// monotone (the type's docs carry the argument), so the
       269	    /// projected pair is ordered and the construction revalidates
       270	    /// nothing.

      1675	// Projection can still raise `min_ticks` (carving one broad tick into
      1676	// disjoint peaks), so it is not monotone under `<=`.

       211	// Debug renders the module's own expression vocabulary, the only structural
       212	// window into a query (there is deliberately no `Eq`; see the module docs), so

Resolution: state once, in `OwnVersion`'s public docs, that projection is a homomorphism of join and meet and therefore order-preserving (`a <= b` implies `a/p <= b/p`), and point `to_span` there; reword version.rs:1675-1676 so `min_ticks` is the stated subject ("`min_ticks` is not monotone under projection"); add the reason `Query` has no `PartialEq` to `Query`'s type docs and fix the query.rs pointer. Acceptance: each pointer resolves to a paragraph that states the argument.

### api-audit-10: forks(u64::MAX) yields one share fewer than asked; the public docs say "exactly `k`" and the test calls this "documented"
- Where: crates/before/src/party.rs:239-276 (related: clock.rs:159-194, party/forks.rs:78-80, party/forks.rs:105-118, clock/forks.rs:7-9, tests/forks_max.rs:1-11)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `Forks::new`, `Split::new`, `Split::size_hint`, tests/forks_max.rs in full, and all four public docs); executed: no
- Verification: confirmed, with two more sites: the `Forks` type docs at party/forks.rs:80 and clock/forks.rs:9 both say "Yields exactly `k`", and tests/forks_max.rs:1-2 calls the saturation "the documented behavior", which nothing public documents; history: no-rationale-found
- Owner-gated: no

`Forks::new` splits `k.saturating_add(1)` ways and hands the first leaf to the
borrowed party, so at `k == u64::MAX` the iterator reports and yields
`u64::MAX - 1` shares. The saturation is the benign choice and is pinned by
tests/forks_max.rs, but the public contract at four sites promises `n`/`k`
shares unconditionally. Reading `len()` at that input costs O(1), so this is
not the infeasible-work corner the doctrine tolerates undocumented.

Evidence:

       239	    /// Splits `n` balanced shares off this [`Party`], as a lazy
       240	    /// [`ExactSizeIterator`].

        80	/// Yields exactly `k` disjoint shares produced one at a time. The party it

       111	        // The count saturates: at `k == u64::MAX` the residual's headroom is
       112	        // spent and the iterator yields one share fewer than asked.
       113	        let whole = mem::replace(party, Party::anonymous());
       114	        let mut split = Split::new(whole, k.saturating_add(1));

         1	//! `forks(u64::MAX)`: the documented behavior at the split count's
         2	//! saturation boundary, pinned in both profiles.

Resolution: one sentence in `Party::forks`, `Clock::forks`, and both `Forks` type docs ("`k == u64::MAX` saturates: `u64::MAX - 1` shares are yielded, the residual taking the last slot"), after which tests/forks_max.rs's "documented behavior" becomes true; or count in `u128` internally so `k` shares are always yielded. Acceptance: the public `forks` docs state the corner, or `forks(u64::MAX).len()` equals `u64::MAX` on 64-bit.

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

### api-audit-12: suanpan::Limbs withholds the exact size Chunks already knows, so before's Ticks::limbs re-derives it by hand
- Where: crates/suanpan/src/limbs.rs:60-72 (related: crates/suanpan/src/limbs.rs:43-45, crates/before/src/version/ticks.rs:112-149, crates/suanpan/src/claims.rs:86)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read limbs.rs and ticks.rs in full; read `dashu-int-0.5.0/src/ubig.rs:90-94` and `src/repr.rs:226-238`: `as_words` returns the normalized word slice, empty for zero, so `chunks.len()` equals `bits().div_ceil(64)` on both word widths); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: additive trait impls on suanpan's public iterator, and the claims roster row names the trait set

`suanpan::Limbs` wraps `core::slice::Chunks` (which is `ExactSizeIterator +
DoubleEndedIterator + FusedIterator`) yet implements only `Iterator` and
`DoubleEndedIterator` with the default `size_hint` of `(0, None)`. before's
`Limbs` therefore carries a `remaining` counter computed from
`bits().div_ceil(64)` with a `-= 1` on every step and an `expect` on the
`usize` conversion, all to state a number the wrapped iterator already holds.

Evidence:

        60	impl Iterator for Limbs<'_> {
        61	    type Item = u64;
        62	
        63	    fn next(&mut self) -> Option<u64> {
        64	        self.chunks.next().map(pack_limb)
        65	    }
        66	}

       112	    pub fn limbs(&self) -> Limbs<'_> {
       113	        Limbs {
       114	            limbs: suanpan::Limbs::new(&self.0 .0),
       115	            remaining: usize::try_from(self.0.bits().div_ceil(64))
       116	                .expect("a stored count's limb count fits usize"),
       117	        }
       118	    }

Resolution: in suanpan, add `size_hint` delegating to `self.chunks.size_hint()`, `impl ExactSizeIterator for Limbs<'_> {}`, `impl FusedIterator for Limbs<'_> {}`, and update the `FAMILY_SURFACE` row "Limbs iteration (Iterator / DoubleEndedIterator)"; then let before's `Limbs` delegate and drop `remaining`. Acceptance: `suanpan::Limbs::new(&x).len()` equals the yielded count under proptest; before's `Limbs` has no `remaining` field.

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

### api-audit-15: Party::decode's Warning is Clock::decode's text and never names Party
- Where: crates/before/src/party.rs:605-610 (related: clock.rs:768-772, party.rs:10-17)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites side by side); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Evidence:

       605	    /// # Warning
       606	    ///
       607	    /// Serializing a [`Clock`](crate::Clock) circumvents its otherwise
       608	    /// compiler-enforced `!Clone` linearity. Deserializing one can violate
       609	    /// causality. Treat serialization/deserialization boundaries as *moves* of
       610	    /// the [`Clock`](crate::Clock).

Resolution: reword in terms of `Party` (the module doc at party.rs:10-17 already carries the right sentence). Acceptance: the warning names `Party`.

### api-audit-16: error::Overlap's summary names only Clock::sync; Clock::sync_all returns it too
- Where: crates/before/src/error.rs:5-5 (related: clock.rs:322, clock.rs:377)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn 'Overlap' crates/before/src` outside tests and instruments: producers are clock.rs:327, 408, 413, inside `sync` and `sync_all`); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Evidence:

         5	/// Two parties were not disjoint during [`Clock::sync`](crate::Clock::sync).

       377	    pub fn sync_all<'a, I>(&mut self, iter: I) -> Result<&Version, Overlap>

Resolution: "...during [`Clock::sync`] or [`Clock::sync_all`]." Acceptance: both origins named.

### api-audit-17: Doc-link and spelling slips in public rustdoc
- Where: crates/before/src/version.rs:603-604 (related: clock.rs:341, span.rs:413-415, causally.rs:59, causally.rs:81, version/rank.rs:198)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read each cited line; grep for each token); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`Version::span_all` links the word `span` to `Version::meet`; clock.rs:341
"iteratatively"; span.rs:414 "the two `hi` endpoint, where seach"; causally.rs:59
"of a [`Span`]s grant them"; causally.rs:81 "arbitary"; rank.rs:198 "never is
larger".

Evidence:

       603	    /// Prefer this to iteratively [`span`](Version::meet)ing [`Version`]s
       604	    /// one-at-a-time, as it is more efficient.

       341	    /// Prefer this to iteratatively calling [`sync`](Clock::sync), as this is

       414	    /// endpoints and a second to compare the two `hi` endpoint, where seach

Resolution: fix the link target to `Version::span` and the five typos. Acceptance: the sites read correctly.

### api-audit-18: Parameter names drift between signature and prose (n vs k; rhs vs other; version vs other)
- Where: crates/before/src/clock.rs:107-113 (related: clock.rs:126, clock.rs:159-192, party.rs:184-206, party.rs:239-274, iter.rs:4, version/rank.rs:278-298, version/rank.rs:332-354, version.rs:236, version.rs:384-593)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read each site; `grep -n 'pub fn [a-z_]*(&self, other'` in version.rs shows `distance`, `lag`, `join`, `meet`, `span` all take `other`, while `concurrent` takes `version`); executed: no
- Verification: confirmed, with one more site: iter.rs:4 says "`n` shallow shares" for the `k` parameter; history: no-rationale-found
- Owner-gated: no

Evidence:

       107	    /// Advances this [`Clock`] by `n` events for its own [`Party`], returning
       108	    /// the new [`Version`]: byte-identical to `n` sequential
       109	    /// [`tick`](Self::tick)s, computed in a bounded number of passes rather
       110	    /// than `n`.
       111	    ///
       112	    /// The count `k` is any unsigned number, since all can be converted into
       113	    /// [`Ticks`].

       278	    /// The difference `self - rhs`, or [`None`] when `rhs` exceeds `self`.
       298	    pub fn checked_sub(&self, other: &Rank) -> Option<Rank> {

Resolution: one letter per concept (`k` for counts, `other` for the second operand) in both prose and signatures. Acceptance: prose and signature agree at each site.

### api-audit-19: Vocabulary tells in public rustdoc: "mint" for constructing values, "honest" for exact
- Where: crates/before/src/lib.rs:49-49 (related: party.rs:15, party.rs:872, version/rank.rs:1064-1069, version.rs:1624, causally/conjunction.rs:30, causally/conjunction.rs:37, version/rank.rs:550, version/rank.rs:580)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep for `mint` and `honest` over both crates' `src/`); executed: no
- Verification: confirmed for the public sites; the same word recurs widely in private skyline and meter prose (`watermark.rs`, `query/web.rs` has a `fn mint`), which is outside this sweep's public-surface lens and is noted for a vocabulary pass rather than listed here; history: no-rationale-found
- Owner-gated: no

Evidence:

        49	//! // New participants fork off a live clock, never mint themselves.

        15	//! mint a second holder from bytes or notation, and

       872	/// Mints identity exactly as the `u8` literal door does — a test and

      1068	/// exact at any width memory admits, at the honest price of exactness past

Resolution: "create"/"build" for mint; "exact"/"the price of exactness" for honest, at the public sites first. Acceptance: no "mint" for construction and no "honest" as a synonym for exact in public rustdoc.

### api-audit-20: The error module's first sentence is a rhetorical question
- Where: crates/before/src/error.rs:1-1
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read error.rs:1); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The first sentence is what the crate index shows beside `error`.

Evidence:

         1	//! What could possibly go wrong?

Resolution: a descriptive sentence, e.g. "The error types: decode, parse, overlap, crossed-span, and count-width failures." Acceptance: the module listing shows a descriptive sentence.

### api-audit-21: TryFrom<u64> for Version can never fail, and its docs do not say so
- Where: crates/before/src/version.rs:1473-1478 (related: version/skyline/literal.rs:18-23, version.rs:1497-1500)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read version.rs:1461-1511 and literal.rs:17-23: `pub(crate) fn leaf(base: u64) -> BitsBuf` is infallible); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The `Result<_, Parse>` shape is forced by the tuple literal's `Version:
TryFrom<T, Error = Parse>` bound, so the fallibility is structural, not
semantic; a user meets an `unwrap()` with no stated reason.

Evidence:

      1473	impl TryFrom<u64> for Version {
      1474	    type Error = Parse;
      1475	    fn try_from(n: u64) -> Result<Self, Parse> {
      1476	        Ok(Version::from_bits(skyline::literal::leaf(n)))
      1477	    }
      1478	}

Resolution: one sentence in the impl doc: "Never fails; the `Result` is the shape the nested `(n, left, right)` literals compose over." Acceptance: the doc states the impl is total.

### api-audit-22: Hand-maintained item count in a test module doc
- Where: crates/before/tests/foreign_reexport.rs:12-14
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the file in full); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Evidence:

        12	//! source files. Demonstrated: with `pub use bytes::Bytes;` added at
        13	//! the crate root, the surface-totality leg reads the same 197 items
        14	//! and exits clean. `pub extern crate <dep>` and a `pub type` alias of

Resolution: drop the number ("reads the same item count"). Acceptance: no literal count in the sentence.

### api-audit-23: suanpan states there is no from-value constructor without saying why
- Where: crates/suanpan/src/lib.rs:244-248 (related: crates/suanpan/src/claims.rs:20-23)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read lib.rs:230-260 and claims.rs:1-30; `git log -S'no from-value constructor'` finds `7ab518ce1` "suanpan: docs round 3 of the fresh-eyes loop", whose message gives no reason); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: the alternative resolution adds a constructor

The sentence states an absence and no mechanism. The reason the sweep infers
(every entry point is a priced row in the claims roster, and a `From` would be
one more door to price) is plausible but unrecorded anywhere I could find.

Evidence:

       244	//! There is no from-value constructor: build with
       245	//! [`new`](Accumulator::new) (or `Default`) and a single `add_*` call, read out
       246	//! with [`sign_magnitude`](Accumulator::sign_magnitude) (or
       247	//! [`sign_limbs`](Accumulator::sign_limbs) at widths the backend cannot
       248	//! hold).

Resolution: add the clause that names what the absence serves, or add the constructor if no reason survives. Acceptance: the sentence carries its reason.

## Positives

- Linearity is pinned at the type definitions with `static_assertions`
  (party.rs:74, clock.rs:62), and the entire `|` surface a `Clock` participates
  in is pinned in both directions (clock.rs:1072-1082), with the reason for
  each rejected cell stated at the definition. I read all three pins.
- `Split::size_hint` (party/forks.rs:65-73) handles a `u64` count on a target
  whose `usize` is narrower by returning `(usize::MAX, None)`, the standard
  spelling, with the reason in a two-line comment: correct at all scales,
  stated where it lives.
- `Rank::decode`'s `# Errors` section (rank.rs:435-445) and `Span::decode`'s
  (span/wire.rs:79-85) are variant-by-variant and name the flush-byte case;
  `Rank::encode_to` (rank.rs:403-406) shows the one-line form for writers.
  They are the template api-audit-8 asks the rest of the surface to meet.
- tests/forks_max.rs pins the saturation boundary in both profiles, holding
  `len()` and `size_hint()` to an exact value rather than "does not panic".
- tests/doc_hidden.rs and tests/foreign_reexport.rs pin the hidden and
  re-exported surfaces by name; the rendered docs confirm no foreign re-export
  in before and exactly the documented `UBig` re-export in suanpan.
- Every fallible reunion hands identity back rather than dropping it:
  `Party::join`/`Clock::join` return the operand in `Err`, `join_all` returns
  the overlapping set, `Forks`' `Drop` rejoins unconsumed shares, and
  `sync`/`sync_all` leave every clock untouched on overlap (clock.rs:322-336,
  377-390, party/forks.rs:134-148).
- The text, literal, and byte doors each document the linearity hole at the
  entry (`Party::from_str` at party.rs:767-768, `Clock::from_str` at
  clock.rs:928-929, `Clock::decode` at clock.rs:768-772), and
  `dangerously_alias` states the exact protocol it exists for.

## Open questions for Finch

1. The brief lists `claims` among suanpan's public API; at this commit it is
   `#[cfg(test)] mod claims;` (crates/suanpan/src/lib.rs:364-365), a private
   test module. Was the brief's premise stale, or is exposing the roster (as
   before does with `surface` under `meter`) intended?
2. Should `Version::new`/`Party::seed`/`Clock::seed` be `const fn`? `Rank::ZERO`
   and `Ticks::ZERO` are consts. The constructors build from a `static` byte
   via `codec::Bits::from_canonical(bytes::Bytes::from_static(..))`;
   `Bytes::from_static` is const, and `from_canonical` (codec/bits.rs:139-146)
   is a plain `pub(crate) fn` whose only body beyond the struct literal is a
   `debug_assert!(padding_is_canonical(&bits), ..)`, so constness hinges on
   making that predicate const or adding a const static-only door. The
   comments at party.rs:125-127 and version.rs:126-128 explain why the buffer
   is a `static` rather than a `const` (shared address); a `const fn`
   returning a value built from that static is compatible with that.
3. `Query` deliberately has no `PartialEq` (query.rs:211-212, a private
   comment); `Floor`/`Ceiling` likewise. If the reason is that two spellings
   can denote one predicate, is a structural `PartialEq` on the normal form
   (the `and` merge maintains one) acceptable, or is the absence meant to keep
   callers from treating queries as values? api-audit-9 asks for the reason to
   be written down wherever the answer lands.
4. `Clock` and `Span` expose no O(1) encoded length (`Party`/`Version` have
   `as_bytes().len()`); a protocol sizing a frame must `encode()` (allocating)
   or write through a counting writer. Is an `encoded_len()` on the composite
   types wanted, or is `encode_to` into a counting writer the intended spelling?

## Dropped

- Sub-claim of sweep finding [0] that `Party::without` is "already covered by
  `Option`/`Result`'s built-in must_use": dropped; `core::option::Option` has no
  `#[must_use]` (only `Result` does), so `without` needs its own attribute. The
  finding survives as api-audit-1 with that correction.
- Sub-claim of sweep finding [4] that "the link does not form" and "only the
  first link [is] present": dropped; the rendered `iter/index.html` shows both
  links formed, the second with a literal backtick inside its text. The finding
  survives as api-audit-5 (nit) with the corrected symptom.
- No whole finding was dropped: every cited excerpt matched the file at the
  cited lines, and no recorded rationale in git history, `.agent-notes/`,
  `crates/before/AGENTS.md`, or `.cargo/mutants.toml` contradicted a claim.
