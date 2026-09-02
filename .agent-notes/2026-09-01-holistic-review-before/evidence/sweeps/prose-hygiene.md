# Sweep prose-hygiene: Ghost references, temporal language, coinages, dialect tells

## Method and coverage

This is the verification pass over the prose-hygiene sweep at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean). Every one of
the sweep's fifteen findings was disputed against the tree: each cited site
was opened with line numbers and its excerpt compared verbatim; each count was
recomputed with an independent grep over the same file set (crates/before
src, tests, benches, examples, fuzz, fuzzfit, wasm32-pins, surfacecheck, docs,
scripts, README, AGENTS.md, Cargo.toml, build.rs; crates/before-fuelscape/src;
crates/suanpan; crates/surface-scan; the root justfile, .cargo/mutants.toml,
.github/workflows); and every phrase whose age matters was traced with `git
log -S`. Recorded rationales were sought in `.agent-notes/` (the
`before-`-prefixed notes and the design-directory migration note),
`crates/before/AGENTS.md`, `.cargo/mutants.toml`'s header, and the commit
messages of the introducing commits.

Recomputed counts: `mint` 82 lines (32 of them in non-test rustdoc);
`honest`/`honestly`/`honesty` 234 and `genuine`/`genuinely` 65; `door` 279
lines in 50 files, `knob` 107 in 19, `seam` 276 in 71, `luck-proof` 1;
`improvement tripwire` 20 (all in tests/meter.rs) against 171 `tripwire` in
total; em-dashes 4140 lines, of which 3255 are rustdoc (`///` or `//!`), 551
plain `//` comments, 104 Rust code lines (string literals), and 230 non-Rust
lines (justfile 57, suanpan README 47 and before README 11, both derived by
cargo-rdme, docs/fuelscape-header.html 24, docs/fuelscape.js 22,
.cargo/mutants.toml 20, ci.yml 9, scripts 7, Cargo.toml comments 13);
calendar dates outside `decided:` literals: three (REGISTRY_RATIFIED,
PINNED_RUSTC, the justfile nightly pin), plus ten `decided: "20..."`
literals and nine `decided: REGISTRY_RATIFIED` uses; TODO/FIXME/XXX/HACK: 0;
PROG-5/COV-7 outside the fuzz workspace: 0, `.agent-notes/` included.

The pass also swept for the word "dated" as an instruction or description and
found two ghost descriptions of the convention the dated-notes excision
commit (d2a9d04e, 2026-07-31) retired; these become one new finding
(prose-hygiene-6).

What this pass could not see: no test was run (none of the surviving
findings is a correctness claim); the comparison kernel's iterativity is
taken from `Version::partial_cmp` routing to `skyline::sweep::causal_cmp`
(version.rs:1730-1741), the skyline module doc, and the pinned
`segments = 0` rows rather than from a reading of `sweep.rs`; the sweep's
sampled positives about "silently" and "the walk" were spot-checked at three
sites, not re-sampled.

## Findings

### prose-hygiene-1: tests/meter.rs narrates the pre-skyline recursive, quadratic implementation as the present
- Where: crates/before/tests/meter.rs:7-11 (related: crates/before/tests/meter.rs:428-429, 440-441, 465-466, 983-985; crates/before/tests/meter.rs:263, 269; crates/before/src/lib.rs:350-358)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read the module doc, `Envelope`, `envelope()`, the envelope table 245-301, the scenario docs; cross-read lib.rs:342-358 and version.rs:1730-1741); executed: no
- Verification: confirmed; history: no-rationale-found (the module doc entered in 434863669 on 2026-07-22 and the scenario docs in 0d1ea4905 the same day; the flag day faf3cd0a landed 2026-07-25; neither doc was rewritten)
- Owner-gated: no

The envelope suite's module doc says the implementation is "far from" its
contract, and the `cmp_*`/`decode_*`/`tick_*` scenario docs describe
recursion frames growing stack segments and a quadratic per-frame comparison.
The envelope table beside them pins `segments = 0` on every row and names the
iterative sweep, and the crate docs promise every asymptotic claim as a hard
guarantee.

Evidence:

         7	//! on value magnitude, tree depth, or encoded size. Today's implementation
         8	//! is far from that — several operations amplify their input by large
         9	//! constants or worse — so every scenario here pins the *current* measured
        10	//! cost, with ×1.25 slack, as a ceiling. A regression fails loudly now; each
        11	//! improvement tightens a committed number.

       440	/// Comparing the dense spine against the empty version stays within its
       441	/// envelope (the recursion-frame cost: heap stays flat, segments do not).

       983	/// Comparing bigroot against the empty version stays within its envelope
       984	/// (today the worst amplifier: per-frame owned path sums, quadratic in the
       985	/// root magnitude × depth).

       263	    pub const CMP_DENSE: Envelope = envelope(30_720, 0, 0, 0); // the iterative sweep over the Bytes-backed at-rest form (OpenedPair states the pair walk's opening move once); word-valued payloads keep the limb column at zero
       269	    pub const CMP_BIGROOT: Envelope = envelope(40_340, 0, 783, 469); // the iterative sweep over the Bytes-backed at-rest form; the wide root's decode is the limb record

       351	//! pathological input shapes. Any asymptotic claim is a hard guarantee that the
       352	//! operation will perform in time proportionate to that bound, for all input

