# Sweep clippy-pedantic: Pedantic and nursery clippy lints over before and suanpan, judged

## Method and coverage

The sweep ran one clippy invocation with `-W clippy::pedantic -W clippy::nursery`
over the workspace members `before`, `suanpan`, and `surface-scan` (log:
`scratchpad/before/sweeps/clippy-pedantic.log`, 996 KB; the log records the
lint groups through clippy's own "implied by" notes but not the command line).
Its parser (`scratchpad/before/sweep-clippy-pedantic/parse.py`) produced
`hits.json`, which I re-read: 2175 unique hits across 60 lints, 2116 in
`before`, 57 in `suanpan`, 2 in `surface-scan`; the four largest lints
(`use_self` 1020, `redundant_pub_crate` 284, `cast_precision_loss` 116,
`missing_const_for_fn` 113) match the sweep's summary.

This pass disputed the sweep's eight numbered findings. For each I opened every
cited site with line numbers, grepped the use sites, and looked for a recorded
rationale in git history (`git blame`, `git log -S`, `git log -L`), the
before-prefixed `.agent-notes/` directories, `crates/before/AGENTS.md`, and the
`.cargo/mutants.toml` header and roster. I ran no cargo, just, or build
command: the two permitted `cargo nextest` invocations went unused because no
committed test exercises any of the disputed claims (`PackedBuilder` appears in
no `tests.rs`; `Shl<i32>` and `emit_offset` have no dedicated tests), and I may
not add one. Every finding below is therefore assessed by reading, and each
correctness or performance entry carries a construction the owner can run.

What this pass could not see: rustdoc's rendered output (finding 5's rendering
claim rests on CommonMark's rule that adjacent code spans are separate inline
nodes, not on a rendered page); the detached workspaces (`fuzz/`, `fuzzfit/`,
`wasm32-pins/`, `surfacecheck/`, `before-fuelscape`), which the sweep's
command did not lint; and the wasm32 target, on which the `u64 -> usize` casts
the sweep read would narrow.

All eight findings survive. Four are narrowed (2, 3, 5, 8), recorded in their
Verification lines and again under Dropped.

## Findings

