# Sweep rumors-dependence: The guarantees rumors relies on from before, checked against before's contract and tests

Repository `/Users/oxide/src/rumors` at `9e5784fb4dce977cfbdfd1619886d1482b5ce764`, working tree clean. This is the verification-and-finalization pass over the sweep's ledger (`scratchpad/before/sweeps/rumors-dependence/ledger.md`) and its five findings. Every claim below was re-checked against the cited lines; where the sweep's framing did not survive, the entry says how it was reframed.

## Method and coverage

Mechanically:

- Read the sweep's ledger in full, then opened every site the five findings cite, in before (`version.rs`, `party.rs`, `clock.rs`, `span.rs`, `laws.rs`, `surface.rs`, `meter.rs`, `meter/tier2/tests.rs`, `party/tests.rs`, `version/tests.rs`, `clock/tests.rs`, `span/tests.rs`, `codec/bits.rs`, `serde_impls.rs`, `auto_traits.rs`, `lib.rs`) and in rumors (`tree.rs`, `tree/typed/path.rs`, `tree/typed/untyped.rs`, `tree/traverse/act.rs`, `peer/gossip.rs`, `bookmark.rs`, `peer.rs`, `rumors/causal.rs`, `tree/mirror/party.rs`, `tree/mirror/streaming/message.rs`, `tree/mirror/streaming/window.rs`, `tree/tests.rs`, `tests/party_conservation.rs`, `tests/bookmark_causality.rs`, `tree/traverse/unknown/tests.rs`), in the cited ranges with line numbers.
- Grepped the whole tree for every identifier a finding turns on (`as_bytes_matches_encode`, `subadditiv`, `is_coincident`, `ptr_eq`, `span_traffic`, `require_marker_padding`, `padding_is_canonical`, `pub mod implementation`, `dangerously_alias`).
- Git: `git log -S` on `self.as_bytes().to_vec()`, `as_bytes_matches_encode`, `padding_is_canonical`, `join_encoding_is_subadditive`, `tick_only_inflates_the_region`, `dropped without further use`, and `pub mod implementation`; `git show` on d0e54d955, 24ff18b93, d800957e8, and 22cdfbe1a; `git blame` on `party.rs:514-516`.
- Rationale sources: `crates/before/AGENTS.md`; `.cargo/mutants.toml`'s header; `.agent-notes/2026-07-22-before-adversarial-resource-amplification` (the subadditivity lemma of record, the Gate A ruling, the laws module's linearity note), `2026-07-25-before-tick-cost-spec`, `2026-07-26-before-formal-tick`.
- No cargo, just, build, or test command was run. The two permitted test invocations were not needed: every disputed claim was settled by reading definitions and history (finding 3 turns on `encode` being defined as `as_bytes().to_vec()`, which is a fact of the source).