Resolution: rewrite lines 4-11 to state what the suite pins (exact counter
envelopes at measured ×1.25, the limb floors, the committed known-bad
kernels) without the status narrative; restate the scenario docs at 428-429,
440-441, 465-466 and 983-985 from the envelope-table comments (iterative
sweep; validate-plus-wrap decode; the fused tick), dropping "today" and every
recursion or quadratic description. Acceptance: no scenario doc in the file
describes a mechanism its own pinned row contradicts, and `grep -n today
crates/before/tests/meter.rs` returns nothing.

### prose-hygiene-2: Tier 2 compactness apparatus describes a representation decision that has been taken
- Where: crates/before/src/meter/tier2.rs:1-19 (related: crates/before/src/meter/tier2.rs:23-32; crates/before/src/testing/compactness.rs:1-20, 31-44, 59-67, 78-84, 115-125; crates/before/src/meter/tier2/tests.rs:223-236, 705-709; crates/before/src/meter.rs:307-309; crates/before/src/version/skyline.rs:1-2, 26-28)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (read every listed range; `git log` on tier2.rs and compactness.rs; read `.agent-notes/2026-07-23-before-skyline-encoding` §12); executed: no
- Verification: confirmed; history: deliberate-but-expired (the instruments landed 2026-07-23 in 3e1a1631 and f2d0011b to inform "the open decision" the skyline-encoding note's §12 names; faf3cd0a closed it on 2026-07-25; the 2026-07-31 excision touched compactness.rs only to remove two calendar dates and left every "today")
- Owner-gated: yes: whether the ratio check survives as a stored-versus-per-node size bound is the owner's call; the prose fix is unconditional

`tier2.rs`, `testing/compactness.rs` and their tests call the Tier 2 coding a
candidate whose adoption a decision "turns on" and call the per-node coding
"today's"; `skyline.rs` states that the stored form is the Tier 2 coding, and
`check_sample`'s own comment concedes it. "Today" now denotes a coding that
survives only as the oracle-lowered `packed_bits_of` form.

Evidence:

         5	//! large a canonical [`Version`](crate::Version) would be if re-encoded as its
         6	//! preorder topology (one flag bit per node, exactly as today) plus its leaf
        10	//! compactness ratio between this size and today's encoded size is the evidence
        11	//! the representation decision turns on, so the walk here is written for

    compactness.rs:
         3	//! The Tier 2 coding stores preorder topology plus delta-coded absolute leaf
         4	//! values ([`crate::meter::tier2`]); the claim its adoption turns on is that
         5	//! its coded size never exceeds ~2x today's size plus O(1) bits per node.
        78	    // The decision-era "current" coding is the min-lifted packed preorder
        79	    // stream (one gamma-coded base per node), re-derived through the
        80	    // oracle lowering; the stored coding is Tier 2 itself.

    skyline.rs:
        26	//! This coding is the stored and wire form of a [`Version`]:

Resolution: owner call on the instrument: dissolve `meter::tier2` and
`testing::compactness` (the claim is settled and the stored coding is the
measured one), or re-denominate them as a stored-coding-versus-per-node-coding
size bound with the per-node coding named for what it is
(`testing::bridge::packed_bits_of`) and every "today"/"decision" phrase
removed. Either way rewrite tier2.rs:1-19 and 23-32, compactness.rs:1-20,
31-44, 59-67 and 115-125, tier2/tests.rs:223-236 and 705-709, and
meter.rs:307-309 in the present tense over what is. Acceptance: `grep -rn -i
"today\|decision" crates/before/src/meter/tier2* crates/before/src/testing/compactness*`
returns nothing, and the module docs state which two codings are compared
and why the comparison is kept.

### prose-hygiene-3: Design-doc citation from code in the board tests, and the doc has moved
- Where: crates/before/src/meter/board/tests.rs:505-505 (related: .agent-notes/2026-07-22-before-adversarial-resource-amplification/, .agent-notes/2026-08-19-design-directory-migration/README.md)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read 495-512; `git log -S` on the phrase; located the cited document with grep over `.agent-notes/`); executed: no
- Verification: reframed: the citation breaches the rule, and its target no longer sits where "the design doc" implies. The adversarial-resource-amplification design document was moved from `design/` into `.agent-notes/2026-07-22-before-adversarial-resource-amplification/` by the 2026-08-19 migration; the root `design/` holds only `rumors-frame-fuzz.md`; history: no-rationale-found (introduced in d3de44396 on 2026-07-26)
- Owner-gated: no

A test doc comment sends the reader to "the design doc's §3 entry" for the
cure's decision record instead of stating the mechanism, and the pointer now
resolves to an agent note.

Evidence:

       504	/// board's `party_join_all_overlap` row carries the same reading at the scales
       505	/// of record; the cure's decision record lives in the design doc's §3 entry.

Resolution: delete the clause after the semicolon; if the cure's rationale
matters to the test, state it in one sentence here. Acceptance: no in-scope
code or rustdoc contains "design doc" or a section-number pointer.