### clippy-pedantic-1: `impl Shl<i32> for Base` serves two unsuffixed test literals and turns a negative amount into a `u32::MAX`-scale shift in release
- Where: crates/before/src/codec/base.rs:448-455 (related: crates/before/src/codec/tests.rs:54, crates/before/src/codec/tests.rs:528, crates/before/src/codec/base.rs:480-488, crates/before/src/testing/snapshots.rs:72, crates/before/src/lib.rs:417, crates/before/src/codec.rs:41)
- Class / severity / confidence: vestigial / low / high
- Provenance: assessed (read the impl and its siblings; grepped every `Shl`/`Shr` impl and every `<< <literal>` in before's src, tests, benches, examples; `git blame` and `git log -S` on the impl); executed: no
- Verification: confirmed; history: no-rationale-found (the impl is unchanged since the crate's first commit as `itc`, 94902e86, 2026-06-01; no note, comment, or roster entry mentions it)
- Owner-gated: no (`Base` is crate-private: `mod codec;` at lib.rs:417, `pub use base::Base` only inside codec.rs)

The impl exists so an unsuffixed literal shift on a `Base` compiles through the
`i32` fallback; the only such sites in the crate are two test lines shifting by
`64`. Its guard is a `debug_assert!` and its body is `rhs as u32`, so in a
release build a negative amount becomes a shift of up to 4,294,967,295 bits
handed to the backend rather than a named failure. A production impl carried
for two test spellings fails the "name what it serves outside itself" test
(Principle 3), and a debug-only guard in front of a lossy sign cast fails the
panic doctrine's "every guard is a one-line proof in every profile".

Evidence:

       448	impl Shl<i32> for Base {
       449	    type Output = Base;
       450	
       451	    fn shl(self, rhs: i32) -> Base {
       452	        debug_assert!(rhs >= 0, "Base left shift must be non-negative");
       453	        self << rhs as u32
       454	    }
       455	}

    crates/before/src/codec/tests.rs
        54	            n = (n << 64) | Base::from(limb);
       528	            value = (value << 64) | Base::from(limb);

    crates/before/src/testing/snapshots.rs (the suffixed idiom already in use)
        72	    let big = Base::from(1u8) << 64u32; // 2^64

    crates/before/src/codec/base.rs (the house form for the side that cannot be total)
       485	        let rhs = usize::try_from(rhs)
       486	            .expect("a left shift this wide exceeds the backend's representable width");

Resolution: suffix the two literals (`n << 64u32`, `value << 64u32`) to match
snapshots.rs:72, then delete the `Shl<i32>` impl. If unsuffixed literals should
keep compiling, replace the body with
`self << u32::try_from(rhs).expect("Base left shift amount is non-negative")`
so the failure is named in every profile. Acceptance: `grep -rn "impl Shl<i32>"
crates/before/src` is empty and `just clippy` is clean; or, under the second
option, `Base::from(1u8) << -1i32` panics at the named `expect` in a
`--release` test.
Construction: delete the impl and run `cargo check -p before --all-targets`;
exactly codec/tests.rs:54 and :528 fail with `no implementation for Base <<
i32`, which confirms the caller set.

### clippy-pedantic-2: `PackedBuilder::patch_bit` truncates the offset to `u32` before the assert that implements its documented panic
- Where: crates/before/src/codec/build.rs:155-159 (related: crates/before/src/codec/build.rs:141-143, crates/before/src/codec/build.rs:321-322, crates/before/src/codec/build.rs:287, crates/before/src/party/ops/build.rs:130-131)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read build.rs:60-175 and 284-326; grepped every caller of `patch_bit`, `read_bits`, `bit_at`, `reserve`; `git log -L` on `patch_bit`); executed: no
- Verification: reframed: the sweep names `read_bits` as sharing the pattern, but its `debug_assert!` at line 287 checks the untruncated `pos + u64::from(n) <= self.len()` at entry, so only `bit_at` (321-322) shares the truncated-check shape; history: no-rationale-found (the cast-before-assert order dates to 525e7324, when `at` was `usize` and the cast was exact, and survived the `u64` widening at 83e61b4d unchanged)
- Owner-gated: no (`pub(crate)`)

The `# Panics` section promises a panic for any `at` at or past the output
length, but the staged-region arm computes `(at - committed) as u32` first and
asserts on the truncated value, so an `at` with `at - committed >= 2^32` whose
low 32 bits fall below `staged_len` passes the assert and patches a staged bit
instead of panicking. Today's only callers (party/ops/build.rs:130-131) pass
positions returned by `reserve`, so the trigger is programmer error only; the
finding is that the check proves less than the sentence above it says (panic
doctrine: a documented panic is a contract clause and its check is its proof).

Evidence:

       141	    /// # Panics
       142	    ///
       143	    /// Panics if `at` is at or past the current output length.
       144	    pub(crate) fn patch_bit(&mut self, at: u64, bit: bool) {
       ...
       154	        } else {
       155	            let offset = (at - committed) as u32;
       156	            assert!(
       157	                offset < self.staged_len,
       158	                "patch position {at} is past the output"
       159	            );

       321	            let offset = (pos - committed) as u32;
       322	            debug_assert!(offset < self.staged_len, "read past the output");

    the entry check that `read_bits` already has (untruncated):
       287	        debug_assert!(u64::from(n) <= SMALL_CODE_BITS && pos + u64::from(n) <= self.len());

Resolution: assert on the untruncated position before the cast:
`assert!(at < self.len(), "patch position {at} is past the output");` and then
`let offset = (at - committed) as u32;` is exact because `at - committed <
staged_len`. Apply the same reorder to `bit_at`'s `debug_assert!` (321-322).
Acceptance: the construction below panics with the documented message.
Construction: in a codec unit test, `let mut b = PackedBuilder::with_capacity(0);
b.push_bit(false); b.patch_bit(1u64 << 32, true);`. Today `committed == 0`,
`offset == 0 < staged_len == 1`, no panic, and `finish()` carries a set bit the
caller never wrote; after the fix the call panics as documented.

### clippy-pedantic-3: `emit_offset` takes `Signed` by value but only borrows it, forcing a clone at both consumed-site callers
- Where: crates/before/src/version/skyline/fill.rs:892-894 (related: crates/before/src/version/skyline/fill.rs:731, crates/before/src/version/skyline/fill.rs:790, crates/before/src/version/skyline/fill.rs:463, crates/before/src/version/skyline/fill.rs:504, crates/before/src/version/skyline/fill.rs:596, crates/before/src/version/skyline/fill/prescan.rs:439, crates/before/src/version/skyline/signed.rs:96-103, crates/before/src/codec/int.rs:18-26)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read fill.rs:455-470, 498-510, 588-602, 670-760, 780-800, 884-935, signed.rs:55-62 and 90-115, codec/int.rs:1-60; grepped every `emit_offset` caller); executed: no
- Verification: reframed: `Signed::magnitude` is an `Int`, which is `Small(u64) | Wide(Base)`, so the clone allocates only when the magnitude is wide; for word-sized magnitudes it is a small copy. The redundancy stands regardless; history: no-rationale-found (the sibling `emit_offset` in prescan.rs:439 already takes `&Signed`)
- Owner-gated: no (private)

`emit_offset` reads `offset` only through `&offset`, `offset.is_zero()`,
`.sum(&offset)`, `offset.sign` (a `Copy` enum), and `&offset.magnitude`
(lines 893, 894, 920, 929, 934), yet takes it by value. Both consumed-site
callers hold `above: &Signed` and must write `above.clone()`. Changing the
parameter to `&Signed` deletes both clones; the three owned-argument callers
(463, 504, 596) pass a reference. Strict deletion of redundant work has a fixed
sign (doctrine under Principle 4): construct and measure, no gating.

Evidence:

       892	    fn emit_offset(&mut self, depth: u64, offset: Signed) {
       893	        self.web.emit_offset(&offset);
       894	        if self.out.is_verbatim() && self.range_is_leaf && offset.is_zero() {

       731	                    self.emit_offset(depth + 1, above.clone());
       790	            self.emit_offset(depth + 1, above.clone());

    crates/before/src/version/skyline/fill/prescan.rs (the sibling already by reference)
       439	    fn emit_offset(&mut self, offset: &Signed) {

    crates/before/src/codec/int.rs
        18	#[derive(Clone, Debug, PartialEq, Eq)]
        19	pub(crate) enum Int {
        20	    /// A value within the machine-word range.
        21	    Small(u64),
       ...
        25	    Wide(Base),

Resolution: `fn emit_offset(&mut self, depth: u64, offset: &Signed)`; pass
`above` at 731 and 790, `&above` at 463 and 596, `&value_offset` at 504.
Re-run the board's fill-family heap cells at the parent and at the change;
tighten any pin that moves. Acceptance: `just gate` clean and no `.clone()`
argument to `emit_offset` remains; any moved heap pin is re-committed with the
attribution.
Construction: under the `limb-meter` build, a tick whose consumed sites carry
wide `above` magnitudes shows two fewer big-integer allocations per consumed
site after the signature change.

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

### clippy-pedantic-5: 87 doc links spelled `[`name`]`(args)`` render as two adjacent code spans with the link on only the first
- Where: crates/before/src/span.rs:36-37 (related: crates/before/src/party.rs:583, crates/before/src/version.rs:1136, crates/suanpan/src/accumulator.rs:745, crates/suanpan/src/accumulator.rs:894, crates/before/src/version/skyline/place.rs:51, crates/before/src/party/tests.rs:629, crates/before/src/meter.rs:1215, crates/before/src/meter.rs:2704, crates/before/src/meter.rs:3328-3329, crates/before/src/meter/registry.rs:98)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (the 87 `doc_link_code` hits recounted from hits.json by file: 68 meter/registry.rs, 11 meter.rs, 2 span.rs, 2 suanpan/accumulator.rs, 1 each party.rs, party/tests.rs, version.rs, skyline/place.rs; each anchor read verbatim); executed: no
- Verification: reframed: the sweep counts "6 in public rustdoc", but party/tests.rs:629 is test code and place.rs:51 is a private module doc; the public sites are span.rs:36-37, party.rs:583, version.rs:1136 (before) and accumulator.rs:745, 894 (suanpan), and the 79 meter sites sit in the feature-gated `pub mod meter`. The rendering claim is by CommonMark's rule (adjacent code spans are separate inline nodes), not a rendered page; history: no-rationale-found
- Owner-gated: no

A backticked intra-doc link immediately followed by a second backtick span
carrying the arguments renders as two `<code>` elements, the link ending before
the parenthesis. Writing the whole call as one code span with an explicit
target renders one span, wholly linked. Documentation altitude: public rustdoc
is read rendered, and the one-span form is what the author meant.

Evidence:

        36	/// | [`Span::new`]`(lo, hi)`                               | the span `lo <= hi`; errors unless `lo <= hi`                       |
        37	/// | [`Span::at`]`(v)`                                     | the singleton span `v <= v`                                         |

    crates/before/src/party.rs
       583	    /// `encode().len()` or [`as_bytes`](Self::as_bytes)`.len()` — the byte
    crates/suanpan/src/accumulator.rs
       894	    /// [`sign`](Accumulator::sign)`() == Equal` is the exact zero test,

Resolution: mechanical pass: `[`Span::new(lo, hi)`](Span::new)`,
`[`as_bytes().len()`](Self::as_bytes)`, `[`sign() == Equal`](Accumulator::sign)`,
and so on. Start with the six public sites; the meter module docs can follow in
the same commit. Acceptance: `just doc` (or the gate's rustdoc leg) clean and
the `doc_link_code` hit count at zero under a re-run of the sweep's command.

### clippy-pedantic-6: Six items whose first doc paragraph runs three to four lines, so the module listing shows a block instead of a name
- Where: crates/before/src/surface.rs:319-321 (related: crates/before/src/meter.rs:3655-3658, crates/before/src/meter/board/ceilings.rs:389-392, crates/before/src/meter/board/shard.rs:128-130, crates/before/src/meter/board/worst.rs:58-61, crates/surface-scan/src/lib.rs:64-67)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (the six `too_long_first_doc_paragraph` hits listed from hits.json; each site read verbatim); executed: no
- Verification: confirmed; `surface` and `meter` are `pub mod` behind the meter feature (lib.rs:438-442); history: no-rationale-found
- Owner-gated: no

rustdoc's item listing shows the first paragraph; at these six sites it is a
three-to-four-line block (surface.rs:319-321 is two sentences in one
paragraph). Splitting after the first sentence gives the listing a one-line
name. Documentation altitude: a doc comment's first sentence stands alone in a
module listing.

Evidence:

       319	/// The roster over the mechanically-extracted inherent `pub fn` surface.
       320	/// The surface-coverage suite's `roster_is_total_over_the_public_fn_surface` holds
       321	/// this equal, name for name, to its extractor's listing.
       322	pub const METHOD_SURFACE: &[SurfaceRow] = &[

    crates/before/src/meter/board/worst.rs
        58	/// The measurement ladder's two sampling scales, at each of which the
        59	/// worst-case map is rendered and pinned: the board's seconds-scale base and
        60	/// the ladder top ([`LADDER_TOP_SCALE`], which owns the ×4 calibration
        61	/// argument).

Resolution: insert a blank `///` line after the first sentence at each site;
where the first sentence itself runs four lines (worst.rs:58-61,
ceilings.rs:389-392) shorten it to the noun phrase and move the qualification
down. Acceptance: the six hits disappear under a re-run of the sweep's command.

### clippy-pedantic-7: A bundle of pedantic hits that are pure spelling improvements with no behavior change
- Where: crates/before/src/codec/base.rs:46-50 (related: crates/before/src/version/skyline/build.rs:138, crates/before/src/version/rank.rs:627-629, crates/before/src/version/skyline/watermark.rs:1160, crates/before/src/version/skyline/watermark.rs:1171, crates/before/src/meter/board/floors.rs:751-752, crates/before/src/causally/query.rs:39, crates/before/src/causally/query.rs:61, crates/before/src/party/ops/sum_split.rs:4, crates/before/src/shape.rs:333, crates/before/src/span/algebra.rs:60, crates/before/src/span/wire.rs:20, crates/before/src/version/ranked.rs:85, crates/before/src/span/tests.rs:662, crates/before/examples/emit_probe.rs:25, crates/before/examples/emit_probe.rs:147, crates/before/examples/emit_probe.rs:163, crates/before/src/version/skyline/grow.rs:654, crates/before/src/oracle/version.rs:124, crates/before/src/codec/buf.rs:254)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (hit lists for `map_unwrap_or`, `manual_ilog2`, `elidable_lifetime_names`, `needless_pass_by_ref_mut`, `ref_option`, `trivially_copy_pass_by_ref`, `unreadable_literal`, `uninlined_format_args`, `match_wildcard_for_single_variants`, `cast_lossless`, `redundant_clone` listed from hits.json; every anchor read verbatim; `Num::bits` and `plus_one` signatures read at rank/num.rs:174 and 232); executed: no
- Verification: confirmed, with two details settled: `Num::bits` returns `u64` and `biased` is `... .plus_one()`, so `w >= 1` and `w.ilog2()` is total; the `needless_pass_by_ref_mut` hits at watermark.rs:1160 and 1171 are on `&mut self` (column 34), whose bodies only read `self.gap`. `cast_lossless` (26) sits entirely in examples, benches, and test-support modules; `redundant_clone` (51) is in tests except grow.rs:654, where `original` (a `Range<u64>`) is cloned at its last use; history: no-rationale-found
- Owner-gated: no (the one public signature touched, `shape::combine` at shape.rs:333, keeps its meaning under lifetime elision)

Each replacement says the same thing in fewer or truer words: `is_ok_and` /
`is_some_and` where buf.rs:254 already uses the latter; `w.ilog2()` for what
the comment at rank.rs:620 calls `ρ = bits(w) − 1`; `&self` on two methods that
never mutate; `Option<Ordering>` by value for a one-byte `Copy`; elided
lifetimes at seven sites; digit separators on four long literals. Legibility
and consistency (Principle 5's code-organization doctrine); `&mut self` on a
read-only method misdocuments its effect.

Evidence:

        46	    pub(crate) fn bit(&self, i: u64) -> bool {
        47	        // A bit index past `usize` can only address zeros: the value's own
        48	        // bit length always fits a `usize`.
        49	        usize::try_from(i).map(|i| self.0.bit(i)).unwrap_or(false)
        50	    }

    crates/before/src/version/rank.rs
       627	    let biased = num.clone().shr(exp).plus_one();
       628	    let w = biased.bits();
       629	    let rho = u64::from(63 - w.leading_zeros());
    crates/before/src/version/skyline/watermark.rs
      1160	    pub(super) fn bridge_add_gap(&mut self, delta: &mut Accumulator) {
      1161	        delta.add_accum(&self.gap);
      1162	    }
    crates/before/src/meter/board/floors.rs
       751	pub(super) fn masked_cmp_floors(
       752	    verdict: &Option<Ordering>,
    crates/before/src/version/skyline/build.rs
       138	            && self.path.last().map(|bit| !bit).unwrap_or(false)

Resolution: one mechanical commit: `usize::try_from(i).is_ok_and(|i| self.0.bit(i))`;
`self.path.last().is_some_and(|bit| !bit)`; `let rho = u64::from(w.ilog2());`;
`&self` at watermark.rs:1160 and 1171; `verdict: Option<Ordering>` at
floors.rs:752 (and the caller); `'_` at the seven lifetime sites; digit
separators at span/tests.rs:662 and emit_probe.rs:25, 147, 163; `From`
conversions at the 26 `cast_lossless` sites; drop the clone at grow.rs:654.
`cargo clippy --fix` with these lints enabled applies most of it. Acceptance:
`just gate` clean; the named lints report zero hits under a re-run of the
sweep's command.

### clippy-pedantic-8: Scan floors route an integer byte count through `f64` to multiply by a constant that is `1.0`, while the sibling floors are integers
- Where: crates/before/src/meter/board/floors.rs:377-380 (related: crates/before/src/meter/board/floors.rs:394, crates/before/src/meter/board/floors.rs:419, crates/before/src/meter/board/floors.rs:526, crates/before/src/meter/board/floors.rs:566, crates/before/src/meter/board/floors.rs:788, crates/before/src/meter/board/ceilings.rs:135, crates/before/src/meter/board/ceilings.rs:139, crates/before/src/meter/board/ceilings.rs:145, crates/before/src/meter/board.rs:287)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (grepped every use of `SCAN_FLOOR_BITS_PER_INPUT_BYTE` in the workspace: the definition, two import lines, five floor sites, nothing outside before/src; read the sibling floor constants and the tick-walk floor site); executed: no
- Verification: reframed and strengthened: the sweep read the constant against the `f64` ceilings; the relevant siblings are the other floors, `SCAN_TOUCH_FLOOR_BITS: u64 = 2` and `TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE: u64 = 8`, and floors.rs:788 already applies the latter as `(packed_bytes as u64).saturating_mul(...)`. The `f64` scan floor is the one floor spelled as a ratio; history: no-rationale-found
- Owner-gated: yes (the constant is `pub` on the feature-gated meter surface via board.rs:287; no reader outside before/src exists, but a `pub const`'s type is API under the stable-API rule)

`SCAN_FLOOR_BITS_PER_INPUT_BYTE` is `f64 = 1.0`, used at exactly five floor
sites as `(bytes as f64 * C) as u64`, producing fifteen cast hits for an
integer identity. Instruments before cures: a floor is the meter's liveness
proof and should be exact by construction; the `f64` detour is exact only
because `1.0` is exact and byte counts stay below 2^53, an argument the integer
form in the same file does not need.

Evidence:

       377	        scan: Liveness::Floor {
       378	            min: (fed_bytes as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64,
       379	            why,
       380	        },

    crates/before/src/meter/board/ceilings.rs
       135	pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;
       139	pub const SCAN_TOUCH_FLOOR_BITS: u64 = 2;
       145	pub(super) const TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE: u64 = 8;

    crates/before/src/meter/board/floors.rs (the integer house form in the same file)
       788	            min: (packed_bytes as u64).saturating_mul(TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE),

Resolution (suggestion): make the constant `u64 = 1` (derivation comment
unchanged) and write `min: (fed_bytes as u64).saturating_mul(SCAN_FLOOR_BITS_PER_INPUT_BYTE)`
at floors.rs:378, 394, 419, 526, 566, matching line 788. The `pub use` at
board.rs:287 carries the new type unchanged. Acceptance: the fifteen cast hits
at those lines vanish under a re-run of the sweep's command, and every floor
reading in the board is byte-identical to the parent commit's.

## Positives

- Every production-path narrowing cast the sweep read is bounded in the same
  function by a named constant or an explicit early return (`n <=
  SMALL_CODE_BITS` at build.rs:96-100 read here; the remainder sweep-reported).
  None is reachable from decoded bytes or caller input with a lossy result.
- `Shl<u64>` / `Shr<u64>` for `Base` (codec/base.rs:466-500) are the model for
  a width conversion in this crate: `usize::try_from(..).expect(..)` on the side
  that cannot be total, a value-preserving clamp on the side that can, and the
  totality argument written at the site (read here).
- `PackedBuilder::read_bits` (build.rs:287) checks its untruncated range at
  entry; the pattern finding 2 asks for already exists one function above.
- The `Open` token (party/ops/build.rs:36-40) pairs `!Clone` with a
  `#[must_use]` reason string and states, at the declaration, what the borrow
  checker thereby prevents (read here).
- `tick_walk_floors` (floors.rs:788) states an integer floor as an integer
  multiply with saturation; finding 8 only asks the scan floor to match it.
- `prescan.rs:439` already takes `&Signed`; finding 3 aligns its sibling.
- Sweep-reported, not re-verified here: the `suspicious_operation_groupings`
  and `nonminimal_bool` hits in laws.rs are false positives on symmetric law
  predicates written in the law's own words; worst.rs:133-137 documents why
  exact `f64` equality is correct before the two `float_cmp` sites; default
  clippy passes under `-D warnings` with 28 single-site `#[allow]`s.

## Open questions for Finch

1. Is the feature-gated `pub mod meter` (lib.rs:438-439) held to the stable-API
   rule? The `encoded_bits` re-homing commit (05d87e1b) treats the meter feature
   as "the instrument surface" distinct from "the unconditional public API".
   Finding 8's constant-type change is owner-gated only if the answer is yes;
   the answer also settles how future sweeps classify meter-surface changes.
2. Finding 4: do you want the type-level `#[must_use]` on `Party` and `Clock`
   (which also fires on a discarded `dangerously_alias()` and on any
   statement-position constructor), or only the method-level attributes on
   `fork` and the pure combinators? No site in before, suanpan, or rumors
   trips either today.
3. Finding 1: should unsuffixed-literal shifts on `Base` keep compiling
   (keep the impl, make its guard total), or should the two test spellings be
   suffixed and the impl deleted? The second is the smaller tree.

## Dropped

- Narrowed, not dropped: finding 2's claim that `read_bits` shares the
  truncated-check pattern. Its `debug_assert!` at build.rs:287 checks the
  untruncated `pos + u64::from(n) <= self.len()`; only `bit_at` shares the shape.
- Narrowed, not dropped: finding 3's "a wide magnitude clone is a big-integer
  allocation" generalized to every consumed site. `Int` is `Small(u64) |
  Wide(Base)`; the allocation occurs only for wide magnitudes.
- Narrowed, not dropped: finding 5's "6 in public rustdoc". party/tests.rs:629
  is test code and place.rs:51 is a private module doc; four before sites and
  two suanpan sites are public.
- Narrowed, not dropped: finding 8's framing against the `f64` ceilings. The
  sibling floors are `u64` and the integer form exists at floors.rs:788; the
  finding is stronger than the sweep stated, and owner-gated because the
  constant is `pub`.
- The sweep's "not adopted" lint groups (use_self, redundant_pub_crate,
  missing_const_for_fn, cast_precision_loss, option_if_let_else, and the rest
  of its open-questions list) were not re-litigated here; none is a finding,
  and the sweep's reasons stand as written.
