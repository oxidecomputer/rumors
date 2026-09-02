# Sweep fresh-eyes: Fresh-eyes user: a scratch application from the public docs alone

## Method and coverage

The sweep built a scratch application against `before` and `suanpan` from their public documentation alone and reported sixteen findings, all documentation or API-shape friction, no correctness defects. This pass disputes each one against the tree at 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (clean working tree).

What ran, mechanically:

- Read in full with line numbers: `crates/before/src/lib.rs`, `error.rs`, `iter.rs`, `causally/convert.rs`, `serde_impls.rs` (lines 1-80); read every cited range in `clock.rs`, `party.rs`, `version.rs`, `span.rs`, `span/wire.rs`, `version/own.rs`, `version/ticks.rs`, `version/rank.rs`, `version/ranked.rs`, `causally.rs`, `causally/forms.rs`, `causally/query.rs`, `clock/forks.rs`, `party/forks.rs`, `shape.rs`, and `README.md`.
- Grepped the crate for: every `FromStr` impl (four: `Clock`, `Version`, `Party`, `Ticks`); every `pub fn decode` (six public plus one private skyline entry); `# Errors` counts per file (`version.rs`: 0, `party.rs`: 2, `clock.rs`: 4, `span.rs`: 1, `rank.rs`: 2, `ranked.rs`: 1); every non-test use of `Overlap` (constructed only in `Clock::sync` and `Clock::sync_all`); `#[source]`/`#[from]` (none in `before` or `suanpan`); `is_human_readable` (absent); the words `mint` and `door` (public sites: `lib.rs:49`, `party.rs:850`, `party.rs:872-873`; roughly two hundred private-prose sites for `door` across `laws.rs`, `meter*`, `codec/*`, `testing/*`, `surface.rs`); `Eq`/`equality` across `causally/` (no module doc states the no-`Eq` rationale).
- Checked history with `git log -S` for the sentences each finding cites (the `PartialEq` sentence, the `Io(io::Error)` variant, the `literal door` wording, the `iter` re-exports, the no-`Eq` comment, the `error` module summary) and searched `.agent-notes/` (the ten `before`-prefixed notes and the rest) for recorded rationales on serde representation, the fork iterator name, `Decode::Io`'s source, `Rank` parsing, and atom `coverage`. None was found; the one relevant commit (e546b6d5e) introduced the "identity-minting door" wording deliberately but records no argument for the terms themselves.
- Read the sweep's own evidence logs (`scratchpad/before/sweeps/fresh-eyes/logs/{check1,run2}.log`) and matched each executed claim to its log line (the eight compile-probe errors; the `TrailingBits`/`Truncated` probes; `source() is_some = false`; the serde_json byte arrays).
- Read the existing rustdoc build at `target/doc/before/` (file dates Sep 1 22:43, later than the HEAD commit; `iter.rs` and `clock/forks.rs` last changed Aug 14) to settle how the fork iterator's return type renders. That settles the sweep's first open question: the signature renders as `pub fn forks(&mut self, k: u64) -> Forks<'_>`, linking to a page titled "Clock in before::iter".