### prose-hygiene-4: Opaque roster IDs PROG-5 / COV-7 in the fuzz workspace
- Where: crates/before/fuzz/Cargo.toml:1-1 (related: crates/before/fuzz/Cargo.toml:48; crates/before/fuzz/README.md:1)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn -E 'PROG-5|COV-7'` over the whole repository including `.agent-notes/`: the three sites are the only occurrences); executed: no
- Verification: confirmed; history: no-rationale-found (the roster these tags index exists nowhere in the tree, the resurrected design notes included)
- Owner-gated: no

Three comments carry tags from a roster no reader of the tree can resolve.
The invariant at line 48 is already named in plain language in the same
sentence.

Evidence:

         1	# Standalone fuzz workspace (PROG-5 / COV-7). The empty `[workspace]` table detaches
        48	# keystone invariant (COV-7) inline — an accepted value re-encodes stably and decodes

    README.md:
         1	# `before` fuzz targets (PROG-5 / COV-7)

Resolution: delete the three tags. Acceptance: `grep -rn -E '\b[A-Z]{2,4}-[0-9]+\b'`
over the fuzz workspace returns only UTF-8 and license identifiers.

### prose-hygiene-5: "mint" for constructing a value at 82 sites, including the identifier Reign::mint
- Where: crates/before/src/version/skyline/query/web.rs:190-190 (related: web.rs:300, 311, 330; crates/before/src/lib.rs:49 and its derived crates/before/README.md:53; crates/before/src/party.rs:15, 872; crates/before/src/version/skyline.rs:7; crates/before/src/meter.rs:18, 1383, 2673; crates/before/src/meter/board/family.rs:1123; the full list of 82 lines is reproducible with `grep -rn -i -E '\bmint(ed|s|ing)?\b'` over the in-scope surfaces)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (recounted: 82 lines, 32 in non-test rustdoc; read the sites quoted); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The writing-style rule is unconditional ("Never write *mint* for constructing
a value — in code, prose, or comments"); the word appears in the crate's
front-page example, in public module docs, in assert messages, and as a
private constructor name.

Evidence:

       190	    fn mint(sign: Sign, offset: &Base, epoch: u32) -> Reign {

    lib.rs:
        49	//! // New participants fork off a live clock, never mint themselves.

    party.rs:
       872	/// Mints identity exactly as the `u8` literal door does — a test and

    skyline.rs:
         7	//! module's cursor vocabulary mints the term). Topology plus absolute leaf

    family.rs:
      1123	        "the disjoint-mount adapter must mint a disjoint pair"

Resolution: rename `Reign::mint` to `Reign::new` (private; no API movement)
and replace every prose, comment and message use with construct, create,
build, or the specific operation (fork, split, emit, allocate); rerun `just
readme` so README.md:53 follows lib.rs:49. Acceptance: the grep above returns
nothing across the in-scope surfaces.

### prose-hygiene-6: Prose still describes owner rationales and re-pin annotations as dated after the dated-notes excision
- Where: crates/before/src/meter/board.rs:170-172 (related: crates/before/src/meter/board/ceilings.rs:233-235; crates/before/fuzzfit/harness/src/bands.rs:5-8, 325-326, 924-925; crates/before/fuzzfit/harness/src/bin/calibrate.rs:373-374, 384-385)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the sites; `grep -rn -E '20[0-9]{2}-[0-9]{2}-[0-9]{2}'` over crates/before/src/meter/board/ finds no date; read d2a9d04e's message and its ceilings.rs hunks, which replace the dated rationales with undated "(owner-ratified: ...)" text); executed: no
- Verification: confirmed (new finding, not in the sweep's report); history: no-rationale-found (d2a9d04e rewrote the declaring constants and collapsed bands.rs's dated re-pin ledger but left these sentences)
- Owner-gated: no

The board module doc and the ceilings header say each declared model carries
"a dated owner rationale committed at the declaring constant"; the declaring
constants read "(owner-ratified: ...)" with no date, as the excision commit
intended. The bands.rs re-pin instruction, and the calibrate.rs template that
regenerates two copies of it, tell the re-pinner to "commit with a dated
movement annotation", the convention the same commit retired in favour of
"movement between pins belongs to the re-pinning commit".

Evidence:

       170	//! Some cells are judged against a **declared model** — a ratified cost law
       171	//! derived at the cell, with a dated owner rationale committed at the declaring
       172	//! constant — in place of one global ceiling, because the global form is

    ceilings.rs:
       119	/// model. The ceiling is the worst honest reading ×1.25, rounded up
       120	/// (owner-ratified: the family-stated ceilings' margin convention; the
       121	/// reading lives in the pin commit), so a kernel that re-reads digit state
       233	// Some cells carry a *declared model* in place of one global ceiling: a
       234	// ratified cost law, derived and priced at the cell with a dated owner
       235	// rationale, that the readings must match — the global ceiling would otherwise

    bands.rs:
         7	//! `just fuzzfit-calibrate`, review the diff like a snapshot, and commit
         8	//! with a dated movement annotation.

    calibrate.rs:
       373	/// Generated by `just fuzzfit-calibrate` — review the diff like a
       374	/// snapshot, commit with a dated movement annotation.

Resolution: in board.rs:171 and ceilings.rs:234 write "with the owner
ratification stated at the declaring constant"; in calibrate.rs:373-374 and
384-385 (the source of bands.rs:325-326 and 924-925) and in bands.rs:7-8
write "commit with the movement and its attribution in the commit message",
then run `just fuzzfit-calibrate` so the generated copies follow.
Acceptance: `grep -rn -w dated crates/before/src/meter/board crates/before/fuzzfit`
returns nothing.

### prose-hygiene-7: Ruling dates embedded as code data, consumed only by a date-shape check
- Where: crates/before/src/meter/registry.rs:1102-1103 (related: registry.rs:1078-1079, 1094-1095, the ten `decided:` literals at 1859, 1864, 1893, 1923, 1945 and surfacecheck check.rs:60, 68, 77, 86; the nine `decided: REGISTRY_RATIFIED` uses; crates/before/src/meter/registry/tests.rs:204-224; crates/before/surfacecheck/src/check.rs:33-34, 259-273; the prose describing the schema at registry.rs:42, 50, 58, 566, 585, 1035, 1039, 1074, 1089, 1960, board.rs:240, check.rs:7, 149, main.rs:16)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -w decided` over before/src, surfacecheck/src and tests: every read of the field is a shape check; read both checks and the excision commit's message); executed: no
- Verification: confirmed; history: already-known (d2a9d04e's message reports the machinery to the owner verbatim: "the enforced dated-exception machinery ... requires field and test changes to dissolve — a design round, not a prose sweep")
- Owner-gated: yes: dissolving the field is a schema and test change the owner has already been asked to rule on

Every ruling carries a `decided: "YYYY-MM-DD"` and the registry a
`REGISTRY_RATIFIED` constant; the only readers check that the string is
shaped like a date. `git log -S` on any reason string yields the ruling date
exactly.

Evidence:

      1102	/// The date the registry's rulings were ratified as the rows of record.
      1103	const REGISTRY_RATIFIED: &str = "2026-07-29";

    registry/tests.rs:
       215	                decided.len() == 10 && decided.chars().filter(|&c| c == '-').count() == 2,
       216	                "{family:?}'s ruling date `{decided}` is not a YYYY-MM-DD date"

    check.rs:
        33	    /// The date of the ruling of record (YYYY-MM-DD).
        34	    pub decided: &'static str,

Resolution: owner ruling. Keep: say once at each `decided` field's doc that
the registry is an embedded decision record and dates are part of its
schema. Dissolve: drop the `decided` fields, `REGISTRY_RATIFIED`, the two
shape checks, and rewrite the fourteen "dated reason/ruling/exception"
phrases listed above to "reason of record". Acceptance: either the field's
doc states the decision-record rationale, or `grep -rn -w -i dated` over
before/src and surfacecheck returns nothing.

### prose-hygiene-8: Historical narration at declaration sites ("retired", "the old", "before this binding existed")
- Where: crates/before/src/testing/validation_index.rs:101-102 (related: crates/before/tests/meter.rs:2744-2746, 9880-9887; crates/before/src/version/skyline/query/tests.rs:2433-2435, 2446-2447; crates/before/src/meter/board/tests.rs:1009-1010; crates/suanpan/src/claims.rs:12-14)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read each site in a ten-line window; `git log -S` on each phrase); executed: no
- Verification: confirmed; history: no-rationale-found (introduced across 2026-07-24 to 2026-07-29: 8b1610d46, 016b91c4a, 669cf3103, a4cc1cf35, e2f4e2a5e, 9e7b7ce33; none records a reason to describe the present by its predecessor)
- Owner-gated: no