Not seen: the rendered fuelscape HTML (the JSON's `contract` field was read instead: `version_tick` declares `O(|self| + |party|)`, `span_dominance` declares `O(|self| + |version|)`); whether rustdoc on the examples flags the broken intra-doc link in finding 6 (no build). `.claude/worktrees/agent-a0e01f4cff55d1ec9/` inside the repository duplicates the same files and belongs to another running agent; it was ignored and not touched.

Verdict on the sweep: its ledger holds. rumors stays inside before's documented contract at every use the ledger enumerates, and its own corpus would catch before regressions on every property it depends on. All five findings survive; two are reframed (2 and 5), one is extended (3), and one new documentation finding fell out of the verification (6).

## Findings

### rumors-dependence-1: Tick's event contract (strict advance, region-locality) is pinned by laws but stated in no public rustdoc of `tick`
- Where: crates/before/src/version.rs:161-182 (related: crates/before/src/party.rs:161-182, crates/before/src/clock.rs:88-105, crates/before/src/laws.rs:15-19, crates/before/src/laws.rs:2453-2478, crates/before/src/lib.rs:218-256, src/tree.rs:83-89, src/tree/typed/path.rs:21-29, src/tree/traverse/act.rs:30-38, src/tree/traverse/act.rs:161-176, src/peer/gossip.rs:645-665, src/bookmark.rs:444-455)
- Class / severity / confidence: dependence / medium / high
- Provenance: assessed (read); executed: no
- Verification: reframed: strictness is already public by example (`version.rs:178`, `party.rs:178`, `clock.rs:100`, the tutorial at `lib.rs:195-198`); what the public contract lacks is region-locality and its consequence, distinct stamps across disjoint parties; history: no-rationale-found (the laws landed with 86dd53a71 and b3f09baa0; nothing records why the contract stayed out of `tick`'s rustdoc)
- Owner-gated: yes: public rustdoc on the stable API, and a new public law name under the `laws` feature

rumors derives leaf identity from version bytes and panics on version reuse, resting on two properties of `tick`: the result strictly dominates the input, and it differs from the input only inside the ticking party's region, so ticks by disjoint parties from any base never coincide. before pins both as laws (`tick_strictly_advances`, `tick_only_inflates_the_region`, `tick_advances_within_the_region`) driven over both law populations and the fuzz target, and states them in the laws module's header, but `Version::tick`, `Party::tick`, and `Clock::tick` say only that the version advances by one event; region-locality appears in no rustdoc outside the feature-gated laws module, and the safety-rules section connects identity disjointness to nothing about events.

Evidence:

       161	    /// Advances this version by one event for `party`.

    laws.rs:
        18	//! disjoint join with `fork` as its splitting inverse, events inflate strictly
        19	//! and only within the owned region, and `rank` is a strictly monotone

      2460	    /// `tick` inflates only within the party's region (§4: `e' = e + f·i`, zero
      2461	    /// outside `i`): projected onto the region's complement, the ticked version
      2462	    /// is unchanged. Vacuous only for the seed party, which has no complement.

    src/tree.rs:
        85	/// [`Path::for_leaf`](typed::Path::for_leaf)). Versions are unique per
        86	/// send — locally by tick, globally by party disjointness — so two
        87	/// content-identical messages sent at distinct moments occupy distinct
        88	/// leaves, and two leaves collide only when a version has been reused,
        89	/// which conforming peers cannot do.

    src/tree/traverse/act.rs:
        33	/// version or payload: version reuse. No input reaches that state —
        34	/// every production insert carries a freshly created version (a fresh
        35	/// tick strictly dominates the ceiling bounding every live leaf, and
        36	/// party linearity keeps regions disjoint), and no wire-derived leaf

Resolution: state the event contract on `Version::tick` and mirror it on `Party::tick` and `Clock::tick`: the result strictly dominates the input; projected onto any region disjoint from `party`, the result equals the input; hence ticks by disjoint parties from one base always differ. Optionally add the consequence as a `VERSION_PARTY_PAIR` law (for example `ticks_by_disjoint_parties_differ`: `!p.is_disjoint(q) || { let mut a = v.clone(); a.tick(p); let mut b = v.clone(); b.tick(q); a != b && (&b / p) == (v / p) }`); the one-world population inhabits the antecedent (`laws.rs:2782-2783`), and the roster and fuzz driver pick a new group member up by construction (`laws.rs:94-97`). Acceptance: the rustdoc of the three `tick` entries states both clauses; a reader of before's public docs can derive stamp uniqueness without the paper or the laws module.

### rumors-dependence-2: Join/meet encoding subadditivity is derived and pinned inside before's test tree and priced against by rumors, but stated in no public contract
- Where: crates/before/src/version.rs:438-461 (related: crates/before/src/version.rs:498-521, crates/before/src/version.rs:48-56, crates/before/src/lib.rs:310-340, crates/before/src/version/tests.rs:109-161, crates/before/src/version/tests.rs:481-528, crates/before/src/meter/tier2/tests.rs:326-346, crates/before/src/meter/tier2/tests.rs:417-453, crates/before/src/meter/tier2/tests.rs:496-560, src/tree/mirror/streaming/message.rs:79-88, src/tree/mirror/streaming/window.rs:320-337, src/tree/mirror/streaming/window.rs:349-358, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:359-373, :455-457)
- Class / severity / confidence: dependence / low / high
- Provenance: assessed (read); executed: no
- Verification: reframed: the sweep undersold before's side. The bound has a written derivation (the rustdoc of `JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS`, a term-by-term argument with the tight constant of 2 bits and its equality witness) and two instruments (the churned and arbitrary proptests in `version/tests.rs`; the tier2 grid across four emitters, including the skyline kernel with its short-circuits stripped). The residual is only where the contract is stated; history: deliberate-and-holds for the pins (the agent note records the lemma of record and the 2026-07-23 ruling), no-rationale-found for keeping the statement out of the public rustdoc
- Owner-gated: yes: public rustdoc

rumors' window pricing charges a cross-side ceiling or floor the sum of the two exchanged bounds, citing before's "pinned join- and meet-subadditivity lemmas". before proves and pins the bound, derivation included, inside `meter/tier2/tests.rs` and its proptests, but `Version::join`, `Version::meet`, the operator table, and the Space Efficiency section state no size relation between a join or meet and its operands. A maintainer reading the join contract cannot learn that the property is load-bearing downstream, and rumors' comments can cite only test names.

Evidence:

    crates/before/src/version.rs:
       438	    /// The join (least upper bound) of this [`Version`] and `other`: their
       439	    /// combined causal history.
       440	    ///
       441	    /// Identical to the operator form `self | other`.

    crates/before/src/meter/tier2/tests.rs:
       331	/// For canonical `a`, `b` and `c` either their join (pointwise max) or meet
       332	/// (pointwise min), the Tier 2 sizes satisfy
       333	/// `size(c) <= size(a) + size(b) - 2`, term by term: the canonical output
       ...
       346	const JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS: u64 = 2;

    crates/before/src/version/tests.rs:
       115	    /// never invent structure beyond both inputs together. Callers that track
       116	    /// version-size maxima rely on this to charge a join of two bounded
       117	    /// versions the sum of their bounds. Probed here over churned

    src/tree/mirror/streaming/window.rs:
       326	    /// and a bound a session assembles across the two joins a ceiling —
       327	    /// or meets a floor — drawn from each side, encoding within the
       328	    /// pair's sum (`before`'s pinned join- and meet-subadditivity
       329	    /// lemmas); a node holds two bounds, hence the double. One priced

Resolution: add one sentence to `Version::join` and `Version::meet` (or one paragraph to the Space Efficiency section, linked from both): the canonical encoding of a join or meet is never longer than the sum of its operands' encodings. Optionally promote `join_encoding_is_subadditive` and `meet_encoding_is_subadditive` to `VERSION_PAIR` laws so the fuzz law target also drives them over decoded values. Acceptance: the statement appears in public rustdoc; rumors' `message.rs` and `window.rs` comments can cite the documented contract rather than "pinned lemmas".

### rumors-dependence-3: The `as_bytes == encode` laws, tests, and the roster pin they anchor are tautological: `encode` is `as_bytes().to_vec()`
- Where: crates/before/src/laws.rs:419-422 (related: crates/before/src/laws.rs:2155-2158, crates/before/src/version.rs:1027-1029, crates/before/src/party.rs:554-556, crates/before/src/party/tests.rs:982-1003, crates/before/src/version/tests.rs:625-651, crates/before/src/clock/tests.rs:296-328, crates/before/src/surface.rs:196-205, crates/before/src/testing/surface_coverage/tests.rs:24-28, crates/before/src/laws.rs:404-410, crates/before/src/laws.rs:2141-2146, crates/before/src/version.rs:1174-1180, crates/before/src/party.rs:698-704, tests/bookmark_causality.rs:1075-1094)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the definitions of `encode` on both types and the git history of the seam; the two sides of each assertion are one expression by definition, so no run is needed); executed: no
- Verification: confirmed and extended to the roster: `surface.rs:203` cites `as_bytes_matches_encode` as one of three production-side pins the codec rows' exclusions rest on, and `clock/tests.rs:300-305` describes a divergence between `as_bytes` and `encode` that the current definition makes impossible; history: deliberate-but-expired: at d0e54d955 (which added `as_bytes` and these tests) `encode` packed independently through `codec::pack_to_writer`, so the comparison discriminated; 24ff18b93 redefined `encode` as `self.as_bytes().to_vec()`, and nothing re-denominated the tests
- Owner-gated: no for re-denominating test and law bodies under their existing names; renaming or removing a law changes the names public under the `laws` feature, which the owner rules on

The laws `version_as_bytes_matches_encode` and `party_as_bytes_matches_encode`, the tests `as_bytes_matches_encode` and `as_bytes_matches_encode_after_fork` (party) and `as_bytes_matches_encode` and `as_bytes_matches_encode_after_ticks` (version), and two assertions inside `encoding_views_agree_over_impl_history` all compare `x.as_bytes()` with `x.encode()`, and `encode` on both types is `self.as_bytes().to_vec()`, so each comparison is a slice against its own copy and cannot fail. The stated invariant (stored padding is canonical after every construction and mutation path) is discharged today by the `debug_assert!` inside `as_bytes` (which the campaign configuration of record does run, per `.cargo/mutants.toml`'s header) and, independently, by the strict-decode legs: `version_codec_roundtrip`, `party_codec_roundtrip`, and the `decode(as_bytes)` assertions in `encoding_views_agree_over_impl_history`, all of which reject a non-canonical tail through `require_marker_padding` (`version.rs:1118`, `party.rs:631`). Principle 3 (circular justification) and Principle 6 (the cheapest passing artifact): an assertion whose two sides are one expression is decoration, and a roster pin citing it certifies nothing. The property is what rumors hashes into leaf paths and ships on the wire; rumors' own regression `retire_into_rebooted_absorber_absorbs_cleanly` guards the seam from outside through strict decode, not through these.

Evidence:

    crates/before/src/laws.rs:
       419	    /// The borrowed byte view is the encoding: `as_bytes == encode`.
       420	    fn version_as_bytes_matches_encode {
       421	        a.as_bytes() == &a.encode()[..]
       422	    }

    crates/before/src/version.rs:
      1027	    pub fn encode(&self) -> Vec<u8> {
      1028	        self.as_bytes().to_vec()
      1029	    }

    crates/before/src/party.rs:
       554	    pub fn encode(&self) -> Vec<u8> {
       555	        self.as_bytes().to_vec()
       556	    }

    crates/before/src/surface.rs:
       201	const CODEC_PINS: &[&str] = &[
       202	    "decode_encode_arbitrary",
       203	    "as_bytes_matches_encode",
       204	    "decode_never_panics",
       205	];

    crates/before/src/clock/tests.rs:
       301	    /// only ever compare `encode`d bytes; the `as_bytes_matches_encode` tests
       302	    /// only build via the oracle. Neither combination drives the impl's own
       303	    /// `fork`/`join`/`sync` *and* reads `as_bytes` — the exact seam where a
       304	    /// normalizing `join` once left stale bits in the stored buffer, so that
       305	    /// `as_bytes` (the borsh wire form) diverged from the canonical `encode`.

    version.rs at d0e54d955 (history, for provenance):
       131	    pub fn encode(&self) -> Vec<u8> {
       132	        let mut bytes = Vec::new();
       133	        self.encode_to(&mut bytes)
       134	            .expect("writing to a Vec is infallible");
       135	        bytes
       136	    }
       146	    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
       147	        codec::pack_to_writer(&self.0, writer)

Resolution: re-denominate each body against the independent judge while keeping the names, so the roster and the duplicate-name table stay untouched: in `laws.rs`, `Version::decode(a.as_bytes()).is_ok_and(|d| d.as_bytes() == a.as_bytes())` (and the party twin; `laws.rs` has no field access); in the party and version test modules, `codec::padding_is_canonical(&v.0)` or the same decode form, so `after_fork` and `after_ticks` assert what their docs say without relying on the debug assertion. Re-state `clock/tests.rs:300-305` in the present tense (what it protects: stored padding after impl-driven `fork`/`join`/`sync`, judged by strict decode). Alternatively delete the two laws and drop the pin from `CODEC_PINS`, since the roundtrip laws already cover it. Acceptance: with the `debug_assert!` in `as_bytes` disabled and an unsealed tail introduced after `join` (the historical seam), the re-denominated tests fail; today they cannot.

Construction: temporarily replace the body of `as_bytes` with a plain `self.0.as_raw_slice()` and make `Party::join` skip `seal_padding` on its output (or hand-build a `Party` whose stored buffer carries stale bits after the marker). The current `as_bytes_matches_encode*` tests and both laws stay green on that mutant; `party_codec_roundtrip` and the re-denominated forms go red.

### rumors-dependence-4: `dangerously_alias`'s "dropped without further use" forbids the read-only uses before's own examples and laws make
- Where: crates/before/src/party.rs:514-520 (related: crates/before/src/clock.rs:870-872, crates/before/src/party.rs:419, crates/before/src/party.rs:452-453, crates/before/src/laws.rs:40-50, crates/before/src/laws.rs:2761-2762, src/bookmark.rs:285-298, src/bookmark.rs:418-425, src/bookmark.rs:466-478, src/peer.rs:718-734, tests/party_conservation.rs:12-14)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Verification: confirmed and extended: before's laws module documents a scoped-use reading (aliases "live and die inside the predicate") and forks an alias inside `projection_monotone_in_region`; rumors' uses (compare, encode, persist, decode-and-join after a crash) are a subset of that reading; history: no-rationale-found (the sentence dates to b3f09baa0's docs pass; nothing records why the letter overshoots the Warning's own hazard)
- Owner-gated: yes: public rustdoc on the stable API

The Warning names the hazard precisely (two live handles that go on to `tick` or `join`) and then states a rule stricter than the hazard: the second copy "must be dropped without further use". before's own examples pass an alias to `covers` and stringify one for `without`; the laws module aliases inside every predicate and forks the alias; rumors' bookmark compares, encodes, and persists aliases. Principle 4: the goal (no second live actor on one region) stands beside a mechanism sentence that overshoots it, so the one production consumer that uses the door sits outside the letter of the contract it cites.

Evidence:

    crates/before/src/party.rs:
       514	    /// The caller must ensure that at most one of the two copies is ever
       515	    /// treated as live; the other must be dropped without further use. The same
       516	    /// rule applies to any [`Clock`](crate::Clock) built from such a party.
       ...
       419	    /// assert!(p.covers(&p.dangerously_alias())); // a region covers itself

    crates/before/src/laws.rs:
        46	//! [`Party::dangerously_alias`] / [`Clock::dangerously_alias`]: the aliases
        47	//! live and die inside the predicate, which owns no clock universe, so the
        48	//! linearity hazard the method documents (two live holders of one region) never
        49	//! escapes a call. The laws quantify over a value's geometry, which aliasing
       ...
      2761	        let mut keeper = p.dangerously_alias();
      2762	        let child = keeper.fork();

    src/bookmark.rs:
       466	        // Store an alias of our party at its current version.
       467	        clocks.push(Clock::from_parts(
       468	            party.dangerously_alias(),
       469	            version.clone(),
       470	        ));

Resolution: reword the Warning on `Party::dangerously_alias` and `Clock::dangerously_alias` to name the mechanism: while one copy is live, the other must not act as a party (`tick`, `fork`, `join`, `sync`, or any operation that makes it a second actor); queries (`covers`, `is_disjoint`, `without`, comparison), encoding, and persistence of the copy are not acting. Keep the boundary hand-off sentence as the motivating case. Acceptance: before's examples, the laws module's predicates, and rumors' bookmark all fall inside the letter.

### rumors-dependence-5: rumors cites `Span::dominance`'s coincident fast path as a cost contract; before documents it only in code comments and a private test, and no meter on either side counts it
- Where: crates/before/src/span.rs:151-182 (related: crates/before/src/span.rs:303-332, crates/before/src/span.rs:546-555, crates/before/src/span/tests.rs:181-201, crates/before/src/meter.rs:3632-3646, src/tree/typed/untyped.rs:535-557, src/tree/tests.rs:1398-1447)
- Class / severity / confidence: dependence / nit / high
- Provenance: assessed (read); executed: no
- Verification: reframed: the sweep said rumors' `span_door_traffic` reads the rung meter to pin this traffic; `meter::span_traffic` counts pair-hull constructions (`Version::span`, `span_all` leaf combines, span-union point combines), not `dominance`'s `ptr_eq` rung, so the cost property rumors documents is unmetered on both sides; history: no-rationale-found
- Owner-gated: yes: public rustdoc (the alternative, a rumors-side rewording, belongs to the rumors review)

rumors' bounds-memo docs state that a leaf's `Span::at(version)` stores one version twice and that clone identity certifies the coincidence in `O(1)`, so classifying a leaf pays one decode of each stream, never two. `Span::at`'s public rustdoc gives `O(1)` for construction only; the coincident routing lives in code comments in `at`, `place`, and `dominance`, and in the private `is_coincident` and its private test. The documented `O(|self| + |version|)` bound on `dominance` covers the fast path, so nothing is wrong today; the finding is that a consumer presents as before's contract a property it can learn only from before's source.

Evidence:

    crates/before/src/span.rs:
       156	    /// # Complexity
       157	    ///
       158	    /// `O(1)`.
       ...
       309	        // Clone identity certifies the coincidence in `O(1)` so one
       310	        // single-bound placement (each stream decoded once) answers where the
       311	        // fused walk would read the shared buffer twice.
       ...
       553	    fn is_coincident(&self) -> bool {

    src/tree/typed/untyped.rs:
       551	    /// leaf's span stores its one version twice, and clone identity
       552	    /// certifies the coincidence in `O(1)`), so routing wholly through
       553	    /// [`span`](Self::span) pays a leaf one decode of each stream,
       554	    /// never two.

    crates/before/src/meter.rs:
      3640	/// kernel regime the consumer actually pays. Counts every pair-hull
      3641	/// construction: every [`Version::span`](crate::Version::span), every leaf
      3642	/// combine of `span_all`, and every point-combine of the span union doors
      3643	/// (`Span | Span` and [`Span::union_all`](crate::Span::union_all) on coincident
      3644	/// operands), which derive their hull through the same kernel. Process-global,

Resolution: either state under `Span::at`'s `# Complexity` that the coincident span shares one stored buffer, so `place`, `dominance`, `precedence`, and `contains` against it cost a single pairwise comparison (before-side, owner-gated), or have the rumors review soften `untyped.rs:548-554` to describe the routing without presenting it as before's contract. Acceptance: rumors' comment cites a public statement or makes none.

### rumors-dependence-6: Three sites cite the retired `implementation` module
- Where: crates/before/AGENTS.md:5-6 (related: crates/before/examples/code_study.rs:5-7, crates/before/src/version/skyline/build/tests.rs:394-395)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git show 22cdfbe1a --stat` lists `crates/before/src/implementation.rs | 270 -------------`, its message reads "The design-essay implementation module is retired.", and `lib.rs` declares no such module today); executed: no
- Verification: found during this pass, outside the dependence lens, recorded here so it has a disposition; history: deliberate-but-expired (the module was retired in 22cdfbe1a; the citations were not excised)
- Owner-gated: no

`crates/before/AGENTS.md` sends the reader to "the public `implementation` module for the design essay"; `examples/code_study.rs` carries an intra-doc link to `before::implementation`; a test doc in `version/skyline/build/tests.rs` cites "the `implementation` essay". No module of that name exists (the `pub mod` declarations in `lib.rs` are `causally`, `error`, `iter`, `shape`, `oracle`, `meter`, `surface`, `laws`). This breaches before's hard rule that nothing in the codebase refers to code that no longer exists, at the guidepost the AGENTS.md itself is.

Evidence:

    crates/before/AGENTS.md:
         5	crate docs for the model (`Party`/`Version`/`Clock`, the Law of
         6	Disjointness) and the public `implementation` module for the design essay,

    crates/before/examples/code_study.rs:
         5	//! This is the instrument behind the crate docs' integer-code figures (the
         6	//! [`implementation`](before::implementation) essay's "Small values over
         7	//! large" trade): the constants below are the as-run parameters of the

    crates/before/src/version/skyline/build/tests.rs:
       394	/// independently; a different integer code (the `implementation` essay
       395	/// contemplates ζ₂, whose zero costs two bits) would silently turn every

Resolution: excise or re-point each citation to where the material lives now, in the present tense (the crate docs for the model; `version/skyline.rs` for the stored coding and its integer-code trade, if the "Small values over large" argument survives there; otherwise drop the reference). Acceptance: `grep -rn implementation crates/before --include='*.rs' --include='*.md'` finds no reference to a module or essay of that name.

## Positives

Everything here was verified by reading the cited lines in this pass.

- Every ingress of a before value into rumors routes through the strict decoders with typed errors and no `unwrap`: `src/tree/mirror/party.rs:131-138` maps `Decode::Io` separately and wraps every other defect as `HandOffMalformed`; serde deserialization of `Version` is `Version::decode` (`crates/before/src/serde_impls.rs:39-44`).
- rumors' mechanism for before's Causal Singularity rule is explicit: the `Network` check precedes any reconciliation (`src/peer/gossip.rs:1155-1163`), and the bookmark record is keyed by `Network` (`src/bookmark.rs:439`).
- The bookmark confronts the documented bytes hole with its obligations stated at the site: one critical section carries the persist-before-transmit and fork-at-snapshot obligations (`src/peer/gossip.rs:645-665`); reclaim gates on `clock.own_version() <= *version` with the reasoning written beside it (`src/bookmark.rs:444-455`); a restart re-bootstraps rather than resurrecting, and the reverted-bookmark hazard is named (`src/bookmark.rs:55-70`).
- `CausalMessages`' key `(Rank, Vec<u8>)` (`src/rumors/causal.rs:54-63`) is exactly the tuple before's law `ranked_orders_by_rank_then_bytes` (`crates/before/src/laws.rs:615-628`) pins equal to `Ranked`'s total order, so rumors' claim of sharing that order is pinned on before's side.
- `crates/before/src/auto_traits.rs` pins `Send + Sync + Unpin` for every public type at compile time; rumors' `Arc`-shared trees and watch channels depend on this without saying so, and the pin makes that safe.
- `encoding_views_agree_over_impl_history` (`crates/before/src/clock/tests.rs:296-341`) drives the implementation's own `fork`/`join`/`sync` and runs strict `decode` on `as_bytes`: the live before-side pin for the seam rumors' `retire_into_rebooted_absorber_absorbs_cleanly` guards from outside.
- The subadditivity lemma comes with a tight constant and its extremal witness: `JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS` (`crates/before/src/meter/tier2/tests.rs:328-346`) and `empty_pair_is_the_subadditivity_equality_case` (`:496-516`), checked across four emitters including the kernel without its short-circuits.
- The meter reach is gated on both sides: `pub mod meter` and `pub mod surface` sit under `#[cfg(any(test, feature = "meter"))]` (`crates/before/src/lib.rs:438-442`); rumors' `span_door_traffic` module is under `#[cfg(feature = "meter")]` (`src/tree/tests.rs:1398`).
- rumors states its alias discipline at the definition (`src/peer.rs:718-725`) and restates it at the head of the conservation suite (`tests/party_conservation.rs:12-14`).

## Open questions for Finch

1. Does the event contract (strict advance, region-locality, hence distinct stamps across disjoint parties) belong in `tick`'s public rustdoc, given that rumors' identity model and bookmark reclaim rest on it? The laws exist; the question is only where the contract is stated (finding 1).
2. `just citecheck` runs `tools/citecheck --root crates/before` over `-p before`'s test list (`justfile:291-292`), so rumors' maintainer prose citing before law names (`span_place_matches_relations`, `span_dominance_coarsens_place` at `src/tree/traverse/unknown/tests.rs:29-31`) is never checked. Should citecheck's haystack include before's law names for cross-crate citations, or should rumors avoid citing before test and law names?
3. Where should the subadditivity derivation of record live: the rustdoc of a test-module constant (today), or the public contract of `join`/`meet` (finding 2)?
4. Carried from the sweep's report for the rumors review, not verified in this pass: `PartyGuard::drop` (`src/peer/gossip.rs:1384-1403`) recovering a speculative fork with only a `debug_assert!(false)` on the impossible failure; `parse_record` (`src/remote/codec/frame.rs:82-84`, `:364-365`) accepting the version atom's CBOR byte-string head without spelling judgment; and `src/tree.rs:83-89`'s collision premise also assuming SHA3-256 path collision freedom, which is rumors' own hashing assumption.

## Dropped

- No finding dropped.
- From finding 5, the sub-claim that rumors' `span_door_traffic` test pins the dominance coincident rung: `meter::span_traffic` counts pair-hull constructions, not `Span::dominance`'s `ptr_eq` path (`crates/before/src/meter.rs:3640-3644`).
- From finding 2, the emphasis on "not laws, so the fuzz law target never exercises them" as the operative gap: true, but secondary once the tier2 grid and the churned and arbitrary proptests are counted as the instruments of record.
- From finding 3, "passes vacuously in any build without debug assertions" as the operative cost: the campaign configuration of record runs with debug assertions (`.cargo/mutants.toml` header), so the cost is the tautology itself and the roster pin that cites it, not release-mode vacuity.