What this pass could not see: it ran no cargo command (the two permitted test invocations were not needed; no finding rests on a runtime claim the sweep's logs and the code do not already settle), and it did not independently re-read `suanpan`, on which the sweep reported no findings.

Verdict: all sixteen findings hold on the code. Two are reframed (Span::decode already carries a `# Errors` section; the iter.rs:10 link resolves and only its backtick is stray), one gains a second defect (the private no-`Eq` comment points at module docs that hold no such rationale), and one is upgraded from an open question to verified (the fork iterator's rendered name). Nothing is dropped whole.

## Findings

### fresh-eyes-1: Crate docs and README name PartialEq as the causal ordering
- Where: crates/before/src/lib.rs:277-279 (related: crates/before/README.md:281-283 (derived), crates/before/src/version.rs:58-60, crates/before/src/lib.rs:30)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read); executed: no
- Verification: confirmed; history: no-rationale-found (a431eaf1d "Doc editing" introduced the sentence)
- Owner-gated: no

The "Comparing and ordering versions" section says `Version`'s `PartialEq` describes the causal ordering. The ordering is `PartialOrd`; the type table one screen above (lib.rs:30) and the `Version` type docs (version.rs:58) say so correctly, and the derived README repeats the slip.

Evidence:

       277	//! [`Version`]'s [`PartialEq`] describes a causal ordering: `a <= b` tests
       278	//! containment of history, and two versions with no containing order are
       279	//! [`concurrent`](Version::concurrent). Three tools extend it:

        58	/// Comparison is **partial** ([`PartialOrd`], not [`Ord`]): two distinct
        59	/// versions can be [`concurrent`](Version::concurrent), and then `a < b`, `a ==
        60	/// b`, and `a > b` are all false.

Resolution: Change `PartialEq` to `PartialOrd` at lib.rs:277, then `just readme` so README.md:281 follows. Acceptance: `grep -n 'PartialEq.*causal' crates/before/src/lib.rs crates/before/README.md` returns nothing and `just readme-check` passes.

### fresh-eyes-2: Version, Party, and Clock decode do not state the whole-input contract and lack # Errors sections
- Where: crates/before/src/version.rs:1095-1110 (related: crates/before/src/clock.rs:765-787, crates/before/src/party.rs:602-623, crates/before/src/error.rs:77-85; the complete sections to copy from: crates/before/src/span/wire.rs:79-89, crates/before/src/version/rank.rs:435-445, crates/before/src/version/ranked.rs:241-247)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (the sweep's run2.log lines 46-50: `Version::decode(value ++ extra) = Err(TrailingBits)`, `Clock::decode(clock ++ version) = Err(TrailingBits)`, `Version::decode(&[]) = Err(Truncated)`, `Clock::decode(party bytes) = Err(Truncated)`; this pass read the three bodies: each calls `read_to_end` then `require_marker_padding`); executed: yes, by the sweep's scratch run, matched to its log
- Verification: reframed: the sweep's resolution also named `Span::decode`, which already carries a complete `# Errors` section (span/wire.rs:79-89); the gap is exactly the three sites here; history: no-rationale-found
- Owner-gated: no for the documentation; the prefix-decoding entry is an owner-gated suggestion

`Version::decode`, `Party::decode`, and `Clock::decode` take a `Read`, consume it to end, and reject any byte past the value as `Decode::TrailingBits`; none of the three docs says so, and none has a `# Errors` section, while `Span::decode`, `Rank::decode`, and `Ranked::decode` carry complete ones. The whole-input rule is discoverable today only on the `Decode::TrailingBits` variant.

Evidence:

      1095	    /// Decodes a [`Version`] from a reader of canonical bytes.
      1096	    ///
      1097	    /// # Complexity
      1098	    ///
      1099	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_decode.html"))]
      1100	    ///
      1101	    /// Strict validation is one pass over the stream, and the result reuses the read buffer.
      ...
      1110	    pub fn decode<R: Read>(mut reader: R) -> Result<Self, Decode> {
      1111	        let mut buf = Vec::new();
      1112	        reader.read_to_end(&mut buf).map_err(Decode::Io)?;

        79	    /// # Errors
        80	    ///
        81	    /// - [`Decode::Truncated`]: the bytes end before the composite does —
        ...
        89	    /// - [`Decode::Io`]: the reader itself fails.

Resolution: Add a `# Errors` section to `Version::decode`, `Party::decode`, and `Clock::decode` in the form `Span::decode` uses, and state in each that the reader is read to end and must hold exactly one value. Owner-gated suggestion, separately: a prefix-decoding entry (for example `decode_prefix(&[u8]) -> Result<(Self, &[u8]), Decode>`) for callers who frame values without a length prefix; the borsh feature already relies on the encodings being prefix-free. Acceptance: `grep -c '# Errors' crates/before/src/version.rs` is at least 1 and each of the three `decode` docs names `TrailingBits` for spurious input.

### fresh-eyes-3: The serde representation is undocumented (JSON emits a numeric byte array)
- Where: crates/before/src/lib.rs:392-393 (related: crates/before/src/serde_impls.rs:20-24, 33-37, 46-50, 64-68; crates/before/src/lib.rs:263-264; crates/before/README.md:386-387 (derived))
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the sweep's run2.log lines 33-34: `serde_json version: [41,42,91,23,118,92,74,75,128]`, `serde_json party: [32]`; this pass read serde_impls.rs: every `Serialize` impl calls `serialize_bytes(&self.encode())` and every `Deserialize` goes through `<Vec<u8>>::deserialize`, and `is_human_readable` appears nowhere in the crate); executed: yes, by the sweep's scratch run, matched to its log
- Verification: confirmed; history: no-rationale-found (no agent note or commit records a decision on the human-readable form)
- Owner-gated: no for stating the representation; yes for changing it

The `serde` feature docs say which types implement the traits and (at lib.rs:263-264) that serde serializes "through the same encodings"; a serde_json user gets each byte as a JSON number, several times the wire size, and cannot learn that without running the code.

Evidence:

       392	//! - **`serde`:** `Serialize`/`Deserialize` for [`Party`], [`Version`],
       393	//!   [`Clock`], [`Rank`], [`Ranked`], and [`Span`].

        20	impl Serialize for Party {
        21	    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        22	        s.serialize_bytes(&self.encode())
        23	    }
        24	}

Resolution: State in the feature docs that every type serializes as the bytes of its canonical encoding via `serialize_bytes`, so binary formats carry the wire bytes and human-readable formats carry a sequence of integers; then `just readme`. Whether to branch on `Serializer::is_human_readable()` and emit the paper notation is an owner decision (see Open questions). Acceptance: the `serde` bullet names the representation.

### fresh-eyes-4: 'mint' and the 'door' metaphor in public rustdoc
- Where: crates/before/src/party.rs:872-873 (related: crates/before/src/party.rs:850, crates/before/src/lib.rs:49, crates/before/README.md:53 (derived))
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read; `grep -rnw 'mint\|mints\|door\|doors'` over the crate located the public sites and roughly two hundred private-prose uses of `door`); executed: no
- Verification: confirmed; history: no-rationale-found (e546b6d5e introduced "identity-minting door" deliberately and uses the terms throughout its message, but records no argument for the words themselves and anchors them nowhere)
- Owner-gated: no

Public rustdoc uses "mint" for constructing a value and promotes "door" to jargon without anchoring it to an identifier or defining it by contrast. At party.rs:850 the plain phrase already sits beside the coinage.

Evidence:

       850	/// Like every literal door, this *creates* identity tied to no existing handle.

       872	/// Mints identity exactly as the `u8` literal door does — a test and
       873	/// fresh-universe door ([Safety rules](crate#safety-rules)).

        49	//! // New participants fork off a live clock, never mint themselves.

Resolution: At party.rs:872-873 write "Creates identity exactly as the `u8` literal does: a constructor for tests and fresh universes ([Safety rules](crate#safety-rules))"; at party.rs:850 drop "Like every literal door,"; at lib.rs:49 write "never create themselves", then `just readme`. The private-prose uses of `door` are a separate question for the owner (below). Acceptance: `grep -rnw 'mint\|mints\|door' crates/before/src/party.rs crates/before/src/lib.rs` finds no line beginning `///` or `//!`.

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

### fresh-eyes-7: Projection docs spell an owned-operand `/` that does not compile
- Where: crates/before/src/version/own.rs:12 (related: crates/before/src/span.rs:69-71; correct spellings at crates/before/src/version/own.rs:1, crates/before/src/version.rs:703, crates/before/src/span.rs:47, crates/before/src/version.rs:1703)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the sweep's check1.log lines 64 and 77: `error[E0369]: cannot divide 'Version' by '&before::Party'` and `cannot divide 'Span<'_>' by '&before::Party'`; this pass read the only `Div<&Party>` impl for versions, on `&'a Version` at version.rs:1703); executed: yes, by the sweep's compile probes, matched to its log
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`OwnVersion`'s first sentence writes `v / &p` and the `Span` algebra section writes `s / &p`, but `Div<&Party>` exists only for `&Version` and `&Span`. The module doc one line above (own.rs:1) and the `Span` table row (span.rs:47) have it right.

Evidence:

         1	//! [`OwnVersion`]: the lazy projection view `&v / &p`.
        ...
        12	/// The projection of a [`Version`] by a [`Party`]: `v / &p`.

        69	/// Projection applies [`Version::project`] pointwise to the low and high ends
        70	/// of the span: for a given [`Span`] `s`, `s / &p` yields the span `(lo / &p)
        71	/// <= (hi / &p)`.

Resolution: Write `&v / &p` at own.rs:12 and `&s / &p` yielding `(&lo / &p) <= (&hi / &p)` at span.rs:70-71. Acceptance: every spelling of the projection operator in rustdoc takes a borrowed left operand.

### fresh-eyes-8: Query has no equality; the public docs do not say so and the private pointer to the rationale dangles
- Where: crates/before/src/causally/query.rs:20-30 (related: crates/before/src/causally/query.rs:211-215, crates/before/src/causally/query.rs:1-8, crates/before/src/causally.rs:1-169)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the sweep's check1.log line 51: `error[E0369]: binary operation '==' cannot be applied to type 'Query<'_, before::causally::Down>'`; this pass grepped `Eq`, `equal`, and `equalit` across `causally.rs` and `causally/*.rs`: the only hits are the `Coverage` derive at query.rs:51, the comment at query.rs:212, and unrelated uses of "equal"); executed: yes, by the sweep's compile probe, matched to its log
- Verification: reframed: the sweep found the rationale hidden in a private comment; this pass finds that the comment's "see the module docs" points at prose that does not exist in either query.rs's or causally.rs's module doc; history: no-rationale-found (db9dfa3ed introduced the comment; its message does not carry the argument either)
- Owner-gated: no

`Query` is `Clone` and `Debug` but not `PartialEq`. The deliberate absence is stated only in a private comment, and that comment defers to module docs that never take the subject up, so the reason survives nowhere in the tree.

Evidence:

        20	/// A causal filter on [`Version`]s and [`Span`]s within a restricted [`Query`]
        21	/// language.
        22	///
        23	/// Queries are composed from the atomic queries in this module, which may be
        24	/// negated with `!` and combined with `&`.

       211	// Debug renders the module's own expression vocabulary, the only structural
       212	// window into a query (there is deliberately no `Eq`; see the module docs), so
       213	// failures and logs read as an expression denoting the same predicate.

Resolution: Add one sentence to the `Query` type docs stating that queries carry no equality (two structurally different queries can denote one predicate, so `==` would not mean what a reader expects) and that `Debug` renders the normal form, and either state the rationale at query.rs:212 inline or point the comment at the sentence that now holds it. Acceptance: `grep -n 'see the module docs' crates/before/src/causally/query.rs` either returns nothing or the module doc it names contains the word `Eq`.

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

### fresh-eyes-10: Parameter named `k` in signatures, `n` in prose, across ticks and forks
- Where: crates/before/src/clock.rs:107-113 (related: crates/before/src/clock.rs:126, 159-166, 192; crates/before/src/party.rs:42-43, 184-186, 239, 274; crates/before/src/iter.rs:4; correct throughout at crates/before/src/version.rs:184-210)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read every listed range and both signatures); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`Clock::ticks`, `Clock::forks`, `Party::ticks`, and `Party::forks` take `k`, but their doc sentences, the `Party` operation table, and the `iter` module doc speak of `n`; `Clock::ticks` uses both in consecutive sentences. `Version::ticks` alone says `k` throughout.

Evidence:

       107	    /// Advances this [`Clock`] by `n` events for its own [`Party`], returning
       108	    /// the new [`Version`]: byte-identical to `n` sequential
       109	    /// [`tick`](Self::tick)s, computed in a bounded number of passes rather
       110	    /// than `n`.
       111	    ///
       112	    /// The count `k` is any unsigned number, since all can be converted into
       113	    /// [`Ticks`].
       ...
       126	    pub fn ticks(&mut self, k: impl Into<Ticks>) -> &Version {

Resolution: Use `k` (the signatures' letter) in every sentence and table row listed. Acceptance: `grep -n '`n`' crates/before/src/clock.rs crates/before/src/party.rs crates/before/src/iter.rs` returns no line in the docs of `ticks` or `forks`.

### fresh-eyes-11: Typos and wrong link targets in public rustdoc
- Where: crates/before/src/version.rs:603-604 (related: crates/before/src/causally/forms.rs:231-234 and 253, crates/before/src/clock.rs:341, crates/before/src/span.rs:413-415, crates/before/src/causally.rs:81, crates/before/src/iter.rs:9-10, crates/before/src/version/ticks.rs:18-24, crates/before/src/party.rs:45, crates/before/src/party.rs:184, crates/before/src/version/rank.rs:198)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read every listed range); executed: no
- Verification: confirmed at every site; the iter.rs:10 item is a stray backtick, not a broken link (the target `crate::Clock::forks` resolves); history: no-rationale-found
- Owner-gated: no

A batch of small prose defects a reader trips on. The one that misroutes: `Version::span_all` links the word `span` to `Version::meet`. `toward`'s doc names its arguments `s`/`e` in one sentence and `p`/`t` in the next, against a signature of `s`/`t`.

Evidence:

       603	    /// Prefer this to iteratively [`span`](Version::meet)ing [`Version`]s
       604	    /// one-at-a-time, as it is more efficient.

       231	/// Everything in the causal future of `s` (including `s` itself) but nothing in
       232	/// the causal future of `e` (including `e` itself).
       233	///
       234	/// Equivalent to `after(p) & until(t)`.
       ...
       253	pub fn toward<'a>(s: impl Into<Cow<'a, Version>>, t: impl Into<Cow<'a, Version>>) -> Query<'a, Up> {

       341	    /// Prefer this to iteratatively calling [`sync`](Clock::sync), as this is

       413	    /// A [`Span`] requires two causal comparisons: one to compare the two `lo`
       414	    /// endpoints and a second to compare the two `hi` endpoint, where seach
       415	    /// comparison costs:

        81	//! [`Query`] with arbitary negation is equivalent to the famously NP-complete

         9	//! See [`Party::forks`](crate::Party::forks) and
        10	//! [`Clock::forks](crate::Clock::forks).

        18	/// Event counts have no ceiling, so the count is unbounded rather than any
        19	/// fixed-width integer: every conversion *into* it is total ([`From`] on
        20	/// unsigned machine integers, and every conversion *out* is explicit
        21	/// about width: `TryFrom<&Ticks> for u64` answers the machine-range case

        45	/// | [`p.is_disjoint(&b)`](Party::is_disjoint)              | whether `p` and `q` share no region, hence may safely interact            |

       184	    /// Advances `version` by `n` events for this [`Party`]

       198	/// **A rank's representation never is larger than the version it measures, and

Resolution: version.rs:603 link to `Version::span`; forms.rs:232 `t` for `e`, forms.rs:234 `after(s) & until(t)`; clock.rs:341 "iteratively"; span.rs:414 "endpoints, where each"; causally.rs:81 "arbitrary"; iter.rs:10 close the backtick; ticks.rs:19-20 close the parenthesis after "unsigned machine integers"; party.rs:45 `q` for `b` (or `b` for `q`); party.rs:184 end the sentence; rank.rs:198 "is never larger". Acceptance: each listed line reads as stated and `cargo doc` reports no new broken intra-doc links (the owner's gate runs it).

### fresh-eyes-12: Party::decode's warning is written about Clock
- Where: crates/before/src/party.rs:605-610 (related: the same paragraph, correctly placed, at crates/before/src/clock.rs:768-772)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read both paragraphs); executed: no
- Verification: confirmed; history: no-rationale-found (e546b6d5e added the paragraph to both sites in one edit)
- Owner-gated: no

The `# Warning` on `Party::decode` speaks only of serializing and deserializing a `Clock`; the linearity hazard applies to the `Party` directly, and on the `Party` page the reader has to translate.

Evidence:

       605	    /// # Warning
       606	    ///
       607	    /// Serializing a [`Clock`](crate::Clock) circumvents its otherwise
       608	    /// compiler-enforced `!Clone` linearity. Deserializing one can violate
       609	    /// causality. Treat serialization/deserialization boundaries as *moves* of
       610	    /// the [`Clock`](crate::Clock).

Resolution: Restate in terms of `Party`: "Serializing a [`Party`] circumvents its otherwise compiler-enforced `!Clone` linearity. Deserializing one can violate causality. Treat serialization/deserialization boundaries as *moves* of the [`Party`]." Acceptance: the paragraph names the type it is attached to.

### fresh-eyes-13: The error module's summary line is a question, and Overlap's doc names one of its two producers
- Where: crates/before/src/error.rs:1-5 (related: crates/before/src/clock.rs:322 and 377, the two functions that construct `Overlap`)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read; `grep -rn Overlap crates/before/src | grep -v /tests` shows `Err(Overlap)` constructed only in `Clock::sync` (clock.rs:327) and `Clock::sync_all` (clock.rs:408, 413)); executed: no
- Verification: confirmed; history: no-rationale-found (a431eaf1d "Doc editing")
- Owner-gated: no

The `error` module's first sentence, which is what the module listing shows, is a rhetorical question; and `Overlap` is documented as arising "during Clock::sync" though `Clock::sync_all` returns it too, which is a hand-maintained caller list already one short.

Evidence:

         1	//! What could possibly go wrong?
         2	
         3	use std::io;
         4	
         5	/// Two parties were not disjoint during [`Clock::sync`](crate::Clock::sync).

       377	    pub fn sync_all<'a, I>(&mut self, iter: I) -> Result<&Version, Overlap>

Resolution: A summary that informs, such as "Error types: decode and parse failures, overlapping parties, crossed spans, and out-of-range tick counts."; for `Overlap`, state the condition without enumerating callers ("Two parties that were required to be disjoint overlap."). Acceptance: the module's first sentence is declarative and `Overlap`'s doc names no specific caller.

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

### fresh-eyes-15: Clock::join hands back a Clock in Err, which does not compose with `?`
- Where: crates/before/src/clock.rs:196-218 (related: crates/before/src/clock.rs:229-237 (`join_all`, `Err(Vec<Clock>)`), crates/before/src/clock.rs:298-322 and 377 (`sync`/`sync_all`, `Err(Overlap)`), crates/before/src/lib.rs:65-68)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (the sweep's check1.log line 170: `error[E0277]: '?' couldn't convert the error: 'before::Clock: std::error::Error' is not satisfied`; this pass read the four signatures and grepped the crate and its tests for a `map_err(|_| Overlap)` idiom: none is shown anywhere); executed: yes, by the sweep's compile probe, matched to its log
- Verification: confirmed; history: no-rationale-found beyond the docs' own "handed back in the error"
- Owner-gated: yes for an `Overlap`-returning variant; no for documenting the idiom

`join` returns `Result<&Version, Clock>` and `join_all` returns `Result<&Version, Vec<Clock>>`; neither error type is an `Error`, so `?` into `Box<dyn Error>` fails, while `sync`/`sync_all` return `Overlap`. The drop-nothing hand-back is a sound design; the friction is that the two halves of the join family differ and no doc shows the bridge.

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

## Positives

- The operation tables on `Clock` (lib.rs:27-34), `Version` (version.rs:50-56), `Party` (party.rs:38-46), and `Span` (span.rs:38-48), plus the "Version vector or vector clock?" section (lib.rs:79-216), were enough for the sweep to write the whole scratch program without opening private source; every operator it reached for meant what the table said, and the quickstart compiled as written. This pass read the tables and found them accurate against the impls it checked (`Div` on `&Version`, `PartialOrd` on `Version`, `|`/`|=` on `Clock`).
- The `Decode` variant docs (error.rs:69-85) draw the `Truncated`/`TrailingBits` boundary precisely, including the flush-against-a-byte case, and the sweep's hostile probes (empty input, a party alone, a value plus trailing bytes) each returned the documented variant with no panic.
- `Span::decode`, `Rank::decode`, and `Ranked::decode` carry model `# Errors` sections (span/wire.rs:79-89, rank.rs:435-445, ranked.rs:241-247): every variant named with the input class that produces it. Finding 2 asks only that the other three decoders match them.
- `Ranked`'s composite key did what the docs promise on real data: BTreeMap order over `ranked().encode()` bytes matched `Ord` on `Ranked`, causes sorted before effects across a 13-version fleet, and `Ranked::decode` recovered the version from the key alone (the sweep's run; not re-run here).
- `Query`'s `Debug` renders the expression vocabulary, so a failed assertion reads as source; the valuation law `rank(a|b) + rank(a&b) == rank(a) + rank(b)` and `lag(a,b) + lag(b,a) == distance(a,b)` held on the fleet (the sweep's run).
- `suanpan`: the crate page's first example was enough to write a correct running total across a 2^128 carry boundary on the first try; the `&mut self` on sign reads and the one-sided `is_literally_zero` are explained where a user meets them (the sweep's report; this pass did not re-read `suanpan`).
- Space claims read true at small scale: 13 parties encode in 1-2 bytes each, and the root's version after 23 events across the fleet is 9 bytes (the sweep's run).

## Open questions for Finch

1. Is a human-readable serde form wanted? Today every type serializes as `serialize_bytes` of its canonical encoding, so serde_json carries a number array. Branching on `Serializer::is_human_readable()` to emit the paper notation (the `Display`/`FromStr` pair already exists for `Party`, `Version`, `Clock`, and `Ticks`) is how uuid and similar crates do it, but `Rank`, `Ranked`, and `Span` would need text forms first, and it changes the serialized bytes for human-readable formats. Recommendation: document the current representation now (finding 3) and decide the rest separately.
2. The `door` coinage: two public sites are the subject of finding 4, but the word appears in roughly two hundred private-prose sites across `laws.rs`, `meter*`, `codec/*`, `testing/*`, and `surface.rs` as the maintainer term for a public entry point. Is it an accepted crate-internal term of art? If so it deserves one definition where a maintainer first meets it (the `implementation` essay or `party.rs`'s module doc), and the public sites should still use plain words; if not, it is a crate-wide vocabulary dissolution, larger than this sweep.
3. The polarity conflict (`since(a) & until(b)`) produces only std's generic "trait `BitAnd` not implemented" error listing the available impls; because `BitAnd` is std's, `#[diagnostic::on_unimplemented]` cannot annotate it. Is the type-level mention at query.rs:26-30 considered sufficient, or is a worked "this does not compile, and why" example in the `causally` module doc wanted?

## Dropped

- fresh-eyes [1], sub-claim "and Span::decode" in the resolution: `Span::decode` already carries a complete `# Errors` section at span/wire.rs:79-89; the finding (fresh-eyes-2) is scoped to `Version`, `Party`, and `Clock`.
- fresh-eyes [9], sub-claim "fix the broken intra-doc link at iter.rs:10": the link target `crate::Clock::forks` resolves; the defect is the unclosed backtick, kept under fresh-eyes-11.
- Sweep open question "how does rustdoc render `Clock::forks`'s return type": settled by reading the rustdoc build at target/doc (renders `Forks<'_>`, linked to a page titled "Clock in before::iter"); folded into fresh-eyes-9.