Several docs describe committed known-bad kernels or current checks by what
they replaced rather than by what they are; the suanpan bullet carries a
review-incident count as rationale.

Evidence:

       101	//! committed known-bad kernels (schoolbook converters, sequential-reduce
       102	//! folds, retired quadratic walks) held red beside the green pins. What

    tests/meter.rs:
      2745	    /// tightened record that retired the frozen-width-per-tooth
      2746	    /// quadratic baseline.
      9880	    /// resolution and strictly under the floor-first two-check shape it
      9881	    /// replaces; against the old *first check alone* the earlier bail

    query/tests.rs:
      2446	    /// The retired per-digit charge: one `parked`-wide product per
      2447	    /// balanced digit of the mass.

    claims.rs:
        12	//!   row must be named by some claim — the table was twice found wrong
        13	//!   in review before this binding existed, so it is held to the roster

Resolution: name each known-bad kernel by mechanism ("the per-digit
schoolbook charge", "the frozen-width-per-tooth kernel", "quadratic
re-walks") and describe each current check without its predecessor; delete
"the table was twice found wrong in review before this binding existed" (the
bullet already states the binding's purpose). Acceptance: `grep -rn -E
'\bretired\b|\bthe old\b|\bit replaces\b|before this binding'` over the
listed files returns nothing.

### prose-hygiene-9: "today"/"currently"/"none at this tip" as relative-time naming of the present implementation
- Where: crates/before/src/meter/board/floors.rs:173-177 (related: floors.rs:182, 220, 233, 265, 291; crates/before/src/meter/board/currency.rs:139; crates/before/fuzzfit/harness/src/strategies.rs:117; crates/before-fuelscape/src/compact.rs:42; crates/before/src/testing/fuelscape_islands.rs:19; crates/before/surfacecheck/src/check.rs:37; crates/before/tests/foreign_reexport.rs:18; crates/before/AGENTS.md:34)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for the terms outside tier2/compactness: 24 lines; each listed site read in context); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Liveness-floor strings, a policy doc, a guard doc, the guidepost, and three
empty-roster notes mark the present with "today"/"currently". In each, the
sentence already says what holds and what change would move it, so the word
is either redundant or a hand-maintained state note ("currently empty",
"none at this tip") that rots when the literal changes.

Evidence:

       173	pub(super) const WHY_LIMB_RANK_ENCODE: &str =
       174	    "deterministic-liveness: the encoder extracts and biases the integral part through \
       175	     one arithmetic pass over the numerator today, one op per 64 numerator bits; a \
       176	     pure bit-walk emission (riding the bias as a carry) would lower this floor \
       177	     deliberately";

    currency.rs:
       139	/// NA on every cell today: the target is walks that never grow the stack, so

    fuelscape_islands.rs:
        19	/// Currently empty: every measured operation's island reaches the

    check.rs:
        37	/// Per-item exceptions: none at this tip. An entry here is a deliberate,

    foreign_reexport.rs:
        18	//! library source (today: none), so adding one is a reviewable diff

    AGENTS.md:
        34	  heap before a deep input can overflow — today those are only test

Resolution: delete "today" in the six floors.rs strings, currency.rs:139,
strategies.rs:117, compact.rs:42 and AGENTS.md:34; in the three empty-roster
docs keep only the sentence stating what an entry means and let the empty
literal speak. Acceptance: `grep -rn -i -E '\btoday\b|currently empty|none at this tip'`
over the listed files returns nothing.

### prose-hygiene-10: Moralized code: "honest" (234) and "genuine" (65) in place of the property that holds
- Where: crates/before/src/meter/board/floors.rs:5-5 (related: per-file counts of `honest`: tests/meter.rs 29, board/ceilings.rs 25, board/floors.rs 18, board/cell.rs 15, board/tests.rs 13, fuzzfit bands.rs 10, party/tests.rs 8, board/ops.rs 7, board/family.rs 7, board.rs 6; crates/before/tests/amp_board_smoke.rs:231-232; crates/before/src/meter/board/currency.rs:140; crates/before/src/testing/compactness.rs:43; justfile:837; .cargo/mutants.toml:93)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (recounted with `\bhonest(ly|y)?\b` and `\bgenuine(ly)?\b`; per-file tallies recomputed; sites quoted were read); executed: no
- Verification: confirmed (my count for the honest family is 234 because it includes "honesty"; the sweep's 213 excluded it); history: no-rationale-found
- Owner-gated: no

"Honest floor", "honest improvement", "honest reading", "honestly refuses",
"genuinely quadratic" stand where a property is meant (derived from
irreducible work; attributed; non-vacuous; a class regression), and one test
binds a variable `honest`.

Evidence:

         5	//! A floor states the least a watching counter can honestly read, derived from

    currency.rs:
       140	/// its honest floor is zero, and a zero floor asserts nothing.

    amp_board_smoke.rs:
       231	    let honest = in_process_spawn(1, &heap)(SMOKE_SCALE).expect("in-process capture succeeds");

    justfile:
       837	# honest tree and fails on any unexpected red OR unexpected green.

    mutants.toml:
        93	    # carries the argument), and no honest meter band can price a zero

Resolution: sweep both words: "honest floor" to "the floor" (its derivation
is stated beside it), "honest improvement" to "an attributed improvement",
"honest reading/tree" to "unmutated" or "within the model", "genuinely
quadratic" to "quadratic"; rename the `honest` binding to `captures`.
Acceptance: the two greps return nothing outside quoted test names.

### prose-hygiene-11: Unanchored coinages: door (279), knob (107), seam (276), luck-proof (1)
- Where: crates/before/src/meter.rs:24-27 (related: crates/before/src/meter/registry.rs:63, 87-90, 273-281, 565, 585; crates/before/src/codec/bits.rs:264; crates/before/src/codec/tree.rs:25; crates/before/src/party.rs:14, 872-873; crates/before/src/meter.rs:2667-2683; crates/before/src/testing/asymptotics.rs:117; crates/before/src/span/tests.rs:169, 908)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (recounted; searched for italic definitions `*door*`, `*knob*`, `*seam*` in before/src and suanpan/src: none; searched identifiers: `seam_rung`, `seam_wide`, `seam_plunge*`, `seam_stop*`, `SeamPlunge*`, `SeamStop*`, `door_scan_bits`, two `*_doors_*` test names; read the seam-shape docs at meter.rs:2667-2700 and registry.rs:183-196); executed: no
- Verification: confirmed, with one refinement: the `seam_*` and `Seam*` identifiers name shape families, and their docs use "the seam shapes" and "propagate-seam" without ever saying what a seam is, so the identifiers do not anchor the term; "kernel-seam probe" (21 uses) is a second sense with no definition either; history: no-rationale-found
- Owner-gated: yes: with door at 279 lines and seam at 276, the choice between one definition site and a plain-term sweep is the owner's

Four recurring terms have neither a defining identifier nor a
define-once-by-contrast sentence, against the anchoring rule; the style
tables list door, knob, and seam as default-dialect for entry point,
parameter, and boundary.

Evidence:

        24	//! additionally earns a column on the amplification board ([`board`]) only when
        25	//! it is a whole-surface adversary rather than a kernel-seam probe (the
        26	//! criterion, each family's coverage answer, and the luck-proof touch list sit
        27	//! on the registry's [`FamilyId`](crate::meter::registry::FamilyId)).

    registry.rs:
        63	//! Two seams stay pinned by tests instead of types, each a deliberate,
       273	    /// One size knob to one packed shape.

    codec/bits.rs:
       264	    /// The byte decode doors' entry: a door walks its whole input buffer as

    meter.rs:
      2673	/// Every stacked boundary and every dying residue the seam shapes mint holds

Resolution: per term, either define once at first use in the owning module
(registry.rs for knob and both senses of seam; codec.rs or the lib.rs
implementation essay for door) or replace with the plain term where the count
is small (luck-proof: name the touch list's property; seam outside the shape
names: boundary). Acceptance: each surviving term has an italic definition
site the module doc links, or the greps return only the shape-family
identifiers.

### prose-hygiene-12: Em-dashes in // and # comments and in message string literals
- Where: crates/before/tests/meter.rs:251-252 (related: plain `//` lines per file: tests/meter.rs 107, skyline/fill.rs 23, skyline/fill/tests.rs 19, version/tests.rs 18, board/ops.rs 17, codec/tests.rs 17; string-literal lines: before-fuelscape/src/ops.rs 31, tests/meter.rs 23, board/coverage.rs 10, surface.rs 9, registry.rs 6, suanpan claims/tests.rs 4; non-Rust: justfile 57, docs/fuelscape-header.html 24, docs/fuelscape.js 22, .cargo/mutants.toml 20, ci.yml 9, scripts 7, Cargo.toml comments 13)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (partitioned every U+2014 hit over the in-scope file set by line prefix: 4140 total, 3255 rustdoc, 551 plain `//`, 104 Rust code lines, 230 non-Rust of which 58 are the two cargo-rdme-derived READMEs); executed: no
- Verification: confirmed (the sweep's 553/106/~170 reproduce within a few lines); history: no-rationale-found
- Owner-gated: no

The owner's doctrine puts spaced double-hyphens in code comments and colons
or semicolons in messages that reach a terminal, with true em-dashes
reserved for rendered prose. Rustdoc and the derived READMEs are exempt;
the remaining 655 Rust lines and 172 non-Rust lines are not.

Evidence:

       251	// mechanism that prices the row; the measurements of record — and every
       252	// re-pin's movement and attribution — live in the pin commits (`git log

    board/coverage.rs:
       181	        "Version ^ Version (BitXor, owned and borrowed — the pair hull)",

    board/coverage/tests.rs:
        78	            "{op}: priced by board rows AND excused in BOARD_NOT_APPLICABLE — \

Resolution: mechanical sweep: in `//` and `#` comments replace ` — ` with
`: `, `; ` or ` -- ` by sentence sense; in string literals that reach a
terminal replace with colons. A `tools/` linter leg rejecting U+2014 outside
`///`, `//!` and Markdown would keep it closed. Acceptance: the partition
reports zero plain-comment, code-line and non-Markdown hits.

### prose-hygiene-13: Contract paragraph intensifiers and a significance adverb in public rustdoc
- Where: crates/before/src/lib.rs:350-357 (related: crates/before/README.md:345-352 (derived); crates/before/src/version/rank.rs:198-201)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (read lib.rs:342-358, rank.rs:194-206, README.md:3 and 343-354); executed: no
- Verification: reframed: two of the sweep's three complaints are dropped. "Hardened against pathological input shapes" has a live adversary (the crate's own vocabulary opposes "organically reachable (i.e. non-adversarial) inputs" to adversarial families), so it is not a register transplant; "which we'll write `‖r‖`" follows the style guide's own "write shared reasoning as we". What survives: "very carefully" is an intensifier the contract does not need, the final clause re-ranks shapes by likelihood immediately after declaring likelihood irrelevant, and rank.rs carries an inversion ("never is larger") and "Notably"; history: no-rationale-found
- Owner-gated: no

Evidence:

       350	//! The operations in this crate have been very carefully hardened against
       353	//! sizes, no matter how unlikely and contorted the shape of the input.
       356	//! organically reachable (i.e. non-adversarial) inputs, with the asymptotic
       357	//! guarantee acting as a backstop in case of pathologically unlikely shapes.

    rank.rs:
       198	/// **A rank's representation never is larger than the version it measures, and
       201	/// expansion, which we'll write `‖r‖`. Notably, `‖r‖` is at most linear in the

Resolution: "The operations in this crate are hardened against pathological
input shapes: every asymptotic claim is a hard guarantee for every input, and
constant factors are tuned for organically reachable inputs." In rank.rs
write "is never larger" and drop "Notably". Run `just readme`. Acceptance:
the paragraph states the guarantee once and ranks no shape by likelihood.

### prose-hygiene-14: "improvement tripwire" names the benign trigger, not the failure the floor detects
- Where: crates/before/tests/meter.rs:208-210 (related: tests/meter.rs:37-53, 219-226, 400-405, 725; the 20 occurrences are all in this file)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -rn -i 'improvement tripwire'`: 20 hits, one file; read the file doc's genre paragraph, the `Envelope` doc and the assert at 398-406); executed: no
- Verification: reframed: the sweep's claim that no known-bad implementation fails the threshold is disputed by the file doc itself, which names one: a meter hook deleted from one `Base` operation reads a near-zero column and trips the floor, which is exactly the table's sense of tripwire (a test a known-bad implementation fails). What survives is the name: it foregrounds the benign trigger ("improvement", which requires a re-pin) over the failure it detects, against the table's "keeps it honest → name the failure it detects"; history: no-rationale-found
- Owner-gated: no

Evidence:

       208	/// One scenario's pinned ceilings (the measured value ×1.25, rounded up)
       209	/// and its limb improvement tripwire (measured ×0.75, rounded down — the
       210	/// file doc's tripwire genre).
       401	        "{name}: limb counter reads {limb_ops}, below the {} improvement \
       402	         tripwire (measured x0.75): attribute the drop — an honest \
       403	         improvement re-pins the band; a dead meter is the bypass this \
       404	         column exists to catch",

Resolution: rename the genre to "bypass floor" (the file's own phrase at
403-404) at the 20 sites and in the genre paragraph at 37-53; the struct
field is already `limb_floor`. Acceptance: the grep returns nothing and the
genre paragraph names the two floor genres by what each detects.

### prose-hygiene-15: Hand-maintained counts in doc comments
- Where: crates/before/fuzzfit/harness/src/bands.rs:76-77 (related: bands.rs:98; crates/before/src/borsh_impls/tests.rs:1077; crates/before/src/meter/tests.rs:1632-1634)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (counted `Band {` entries in `BANDS`: 49, so the number is currently right; located calibrate's splice marker at bands.rs:312, so lines 72-100 are the hand-written head, not regenerated; no test in fuzzfit/harness pins a key count); executed: no
- Verification: confirmed; history: no-rationale-found (d2a9d04e collapsed this section to the current pin and kept the numbers)
- Owner-gated: no

Evidence:

        76	//! the toolchain in [`PINNED_RUSTC`], wasmtime 47 fuel. 49 band keys: 44
        77	//! kernels, of which five have sampled rejection arms (`clock_join` and
        98	//! All 49 band keys read linear-or-flatter within every family above the

    borsh_impls/tests.rs:
      1077	/// Self-delimitation totality across types: for each of the 36 ordered

    meter/tests.rs:
      1632	    /// canonical family member through both callers at the closed-form

Resolution: bands.rs: state the structure (one key per kernel plus one per
sampled rejection arm, the arms listed) and let `BANDS` carry the count, or
have calibrate emit the count into the generated region; borsh tests: line
1074 already says "every ordered pair of the six wire types", drop "36";
meter/tests.rs: name `ascend_cliff` and `ascend_cliff_plateau`. Acceptance:
no doc comment in the listed files states a tally the code can change
without touching it.

### prose-hygiene-16: Residual dialect tells: load-bearing, earns, backstop, surface-as-verb, flavour, story, dial
- Where: crates/before/src/meter/board/floors.rs:136-136 (related: floors.rs:112; crates/before/src/meter/registry.rs:43, 566, 584, 887, 1036; crates/before/src/meter/board.rs:238; crates/before/src/version/skyline.rs:102; crates/before/src/clock.rs:1061; crates/before/src/meter.rs:400, 2167, 3298; crates/before/src/borsh_impls/tests.rs:19, 616, 860, 925, 993, 1183; crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:11, 29, 40; crates/before/tests/support/fuzz_seed_set.rs:34, 263, 279, 290; .github/workflows/ci.yml:26; .github/workflows/pages.yml:46; crates/before-fuelscape/src/ops.rs:2190; crates/before/src/meter/tests.rs:258; crates/before/tests/meter.rs:3850; crates/suanpan/src/lib.rs:57, 257)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (recounted: load-bearing 21, earn(s) 12, backstop 6, surface-as-verb 33, flavour 7, story 2, dial 3; each listed site read); executed: no
- Verification: reframed: "mandate" is dropped from the pattern: board.rs:173 "work their contracts mandate" and ceilings.rs:3 "an operation's own contract mandates" are the plain verb (a contract requires), leaving only the noun at registry.rs:887; the sweep's backstop count of 8 recounts as 6; history: no-rationale-found
- Owner-gated: no

Evidence:

       136	     bound, so the bench judge's time leg is the backstop that the compare stays linear";

    registry.rs:
       584	/// every family belongs on the board: a whole-surface adversary earns a column,
       887	    /// implementation, never mandate.

    skyline.rs:
       102	//! The accumulator choice is load-bearing, not an optimization: on the boundary

    borsh_impls/tests.rs:
        19	/// A borsh stream that ends mid-tree surfaces the reader's own I/O error

    fuzz_decode_ops.rs:
        29	    let Some((&flavour, rest)) = data.split_first() else {

    pages.yml:
        46	      # share one provenance story for wasm-pack.

Resolution: replace per the style tables in one sweep: load-bearing to "the
premise the bound rests on" or "essential"; "earns a column" to "meets the
board criterion"; backstop to "the check of last resort"; "surfaces as" to
"is reported as"; flavour to kind (rename the `flavour` binding); story to
"the design"; dial to parameter; "never mandate" to "never requirement".
Acceptance: the seven greps return nothing outside quoted identifiers.

## Positives

- The dated-notes excision commit d2a9d04e is the doctrine applied at scale
  and reported honestly: it collapsed two accreted history ledgers into
  standing prose, kept the toolchain pin as data, and named the `decided`
  machinery it could not dissolve as an owner question in its own message
  (verified by reading the commit).
- The envelope-table comment at tests/meter.rs:245-257 pushes every
  measurement's history to `git log -S` on the constant and keeps only the
  pricing mechanism in prose (verified by reading).
- The recurse.rs module doc's inventory of depth-recursive test surfaces
  (lines 9-14) matches the `descend!` call sites: testing/bridge.rs (4),
  skyline/grow/tests.rs (3), meter/tests.rs (2); the query/tests.rs:1167 hit
  is a comment mention (verified by grep).
- Zero TODO/FIXME/XXX/HACK markers across every in-scope surface (verified
  by grep).
- The opaque-ID discipline holds everywhere except the three fuzz-workspace
  sites: no roster tags anywhere in src, tests, benches, fuelscape, suanpan
  or surface-scan (verified by grep).
- The skyline vocabulary is anchored at its definition site: skyline.rs:4-5
  defines *skyline* in italics by contrast with the step function it names
  and introduces plateau beside it (verified by reading); the sweep reports
  the same for plateau, anchor, tooth, spine, genre, liveness floor,
  currency, band, hole/floor/ceiling, roster and fuel (sweep-reported, not
  re-checked here).
- "silently" is paired with its mechanism at the three sites spot-checked
  here (clock.rs:71 names the corrupted causal history; query.rs:389 the
  mispriced trigger; grow/tests.rs:14 the rerouted pairs); the sweep reports
  the same at 26 sampled sites and "the walk" as an actual traversal at 20
  (sweep-reported).

## Open questions for Finch

1. prose-hygiene-2: keep the compactness ratio as a documented
   stored-coding-versus-per-node-coding size bound, or dissolve
   `meter::tier2` and `testing::compactness` now that the skyline coding is
   the stored form? Recommendation: dissolve unless a consumer of the ratio
   outside the module can be named; the prose fix is unconditional either
   way.
2. prose-hygiene-7: are the `decided` fields and `REGISTRY_RATIFIED` an
   intended embedded decision-record schema (then say so once at the field
   doc), or dated rationale to dissolve? d2a9d04e's message already put this
   question to you. Recommendation: dissolve; `git log -S` on the reason
   string recovers every date.
3. prose-hygiene-11: for door (279 lines) and seam (276), one definition site
   per term or a plain-term sweep? Recommendation: define once for door and
   the shape sense of seam; replace "kernel-seam probe" (21 uses) with
   "kernel-boundary probe" and luck-proof with the property.
4. tests/meter.rs:465-466 calls the tick's cost "the fill-splice round-trip
   cost" while its row `query_env::TICK_DENSE` (line 6848) says "the fused
   tick: copy-on-first-divergence defers the output buffer past the collapse
   scan". Is "fill-splice round-trip" still the mechanism's name? Not
   asserted as a finding because the fused walk was not read here.
5. The `deterministic-liveness` floors in floors.rs (173-183 and the four
   others) are derived from the present mechanism's pass count, with "today"
   marking that fact; the metering doctrine wants floors from irreducible
   work. Deleting "today" is safe, but whether these floors are of the right
   genre is an instruments-lens question.
6. justfile:731 cites `formal/PROGRESS.md` from a rumors-only recipe: a
   build-surface design-doc citation outside this review's scope, for the
   rumors review.
7. prose-hygiene-12 proposes a `tools/` linter leg rejecting U+2014 outside
   rustdoc and Markdown; wanted, or is a one-time sweep enough?

## Dropped

- Sweep finding 9, the "hardened" clause: the crate's own vocabulary opposes
  adversarial families to "organically reachable (i.e. non-adversarial)
  inputs", so the adversary is live and the word is not a register
  transplant; the rest of the finding survives as prose-hygiene-13.
- Sweep finding 9, "which we'll write `‖r‖`": the style guide directs
  "write shared reasoning as we"; dropped from prose-hygiene-13.
- Sweep finding 11's claim that no known-bad implementation fails the
  ×0.75 floor: tests/meter.rs:46-49 and 401-404 name one (a meter hook
  deleted from one `Base` operation); the finding survives only as a naming
  nit, prose-hygiene-14.
- Sweep finding 14's "mandate" sites at board.rs:173 and ceilings.rs:3, 237:
  "work their contracts mandate" is the plain verb, not the governance noun
  the style table targets; registry.rs:887 (the noun) stays in
  prose-hygiene-16.
- Sweep finding 0's site at tests/meter.rs:466 is kept only for the word
  "today"; whether "fill-splice round-trip" misnames the fused tick is an
  open question (item 4), not a finding, because the fused walk was not
  read.
- The sweep's positive "No .agent-notes or design/ citation reaches any
  in-scope code" is narrowed: no path citation does, but board/tests.rs:505
  cites "the design doc's §3 entry" by name (prose-hygiene-3).
