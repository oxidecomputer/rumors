# Partition party: Party: the id tree, its packed-bit operations (build, compare, diff, index, split, sum), forks, and idbits

## Partition summary

The partition is the identity half of `before`: `Party` (a nonempty dyadic share of `[0, 1)` stored as a canonical 2-bit-per-node preorder stream), the consuming cursor `IdReader` over that coding (`idbits.rs`), and the kernels that operate on it directly: `split` (a positional spine walk plus verbatim splices), `sum` (a two-cursor lockstep merge writing final tags and collapsing `(1, 1)` by fixed-width truncation), `covers`/`is_disjoint` (one shared predicate walk, `lockstep_holds`, parametrized by the single node that settles a pair), `diff` (a boolean-skyline sweep over the overlay law with covered-block early exits), `sum_split` (the fused join-then-fork behind `Clock::sync`), and `IdIndex` (a per-fold random-access table over one fixed operand, so `join_all` does not re-walk the accumulator per input). `forks.rs` layers the balanced k-way split and its residual-keeping iterator on `fork`. Every walk is iterative, with depth priced in bits, and the differential suite in `party/tests.rs` holds each kernel to the recursive oracle, to constructed deep witnesses at 10 000 and 100 000 levels, and (under `scan-meter`) to derived floors and ceilings.

The kernels are correct as far as reading establishes, and the maintainer-facing argument is unusually complete: `sum_split`'s fusion proof, `Lockstep`'s empty-stack-on-unary-chains invariant, `diff`'s covered-block taxonomy, `PosStack`'s delta coding, and `IdIndex`'s stated trade are each written where the code lives, and every `unreachable!`/`expect` reads as a one-line proof. The known-bad oracle in `join_all_differential_convicts_the_dropped_group_oracle` and the exact two-sided fork orbit pins are the adequacy and shape-over-point instruments the doctrine asks for. The dominant issues are not in the mechanisms but around them: a cluster of ghost references that breach the tree's hard rule (`IdLit`, `EvNode`, a removed `compare` op, a nonexistent oracle note, a "recursive form" label on a loop); public claims contradicted by the code or by the crate's own pins ("no allocation" on `covers`/`is_disjoint`, "nothing allocates" on `shape`, "strictly smaller than the operand" on the index table, `O(n)` on the tuple-literal door, "exactly `k`" shares at `u64::MAX`, the `join_all` `# Errors` contract, the fold index's `B` term); one headline promise with no instrument (`forks`' minimal-depth balance); a 32-bit `ExactSizeIterator::len` panic reachable from caller input; and a fold-index cliff past 2^32 packed bits that the public bound does not carry. The structural remainder is fixed-sign and small: the 2-bit tag decode and subtree skip hand-spelled at six sites, three walks stacking bits on the output buffer type where `BitStack` exists, a redundant right-child scan in `split`, and the `sum_split` spine accumulator duplicating input bits.

Files read in full with line numbers: `party.rs` (913), `party/forks.rs` (206), `party/ops.rs` (50), `party/ops/build.rs` (370), `compare.rs` (190), `diff.rs` (438), `index.rs` (291), `split.rs` (119), `sum_split.rs` (223), `sum.rs` (194), `party/tests.rs` (1162), `idbits.rs` (218): 4374 lines, of which `party/tests.rs` is the one test file (there are no other `tests.rs` siblings in the partition; the other files hold no inline test blocks). Supporting files read in part: `version/skyline/overlay.rs`, `codec/{stack,scan,buf,build,literal}.rs`, `tests/meter.rs`, `tests/forks_max.rs`, `before-fuelscape/src/ops.rs`, `laws.rs`, `fold.rs`, `clock.rs`, `clock/forks.rs`, `meter/board/{ops,floors,coverage}.rs`, `fuzzfit/harness/src/bands.rs`, `testing/{generators,exhaustive}.rs`, `version/skyline/{shape,grow,fill}.rs`, and the fold-unification survey in `.agent-notes`. No cargo, just, or test command was run; every "verified" item below is a grep, `git show`, or read fact, and every complexity derivation is a hand trace of the code.

## Findings

### party-1: `covers`/`is_disjoint` contracts say "no allocation"; the walk allocates and the crate's own envelopes pin the heap
- Where: crates/before-fuelscape/src/ops.rs:821-821 (related: crates/before-fuelscape/src/ops.rs:835, crates/before/src/party.rs:386, crates/before/src/party.rs:410, crates/before/src/party/ops/compare.rs:113-115, crates/before/src/party/ops/compare.rs:162-165, crates/before/tests/meter.rs:274-275, crates/before/src/codec/stack.rs:47-56)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read the contract strings, the `Lockstep::pending` field and its `push` sites, the `ID_COVERS`/`ID_DISJOINT` envelope constants, and `envelope()`'s first parameter being `peak_heap`); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (da0f6a937 pinned heap 8 on both rows; 7e63c1b96 and c6d8106a1 later wrote "no allocation" without reconciling)
- Owner-gated: no

The public `# Complexity` islands rendered into `Party::is_disjoint` and `Party::covers` promise no allocation, but `lockstep_holds` queues two bits per both-present ancestor pair on a `BitsBuf` (a heap-backed byte vector), and the committed envelopes for both operations pin a nonzero peak heap. A public space claim is a hard guarantee per the crate docs ("any violation is a bug"); this one is contradicted by the crate's own committed measurement. Moving the stack to `BitStack` (party-19) does not restore the claim: the register spills a word every 64 bits (stack.rs:49-53), so the honest statement is a bound, not an absence.

Evidence:

       821	        contract: "`O(|self| + |other|)`, no allocation",
       835	        contract: "`O(|self| + |other|)`, no allocation",

       113	struct Lockstep {
       114	    /// Two presence bits per queued right child pair, innermost on top.
       115	    pending: BitsBuf,

       274	    pub const ID_COVERS: Envelope       = envelope(        10,        0,             0, 0); // iterative id walks
       275	    pub const ID_DISJOINT: Envelope     = envelope(        10,        0,             0, 0); // iterative id walks

Resolution: Restate both contracts as "`O(|self| + |other|)`; transient state is two bits per queued ancestor pair" (the same wording fits the board's `party_disjoint`/`party_covers` heap floors, which already treat heap as structurally near-zero rather than absent). Acceptance: the rendered `# Complexity` text on `Party::covers` and `Party::is_disjoint` no longer contradicts the `ID_COVERS`/`ID_DISJOINT` heap pins; no envelope or band moves.
Construction: the `IdSpine` divert pair already used by `id_covers_envelope`/`id_disjoint_envelope` (tests/meter.rs:6135-6161): one both-present node queues one right pair, allocating the `BitsBuf`'s byte vector; the pinned ceiling of 10 bytes is the measured 8 × 1.25. Any pair with a both-present node reaches the `push` at compare.rs:163-164.

### party-2: Hand-maintained operation and caller rosters have drifted
- Where: crates/before/src/idbits.rs:1-3 (related: crates/before/src/idbits.rs:66-70, crates/before/src/idbits.rs:129-131, crates/before/src/idbits.rs:175-176, crates/before/src/idbits.rs:207-209, crates/before/src/party/ops.rs:1-2, crates/before/src/party/ops.rs:6-7, crates/before/src/party/tests.rs:1-2)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn skip_subtree` shows callers in idbits.rs, diff.rs:368/371, grow.rs:301, meter/board/family.rs:1199; `grep -rn 'IdReader::at('` shows sum_split.rs:176/198-199, split.rs:116, fill.rs:524; `peek` callers at sum.rs:38-39, sum_split.rs:80, split.rs:22, build.rs:89; `bits()` callers at split.rs:26, sum_split.rs:114-115/172, build.rs:94); executed: no
- Seen by: prose, claims (the `grow`-only sentence), structure (item 5), refutation (three new sites); refutation: confirmed; history: contradicts doctrine (drift traceable: 7139904bc, a99e8c8f7, e5151f9bd, c7c9d3e8f, 33d37fb0f, 29d3c8f27, 522705cff)
- Owner-gated: no

Seven module- and item-level docs enumerate the operations or callers that use a thing, and each list is now incomplete or wrong: the idbits roster omits `diff`/`covers`/`sum_split`; `IdReader::peek`'s doc names `fill` as its user while four party kernels peek; `IdReader::bits`'s doc names `sum`/`diff` capacity hints while `split`, `sum_split`, and `copy_reader` call it for splice ranges; "The only operation that reads a tree twice is `grow`" omits `fill`'s memoized pre-scan, `IdIndex::build`'s two passes, and the two `IdReader::at` probes in `split`/`sum_split`; `skip_subtree`'s doc names two runners while `diff::consume` and the board's family generator also call it; ops.rs promises `O(n + m)` for "Every operation" while its own `index` submodule documents an extra `B log n` term, and lists two predicates that emit nothing under a "mutate by re-emission" rationale; tests.rs:1-2 says the tests are "all differential against the oracle" while the file holds orbit pins, scan floors, and constructed byte-checks. Doctrine: no hand-maintained enumerations of module contents or callers; state the structure, not the tally.

Evidence:

         1	//! A read-only cursor over the packed id encoding, shared by the party
         2	//! operations (`split`/`sum`/`is_disjoint`/`compare`) and the event operations
         3	//! (`fill`/`grow` walk the packed id alongside the working event tree).

        66	/// Not `Copy` or `Clone`: a cursor is single-use. Advancing it consumes the
        67	/// stream, so a stale or duplicated cursor (a re-scan, which would break the
        68	/// `O(n + m)` bound) cannot be formed by accident. The only operation that
        69	/// reads a tree twice is `grow` (on the event side), which rebuilds a fresh
        70	/// cursor from the source per pass.

       207	/// The single shared spelling of this scan: [`IdReader::skip`] runs it on the
       208	/// packed id encoding, and the skyline `grow` walks run it to skip event
       209	/// subtrees (one topology flag plus one skipped payload code per node).

         6	//! the *absence* of a child, never a node. Every operation is `O(n + m)` in its
         7	//! inputs, with no re-scan to find a right child, and none recurses — a deep

Resolution: Replace each roster with the mechanism it stands for: idbits.rs:1-3 "shared by the party operations in `party::ops` and by the event-side `fill`/`grow` walks"; idbits.rs:66-70 "a second cursor over a range is formed only deliberately, by `IdReader::at` from a recorded position, and each such site bounds its re-read where it lives"; idbits.rs:129-131 "a look at the current node, leaving the cursor in place"; idbits.rs:175-176 "for capacity hints and verbatim splice ranges"; idbits.rs:207-209 "every packed-tree skip in the crate routes through it"; ops.rs:6 "Every cursor operation is `O(n + m)` in its inputs (`index` documents the fold-only search term)"; ops.rs:1-2 drop the operation list; tests.rs:1-2 name the categories without "all". Acceptance: no module or item doc in the partition enumerates a caller or operation set that grep shows to be incomplete; the `compare` ghost (party-3) is gone from the same lines.

### party-3: Prose refers to code that no longer exists: `EvNode`, `IdLit`, a removed `compare` op, an absent oracle note, a former encoding, and a "recursive form" on a loop
- Where: crates/before/src/idbits.rs:32-36 (related: crates/before/src/idbits.rs:2, crates/before/src/idbits.rs:61-64, crates/before/src/party.rs:802-809, crates/before/src/party/ops.rs:1-2, crates/before/src/party/tests.rs:368, crates/before/src/party/ops/diff.rs:63-66, crates/before/src/party/ops/split.rs:19)
- Class / severity / confidence: documentation / high / high
- Provenance: verified (`grep -rn EvNode crates/before/src` returns idbits.rs:35 only, and `git grep -c EvNode 782064269^` shows the grow.rs and grow/tests.rs occurrences that 782064269 removed; `grep -rn IdLit crates/` returns party.rs:806 only, the trait is `PartyLiteral` at party.rs:817, and the `///` block at 802-808 is attached to `mod sealed` at 809; `git log -S'fn compare' -- crates/before/src/party crates/before/src/idbits.rs` names 7139904bc; `grep -rn BitAnd crates/before/src` finds no prose note in the oracle, only `impl BitAnd<Version> for Version` at oracle/version.rs:454; split.rs:45-53 is a loop); executed: no
- Seen by: prose, structure, correctness (nit), claims; refutation: confirmed (its item 3, the `IdReader::at` pointer, is disputed and dropped); history: contradicts hard rule (each expiry commit named above)
- Owner-gated: no

Six sites breach the root and crate AGENTS.md hard rule "Nothing in the codebase refers to code that no longer exists": an event-side type `EvNode` (removed 782064269), a trait `IdLit` (the trait is `PartyLiteral`, and the doc block describing it sits on `mod sealed`, so it renders on the wrong item), an id operation `compare` listed twice and used as `compare == None` (removed 7139904bc), a pointer to "the note on the absent `BitAnd for Clock`" in an oracle module that contains no such note, a past-tense description of the encoding the pruned form replaced ("as they did when `0` was a real leaf in the stream"), and `split` described as "The recursive form" of the oracle when `build_split` has been a loop since 32a655438 (the sibling docs at sum.rs:17 and compare.rs:11 say "The cursor form"). Each sends a maintainer to a name or place that is not there.

Evidence:

        32	/// A decoded id node: the empty `0` leaf, the full `1` leaf, or an internal
        33	/// node tagged with which of its children are present.
        34	///
        35	/// The id-side analogue of the event side's `EvNode` — the clean shape the
        36	/// operations match on (the paper's id grammar `i ::= 0 | 1 | (i1, i2)`).

       806	/// literals. Unlike the public `TryFrom`, an `IdLit` leaf of `0` is allowed (it
       807	/// is a valid *sub-tree*); the anonymous check happens only once the whole id
       808	/// is assembled (see [`finish_id`]).
       809	mod sealed {

       368	// the overlap arms (`compare == None`, `sum == None`, `join == Err`) that the

        65	    /// it linearity-safe where a general id *meet* is not (see the note on the
        66	    /// absent `BitAnd for Clock` in [`oracle`](crate::oracle)): carving a

        64	///   exactly as they did when `0` was a real leaf in the stream.

        19	    /// The recursive form of `oracle::Party::split` (the paper's `split`).

Resolution: idbits.rs:35 drop the `EvNode` clause or re-state positively ("the shape the id walks match on"); move the party.rs:802-808 block onto `pub trait PartyLiteral` and write `PartyLiteral` (or "a literal leaf") for `IdLit`; delete `compare` from ops.rs:2 and idbits.rs:2 (party-2 rewrites those lines anyway) and rewrite tests.rs:368 over the ops that exist (`is_disjoint == false`, `sum == None`, `join == Err`); at diff.rs:65-66 either state the linearity note inline (the `diff` method doc is its natural home: a general meet can synthesize a region shared with a third live party, a difference cannot) or drop the pointer; idbits.rs:64 "exactly as they would if `0` were stored as a leaf"; split.rs:19 "The cursor form of `oracle::Party::split` (the paper's `split`)". Acceptance: `grep -rn 'EvNode\|IdLit\|compare\b' crates/before/src/party crates/before/src/idbits.rs` returns no prose hits; the private-items rustdoc shows the literal-door text on `PartyLiteral`; diff.rs:65-66 links to text that exists; `grep -n 'recursive form' crates/before/src/party/ops/split.rs` is empty.

### party-4: The 2-bit tag decode and the id subtree skip are hand-spelled at six sites
- Where: crates/before/src/idbits.rs:145-156 (related: crates/before/src/idbits.rs:100-109, crates/before/src/party/ops/diff.rs:312-314, crates/before/src/party/ops/diff.rs:352-372, crates/before/src/party/ops/index.rs:200-201, crates/before/src/party/ops/split.rs:47-53, crates/before/src/party/ops/split.rs:115-119, crates/before/src/party/ops/sum_split.rs:176-178, crates/before/src/version/skyline/grow.rs:290-306)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read every site; `grep -rn skip_subtree` and `grep -rn 'IdReader::at('` enumerate the callers); executed: no
- Seen by: structure, prose (the `record_bits` spelling); refutation: confirmed; history: no rationale found (the sites accreted across 32a655438, 183e3cab7, 13d8c398, a99e8c8f7)
- Owner-gated: no

The id header probe for `skip_subtree` (record 2 bits, count the two tag bits, advance 2) is written out in `IdReader::skip`, verbatim again in `diff::consume`, and a third time in `grow::id_skip`; `split::subtree_end` and `sum_split::branch_children` each construct a throwaway `IdReader::at` only to call `skip` and read `pos`. The raw tag read `(bits.bit(p), bits.bit(p + 1))` appears in `IdReader::tag`, `diff::enter`, `diff::consume`, `IdIndex::is_disjoint`, `build_split`, and `grow::id_tag`, each with its own `record_bits(2)` line or none. idbits.rs:207-209 calls `skip_subtree` "The single shared spelling of this scan", but what is shared is the generic counter; the id-specific probe is re-derived per caller, and the meter hook rides in each copy, which is how `build_split`'s spine read (party-27) came to be the one tag read that records nothing.

Evidence:

       145	    pub(crate) fn skip(&mut self) {
       146	        if let IdReader::At { bits, pos } = self {
       147	            let bits = *bits;
       148	            *pos = skip_subtree(*pos, |at| {
       149	                // One 2-bit tag scanned per skip step. Children present =
       150	                // the two tag bits; the tag is 2 bits wide.
       151	                crate::codec::scan::record_bits(2);
       152	                let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
       153	                (children, at + 2)
       154	            });
       155	        }
       156	    }

       359	        let bits = self.bits;
       360	        let scan = |at: u64| {
       361	            // One 2-bit tag scanned per skipped node. Children present = the
       362	            // two tag bits; the tag is 2 bits wide.
       363	            crate::codec::scan::record_bits(2);
       364	            let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
       365	            (children, at + 2)
       366	        };

Resolution: In `idbits`, add two `pub(crate)` free functions and route every site through them: `tag(bits, pos) -> IdNode` (the existing private `IdReader::tag`, recording its own 2 bits) and `subtree_end(bits, at) -> u64` (the id instantiation of `skip_subtree`, recording per step). `IdReader::{read, peek, skip}` call them; `diff::{enter, consume}`, `IdIndex::is_disjoint`, `split::subtree_end`, `sum_split::branch_children`, and `grow::{id_tag, id_skip}` become one-line calls. `build_split`'s spine loop is the one site whose routing changes a committed reading; take it in the same commit as party-27's re-pin or leave it raw and state the exemption there. Acceptance: `grep -n '\.bit(' crates/before/src/idbits.rs crates/before/src/party/ops/*.rs crates/before/src/version/skyline/grow.rs` shows tag-bit reads only inside the two shared helpers (plus `build.rs:257`'s debug assert if kept); every scan-meter test and board scan pin reads an identical number except where the spine newly records.

### party-5: Register and vocabulary sweep items: `mint`, em-dashes in `//` comments, `honest`, `tripwire`, a `d_` prefix, a dependent first sentence
- Where: crates/before/src/party.rs:13-17 (related: crates/before/src/party.rs:872, crates/before/src/party/ops/sum_split.rs:166, crates/before/src/party/tests.rs:1042, crates/before/src/party/tests.rs:30-32, crates/before/src/party/tests.rs:323, crates/before/src/party/tests.rs:430, crates/before/src/party/tests.rs:584, crates/before/src/party/tests.rs:993; em-dash `//` sites: party.rs:337, 344, 398, 399, 635, 638; forks.rs:51, 108, 109, 193; diff.rs:83, 84, 118, 119; index.rs:182, 187; compare.rs:19, 37; tests.rs:69, 70, 74, 365, 366, 1040)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn -i '\bmint'` over the partition returns exactly the four sites; `grep -n '^\s*//[^/!].*—'` over the twelve files returns exactly the 24 sites listed; `grep -n -i 'honest\|tripwire' tests.rs` returns lines 32, 61, 121, 177, 181, 251, 259, 279, 430, 584, 746; `grep -rn 'fn d_' crates/before/src` returns tests.rs:323 alone); executed: no
- Seen by: structure, prose; refutation: confirmed; history: contradicts writing-style rules (~/.claude/writing-style.md:149-151, 170, 326-330, 406) that postdate every site; crate-wide counts: `mint` 58, em-dash-in-`//` 374, `honest` 159, `tripwire` 75
- Owner-gated: no

Four uses of `mint` for constructing a value (the one banned word); 24 `//` comments carrying true em-dashes where the register rule wants ` -- `; `honest` eight times for "non-duplicated" or "defect-free" and `tripwire` twice (430, 584) for tests no committed known-bad fails (177 uses it correctly for the `surface_coverage` roster); a lone `d_` test-name prefix whose only sibling migrated to `laws.rs` at 86dd53a71; and a test doc whose first sentence ("The invariant holds for both halves ...") depends on the previous test's doc. These are instances of crate-wide sweeps, listed here so the partition's sites are in one place; the `mint` rule is the one with a stated ban.

Evidence:

        14	//! documented escape hatches: the serialization and text/literal doors, which
        15	//! mint a second holder from bytes or notation, and

       872	/// Mints identity exactly as the `u8` literal door does — a test and

       166	#[allow(clippy::type_complexity)] // two optional bit ranges: an inline pair over a minted name

        30	/// Aliased inputs stay best-effort: a duplicated share collides on its way in
        31	/// and is handed back whole (nothing panics, nothing is dropped), while the
        32	/// honest copy of every share still reunites the seed region.

       323	    fn d_fork_join_roundtrip(ops in world_strategy(), i in 0usize..64) {

       993	    /// The invariant holds for both halves produced by `fork` (the split path),

Resolution: party.rs:15 "create a second holder", party.rs:872 "Creates identity exactly as", sum_split.rs:166 "the inline pair reads better than a coined name" (party-6 removes the allow anyway), tests.rs:1042 "add more than its one tree level"; ` -- ` at the 24 `//` sites; "the original" / "the defect-free transcription" / "the sublinear arm" for `honest`; "The deterministic companion to" (430) and "witnesses and floors" (584) for `tripwire`; rename `d_fork_join_roundtrip` to `fork_join_roundtrip_matches_oracle`; tests.rs:993 "`as_bytes` equals `encode` for both halves produced by `fork`". Acceptance: `grep -rni '\bmint' crates/before/src/party.rs crates/before/src/party/ crates/before/src/idbits.rs` empty; `grep -n '^\s*//[^/!].*—'` over the partition empty; every `tripwire` in tests.rs names a committed known-bad artifact.

### party-6: Idiom nits: `from_frozen` bypassed by `decode`/`seed`, qualified `core::fmt`/`crate::`/`std::io` paths beside a once-used import, `then(|| ..)` and positional `(u64, u64)` ranges under a `type_complexity` allow
- Where: crates/before/src/party.rs:19-19 (related: crates/before/src/party.rs:95-96, crates/before/src/party.rs:131-133, crates/before/src/party.rs:353, crates/before/src/party.rs:500-501, crates/before/src/party.rs:572, crates/before/src/party.rs:623, crates/before/src/party.rs:640, crates/before/src/party.rs:724-732, crates/before/src/party.rs:750-759, crates/before/src/party.rs:784, crates/before/src/party/ops/index.rs:219, crates/before/src/party/ops/sum_split.rs:133-139, crates/before/src/party/ops/sum_split.rs:166-171, crates/before/src/version.rs:1124)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn from_frozen` shows `Party::from_frozen`'s only callers are clock.rs:820 and borsh_impls.rs:164 while version.rs:132/1124 route `Version`'s seed and decode through `Version::from_frozen`; grep counts across before/src show 5 `impl core::fmt::` headers against 12 imported-name impls, so no crate convention exists); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (4874f527b added `from_frozen` and left `decode`/`seed` direct in the same commit; 5d167a63's allow rationale argues against coining a name, which `Range<u64>` from std satisfies)
- Owner-gated: no

Three legibility items in one class. `from_frozen` is documented as "the decode-side gate" yet `Party::decode` and `Party::seed` construct the tuple directly, so a grep for the gate misses the primary decode door while the `Version` side spells it consistently. `Display` is imported and used once while `core::fmt::{Display, Debug, Formatter, Result}`, `core::hash::{Hash, Hasher}`, `core::str::FromStr`, `std::io::{Read, Write}`, `crate::shape::Regions`, and `crate::fold::balanced_try_fold` are spelled in full at their use sites (doctrine: imports over long paths except where the qualification informs, which `fmt::Result` does). `al.then(|| p + 2)` wraps a trivial expression in a closure where `then_some(p + 2)` is the idiom (clippy does not flag it because it treats `p + 2` as possibly panicking, so this is legibility, not a lint fix), and `branch_children`/`union_child`/`UnionChild::Verbatim` carry bit ranges as `(u64, u64)` pairs under a `type_complexity` allow where `Range<u64>` names the thing and removes the allow (`split.rs:54` already writes `start..prefix_end`).

Evidence:

        19	use core::fmt::Display;

       750	impl core::fmt::Display for Party {
       751	    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {

       640	        Ok(Party(codec::Bits::from_canonical(buf.into())))

       219	                            let a_left = al.then(|| p + 2);

       166	#[allow(clippy::type_complexity)] // two optional bit ranges: an inline pair over a minted name
       167	fn branch_children(
       168	    reader: &IdReader<'_>,
       169	    left: bool,
       170	    right: bool,
       171	) -> (Option<(u64, u64)>, Option<(u64, u64)>) {

Resolution: `Ok(Party::from_frozen(codec::Bits::from_canonical(buf.into())))` at 640 and `Party::from_frozen(...)` at 131; `use core::fmt::{self, Debug, Display, Formatter}; use core::hash::{Hash, Hasher}; use core::str::FromStr; use std::io::{Read, Write}; use crate::fold::balanced_try_fold; use crate::shape::Regions;` and shorten the sites, keeping `fmt::Result`; `al.then_some(p + 2)`; `Option<Range<u64>>` in `branch_children`/`union_child`, `Verbatim(BitsView<'a>, Range<u64>)`, and delete the allow. Acceptance: `grep -n 'Party(codec::Bits' crates/before/src/party.rs` returns only `dangerously_alias`'s clone; `grep -c 'core::fmt::' crates/before/src/party.rs` is 0; no `type_complexity` allow in sum_split.rs; `just clippy` clean.

### party-7: Public rustdoc slips in `party.rs`: `n` in prose against `k` in signatures, a missing period, "logarithmic factor" for an additive term, and a `# Warning` that is `Clock::decode`'s text verbatim
- Where: crates/before/src/party.rs:184-184 (related: crates/before/src/party.rs:206, crates/before/src/party.rs:239-245, crates/before/src/party.rs:274, crates/before/src/party.rs:605-610, crates/before/src/clock.rs:166, crates/before/src/clock.rs:192, crates/before/src/clock.rs:768-772, crates/before/src/party/forks.rs:16-17, crates/before-fuelscape/src/ops.rs:772)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the six lines against clock.rs:768-772, which the warning matches word for word; `git show 2efff1498` confirms the `n -> k` parameter rename with its rationale "counts renamed to k so n means bytes alone"; `git show e546b6d5e:crates/before/src/party.rs` shows the earlier Party-specific warning text that b3f09baa0 replaced); executed: no
- Seen by: prose, claims ("logarithmic factor"); refutation: confirmed; history: the `k` side is deliberate (2efff1498), the prose `n` is the un-renamed remainder; the `Clock` warning replaced a Party-specific one in the owner's doc pass (b3f09baa0) with no message
- Owner-gated: no

Four public-doc accuracy items on one type. `ticks` and `forks` document `n` while their signatures take `k` (the rename to `k` is the deliberate side: "so `n` means bytes alone"); `ticks`' first sentence has no terminal period; `forks` says each share "increases in size by only a logarithmic factor" when the growth is an additive `2·⌈log₂(k+1)⌉` bits (forks.rs:16-17 and the island contract `k (|self| + log k)` both spell it additively); and `Party::decode`'s `# Warning` names `Clock` in every sentence and `Party` in none, being the clock.rs:770-772 text copied whole. The same `n`/`k` mismatch sits at clock.rs:166/192.

Evidence:

       184	    /// Advances `version` by `n` events for this [`Party`]

       206	    pub fn ticks(&self, version: &mut Version, k: impl Into<Ticks>) {

       242	    /// Unlike repeatedly calling [`fork`](Party::fork), which deepens its
       243	    /// representation into a biased linear tree (see its warning), every
       244	    /// resultant [`Party`] produced here increases in size by only a
       245	    /// logarithmic factor.

       607	    /// Serializing a [`Clock`](crate::Clock) circumvents its otherwise
       608	    /// compiler-enforced `!Clone` linearity. Deserializing one can violate
       609	    /// causality. Treat serialization/deserialization boundaries as *moves* of
       610	    /// the [`Clock`](crate::Clock).

Resolution: prose to `k` at party.rs:184, 239 and clock.rs:166 (do not rename the parameters back); add the period at 184; 242-245 "grows by only `O(log k)` bits over the party it was split from"; rewrite 605-610 in terms of `Party` ("Decoding creates a second holder of the share the bytes name, circumventing the compiler-enforced `!Clone` linearity; treat an encode/decode boundary as a *move* of the [`Party`], and the same for any [`Clock`](crate::Clock) built from it" — the e546b6d5e text is a usable reference). Acceptance: every `Party`/`Clock` doc names the parameter its signature declares; the `forks` sentence agrees with the island contract; the warning on `Party::decode` names `Party`.

### party-8: `join_all`'s `# Errors` contract promises the overlapping inputs back; the fold hands back coalesced unions
- Where: crates/before/src/party.rs:310-315 (related: crates/before/src/laws.rs:2402-2416, crates/before/src/fold.rs:31-36, crates/before/src/party/tests.rs:89-103)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read party.rs:312-315 against laws.rs:2406-2408 "the closing drain legitimately hands back *coalesced* groups, byte-distinct from every input", fold.rs:31-36's retention policy, and tests.rs:93-100's four-input hand-back `alias∪c∪d∪e`; `git show 4f12b8218:crates/before/src/party.rs` shows the earlier text carried the coalescing clause); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (b3f09baa0, the owner's doc pass, replaced the clause without a message)
- Owner-gated: no

The public sentence says the returned parties are "the parties which *overlapped*" and that "every input [`Party`] is either merged into `self` or handed back", but the balanced counter returns unions of several inputs, some of which overlapped nothing, whenever a coalesced group later fails a combine; the partition's own test and the laws roster both state this as correct behavior. A caller who treats each returned `Party` as one offending input (counting them, or retrying `join` with "the overlapping ones") has a wrong model. The true invariant is region conservation (`party_join_all_err_conserves_the_region_union`), and the prose should state it.

Evidence:

       310	    /// # Errors
       311	    ///
       312	    /// Returns the parties which *overlapped* and so could not be folded in,
       313	    /// dropping nothing: every input [`Party`] is either merged into `self` or
       314	    /// handed back. In case of partial error, the set of parties which are
       315	    /// absorbed vs. handed back is unspecified.

Resolution: Rewrite: on `Err`, `self` has absorbed some inputs (possibly none); the returned parties are unions of the remaining inputs, each containing at least one input that overlapped `self` or another input; no region is lost (`self` joined with the returned parties covers the original region plus every input's); which inputs are absorbed and how the rest are grouped is unspecified. Acceptance: the `# Errors` text describes union hand-back and region conservation, and `join_all_agrees_with_oracle_on_aliased_coalesced_group` reads as an instance of it rather than an exception.

### party-9: `Party::shape` says "nothing allocates"; the walk's path stacks spill a word every 64 levels, and nothing prices the drain
- Where: crates/before/src/party.rs:480-481 (related: crates/before/src/shape.rs:204-209, crates/before/src/version/skyline/shape.rs:74-87, crates/before/src/version/skyline/overlay.rs:471-485, crates/before/src/codec/stack.rs:47-56, crates/before/src/meter/board/coverage.rs:617-621)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read the chain `Regions::of_party` -> `PartyWalk::open` -> `overlay::IdLeafCursor` with `path: BitStack`/`right_present: BitStack`, and `BitStack::push`'s `self.words.push(self.top)` at 64 bits; `grep -rn 'party_shape\|Party::shape' crates/before/tests/meter.rs` is empty and coverage.rs:617-621 lists the operation unpriced with a rationale); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (46eb64f97's design note intends allocation-free draining; the word spill was not considered)
- Owner-gated: no

A public space claim is a hard guarantee per lib.rs:350-358. The `O(|self|)` drain is right and argued (each tag read once, each path bit pushed and popped once), but "nothing allocates" is false for any party deeper than 64 levels (a 64-deep fork chain is a plain construction), and no envelope, board cell, or band prices the drain. `Version::shape`'s neighbouring sentence (skyline/shape.rs:22-25 region) already states the accurate form for its own walk.

Evidence:

       480	    /// Draining the iterator is linear in the party's encoded size: each
       481	    /// region costs `O(1)`, and nothing allocates.

        48	    pub(crate) fn push(&mut self, bit: bool) {
        49	        if self.top_len == 64 {
        50	            self.words.push(self.top);

Resolution: Restate: "each region costs amortized `O(1)`; transient state is two bits per open ancestor (one machine word per 64 levels), nothing per region." If the owner wants the drain priced, a `tests/meter.rs` sweep row over the `IdSpine` party (scan = every tag once, heap = the two stacks' words) is a closed-form pin. Acceptance: the doc states the bit-per-level transient; if a row is added, its scan reading equals the party's packed bits and its heap reading equals `2·⌈depth/64⌉·8` bytes plus allocator slack.
Construction: `shape_party(Shape::LeftSpine, 200)` (testing/generators.rs) drained under the `PeakAlloc` meter used by `tests/meter.rs`: two `BitStack` spills of one word each (`path` and `right_present`), so peak heap is nonzero.

### party-10: Maintainer-facing "single gate"/"single point" overclaims and small slips in `party.rs`, `sum.rs`, and `compare.rs`
- Where: crates/before/src/party.rs:791-800 (related: crates/before/src/party.rs:45, crates/before/src/party.rs:233-237, crates/before/src/party.rs:298-306, crates/before/src/party.rs:453, crates/before/src/party.rs:458-465, crates/before/src/party.rs:646, crates/before/src/party.rs:673-676, crates/before/src/party.rs:712-714, crates/before/src/party/forks.rs:113, crates/before/src/party/ops/sum.rs:11-13, crates/before/src/party/ops/sum_split.rs:85-90, crates/before/src/party/ops/compare.rs:11-12, crates/before/reference/itc2008.md:182, crates/before/src/oracle/party.rs:145)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn finish_id` lists callers at party.rs:787, 866, 889, 911 only; `fork`, `join`, `sum_split` call `from_bits` directly and `without` re-spells `id_is_empty`; `grep -n -i disjoint reference/itc2008.md` returns one line, inside the invariant paragraph, and oracle/party.rs:145 defines `is_disjoint`); executed: no
- Seen by: structure, prose; refutation: confirmed; history: `finish_id`'s claim expired at f0b837336 (`without` inlined the check from birth); sum.rs:11 expired at c7c9d3e8f; `mem::swap` predates `forks` (14d36c62d used `mem::replace` from birth); compare.rs:11's paper attribution is a choice with no stated reason (517c7ebeb)
- Owner-gated: no

Six maintainer-facing statements are false or imprecise, and two of them are exactly the kind a reviewer relies on to skip auditing other paths. `finish_id` is "The single gate through which every parsed/built top-level `Party` passes", but only the text and literal doors call it; `fork`/`join`/`sum_split` freeze through `from_bits`, and `without` duplicates the emptiness test inline. `sum` is "the single point of overlap detection", but `sum_split` detects overlap on the spine itself (its own doc says so). `anonymous()` cites `mem::swap` while the one use is `mem::replace`. The operations table names `b` in the call and `q` in the meaning. The `without` example compares `to_string()`s where `Party: Eq + Debug` makes `assert_eq!` on values direct. `is_disjoint` is attributed to "the paper's region-disjointness test" when the paper states disjointness as an invariant (`∀i1 ≠ i2. i1 · i2 = 0`), not an operation, and the reference the siblings cite by name is `oracle::Party::is_disjoint`.

Evidence:

       791	/// Wrap validated id bits as a `Party`, rejecting the anonymous (empty)
       792	/// identity. The single gate through which every parsed/built top-level `Party`
       793	/// passes.

       459	        let bits = self.view().diff(other.view());
       460	        if codec::id_is_empty(codec::built_view(&bits)) {

        11	    /// This is the single point of overlap detection: callers (`Party::join`)

       646	    /// Internal and transient only (i.e. for use in `mem::swap`) and *never* a

        45	/// | [`p.is_disjoint(&b)`](Party::is_disjoint)              | whether `p` and `q` share no region, hence may safely interact            |

       453	/// assert_eq!(p.without(&q).unwrap().to_string(), keep.to_string());

        11	    /// The cursor form of the paper's region-disjointness test, on the shared
        12	    /// lockstep predicate walk ([`lockstep_holds`]).

Resolution: Extract `fn nonempty(bits: codec::BitsBuf) -> Option<Party>` (the emptiness gate plus `from_bits`); `finish_id` becomes `nonempty(bits).ok_or(Parse::Anonymous)` and `without` becomes `nonempty(self.view().diff(other.view()))`; describe it as the gate for every top-level `Party` built from possibly-empty bits (the kernels that prove non-emptiness structurally freeze through `from_bits`). sum.rs:11: "the point of overlap detection for `join`: a successful `sum` is the disjointness proof (`sum_split` detects it the same way on the spine)". party.rs:646 `mem::replace`; party.rs:45 `q` in both columns; party.rs:453 `assert_eq!(p.without(&q).unwrap(), keep);`; compare.rs:11 "The cursor form of `oracle::Party::is_disjoint` (the paper's disjointness invariant `i1 · i2 = 0`, as a test)". Acceptance: `grep -n 'id_is_empty' crates/before/src/party.rs` shows one site; `grep -n 'single gate\|single point'` over party.rs and sum.rs returns claims true of the code; every "form of" line in party/ops names the oracle function it mirrors.

### party-11: The tuple-literal door is `O(n·depth)`, not `O(n)`: every nesting level re-copies and re-validates its subtree
- Where: crates/before/src/party.rs:897-899 (related: crates/before/src/party.rs:838-844, crates/before/src/codec/literal.rs:52-66, crates/before/src/codec/text.rs:75-84, crates/before/src/party.rs:794-800)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read `id_node`: it copies both children into a fresh buffer and calls `validate_id` over the assembled subtree; the tuple impl recurses per level; the text door validates once at the end); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (per-level validation since eecf92295; the `O(n)` line came later in 3bba6cbbc)
- Owner-gated: yes: the one-pass fix changes the signature of the `#[doc(hidden)]` sealed `PartyLiteral::into_id_bits`; the doc-fix alternative is not gated

The `# Complexity` section claims `O(n)` in the built party's bytes, but a `d`-deep literal costs `Σ O(subtree)` copies and full re-parses, `O(n·d)`. The per-level `validate_id` is also a recompute-and-compare on a construction whose two local checks (`(0, 0)` and `(1, 1)`) are the whole normal-form rule, and children are normal by induction, so it catches nothing the local checks miss (the doctrine on runtime asserts over deterministic pure functions). The depth is fixed by the tuple type, so no runtime input controls it; the claim is still false as stated.

Evidence:

       897	/// # Complexity
       898	///
       899	/// `O(n)`, `n` the built party's size in bytes.

        59	    let mut b = BitsBuf::with_capacity(2 + l.len() + r.len());
        60	    b.push(!l.is_empty()); // bit 0 = left present
        61	    b.push(!r.is_empty()); // bit 1 = right present
        62	    b.extend_from_buf(l);
        63	    b.extend_from_buf(r);
        64	    validate_id(super::buf::built_view(&b))?;

Resolution: Delete the `validate_id` call in `id_node` (keep the two local collapse checks; validate once in `finish_id` if a belt is wanted) and either restate the bound as `O(n·d)`, `d` the literal's nesting depth, or thread one `IdBuilder` through `PartyLiteral::into_id_bits` (reserve/patch/close per level, one pass, truly `O(n)`). Acceptance: a left-nested literal of depth `d` performs one validation pass in total (scan-meter reading about `2·nodes` bits, not a sum over levels); the `# Complexity` section states the bound the code implements; `parse_bare_notation` and the text round-trip laws stay green.
Construction: under `--features scan-meter`, build `Party::try_from(((((1u8, 0u8), 0u8), 0u8), 0u8))` and its depth-8 and depth-16 extensions, reading `scan_bits()` around each: level `i` copies and re-parses about `2i` bits, so the reading grows about quadratically in `d` for a `2d`-bit result where an `O(n)` door reads linearly.

### party-12: `forks`' minimal-depth balance, the reason the operation exists, has no committed instrument
- Where: crates/before/src/party/forks.rs:16-17 (related: crates/before/src/party/forks.rs:57, crates/before/src/party/forks.rs:154, crates/before/src/laws.rs:2053, crates/before/src/laws.rs:2065, crates/before/src/laws.rs:2358, crates/before/tests/forks_max.rs:22-39, crates/before/fuzzfit/harness/src/bands.rs:568-579, crates/before-fuelscape/src/ops.rs:766-777, crates/before/src/meter/board/ops.rs:1270-1518)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep for `forks(` together with `encoded_bits|depth|log2|ilog2` over tests/, laws.rs, clock/tests.rs, testing/, fuzz, fuzzfit returns nothing; the laws touching `forks` are `forks_matches_from_array`, `forks_partial_drop_folds_back`, `party_join_all_reunites_forks_at_any_width` and their clock twins, none of which reads a depth; the board's `party_*` cells (ops.rs:1270-2250) include `party_fork` but no `party_forks`; the fuelscape island fixes the share count at 8; the `ff_party_forks` band is a fuel slope band with `width_above` 0.097); executed: no
- Seen by: claims; refutation: confirmed (the 3:1 known-bad's passage through the fuel band is assessed, not run); history: no rationale found (14d36c62d's laws hold for any partition; nothing defers a depth pin)
- Owner-gated: no

The public promise (every share a leaf of a minimal-depth `⌈log₂ k⌉` tree; forks.rs:5-7 calls the balanced split "the cure for the `fork` footgun") is asserted by no test, band, board cell, or envelope. Every committed check compares two forms that share `Split` or checks reunion, which hold for any disjoint tiling, so an unbalanced split passes the gate. Principle 2 and the crate's own rule that every asymptotic claim is a hard guarantee.

Evidence:

        16	/// A lazy balanced partition: yields a region's `k` shares one at a time, in
        17	/// preorder, each a leaf of a minimal-depth (`⌈log₂ k⌉`) id tree.

        55	        while count > 1 {
        56	            let right = region.fork();
        57	            let left_count = count.div_ceil(2);

Resolution: Add a closed-form pin in party/tests.rs: for `Party::seed()` and `k` over a ladder (1..=64, 1023, 1024, 1025, 2^16), every yielded share and the residual read `encoded_bits() == 2 + 2·d` with `d ∈ {⌊log₂(k+1)⌋, ⌈log₂(k+1)⌉}`, and the count of shares at the deeper level equals `2·(k+1) − 2^⌈log₂(k+1)⌉` (the complete-tree leaf split), so the whole tiling is fixed. Commit the 3:1 split as a known-bad witness (a `cfg(test)` `Split` variant, or an inline transcription of `Split::next` with `count - (count / 4).max(1)`) and assert the pin convicts it. Cite the pin by name at forks.rs:16-17 and :154. Acceptance: the new test fails when `left_count` is `count - (count / 4).max(1)` and passes on `count.div_ceil(2)`.
Construction: change forks.rs:57 to `let left_count = count - (count / 4).max(1);` (a 3:1 split, depth about `2.4·log₂ k`). `forks_matches_from_array` still holds (both forms run the same `Split`), the reunion and partial-drop laws hold (the shares remain a disjoint tiling), `party_forks_max_saturates_without_panic` terminates, and at the island's `k = 8` the extra work is roughly ×1.6 fuel, under the band's `width_above + ENFORCE_MARGIN` ceiling. Nothing committed fails.

### party-13: `Forks`/`Split` implement `ExactSizeIterator`, whose default `len()` panics for `k >= usize::MAX` shares on 32-bit targets
- Where: crates/before/src/party/forks.rs:65-76 (related: crates/before/src/party/forks.rs:132, crates/before/src/clock/forks.rs:57, crates/before/tests/forks_max.rs:26, crates/before/tests/forks_max.rs:49, crates/before/src/party.rs:239-240)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read `size_hint`; read std's default `ExactSizeIterator::len` in the pinned toolchain's `core/src/iter/traits/exact_size.rs:116-124`: `assert_eq!(upper, Some(lower))`; `grep -rn 'forks\|Forks' crates/before/wasm32-pins/` is empty; tests/forks_max.rs:26 and :49 carry `.expect("64-bit test host")`); executed: no (no 32-bit build permitted in this review)
- Seen by: structure; refutation: confirmed; history: no rationale found (cdad46060 widened `k` to `u64` and addressed only the `Iterator` face; the 32-bit `len()` was not examined)
- Owner-gated: yes: a public trait impl's contract
- Witness (witness/results.md, `## 32-bit pass`): demonstrated (run in the `wasm32-pins` executor; the main pass was inconclusive on the 64-bit host). `Party::seed().forks(1u64 << 32).len()` traps, `size_hint()` at that `k` has upper bound `None`, and `forks((1u64 << 32) - 1).len()` returns 4294967295 with no panic, so `k = 2^32` is the minimal panicking input and the panic is O(1) from caller input. One narrative correction: after the residual is drawn `remaining` is `2^32`, not the Construction's `2^32 + 1`; the mechanism and conclusion stand.

`size_hint` returns `(usize::MAX, None)` past `usize`, which is the standard `Iterator` spelling, but all three `impl ExactSizeIterator` blocks inherit std's default `len()`, which asserts `upper == Some(lower)`. On the crate's pinned `wasm32-unknown-unknown` target, `Party::seed().forks(u64::from(u32::MAX) + 1).len()` panics from caller input with no `# Panics` section. Correct at all scales, for all inputs: the likelihood of a 2^32-share request carries no weight, and the type today satisfies the `Iterator` contract while violating the `ExactSizeIterator` one on exactly the inputs the `u64` signature was widened to accept.

Evidence:

        65	    fn size_hint(&self) -> (usize, Option<usize>) {
        66	        // The count is u64 and `usize` may be narrower: past its range the hint
        67	        // is `(usize::MAX, None)`, the standard spelling for an iterator of
        68	        // more than `usize::MAX` items.
        69	        (
        70	            usize::try_from(self.remaining).unwrap_or(usize::MAX),
        71	            usize::try_from(self.remaining).ok(),
        72	        )
        73	    }
        74	}
        75	
        76	impl ExactSizeIterator for Split {}

Witness output (32-bit pass; the same guest and harness run as clock-17's):

    ```text
            PASS [   0.142s] ( 6/11) wasm32-pins-harness::zz_witness party_forks_len_past_usize_checked_guest
            PASS [   0.143s] ( 7/11) wasm32-pins-harness::zz_witness party_forks_size_hint_past_usize_checked_guest
            PASS [   0.144s] ( 9/11) wasm32-pins-harness::zz_witness party_forks_len_at_usize_checked_guest
    ```

    (`..._past_usize_...` asserts `Outcome::Trapped(Trap::UnreachableCodeReached)` at `k = 2^32`; `..._size_hint_...` asserts the `(_, None)` hint; `..._at_usize_...` asserts `Outcome::Value(4_294_967_295)` at `k = 2^32 - 1`.)

Resolution: Owner's choice among: document a `# Panics` on `Forks`/`clock::Forks` for `len()` past `usize::MAX` shares on 32-bit targets; override `len()` to saturate at `usize::MAX` (documented as the one place the count is inexact); or stop implementing `ExactSizeIterator` (an API removal, least attractive). Whichever is chosen, add a wasm32 pin. Nit alongside: `size_hint` converts `remaining` twice; compute `usize::try_from(self.remaining)` once. Acceptance: a committed wasm32-pins test exercises `len()` on `forks(u64::from(u32::MAX) + 1)` under the chosen contract; tests/forks_max.rs loses its `64-bit test host` caveat or states why it remains.
Construction: on any 32-bit target: `let mut p = Party::seed(); let it = p.forks(u64::from(u32::MAX) + 1); let _ = it.len();`. After the residual is drawn, `remaining` is `2^32 + 1`, `size_hint` is `(usize::MAX, None)`, and the default `len()`'s `assert_eq!(upper, Some(lower))` fails.

### party-14: `Forks` promises "exactly `k`" shares; at `k == u64::MAX` it yields one fewer, and only a `//` comment and a test file say so
- Where: crates/before/src/party/forks.rs:78-80 (related: crates/before/src/party.rs:239-240, crates/before/src/clock/forks.rs:9, crates/before/src/clock.rs:166, crates/before/src/party/forks.rs:111-114, crates/before/tests/forks_max.rs:1-11)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -n 'saturat\|u64::MAX'` over party.rs, party/forks.rs, clock.rs, clock/forks.rs finds only forks.rs:111 (a `//` comment) and :114 (the `saturating_add`); no `///` or `//!` line mentions the boundary; `git show cdad46060:crates/before/src/party/forks.rs` shows the public clause "(at the one saturating input `n == u64::MAX`, one fewer — see [`Party::forks`])" that b3f09baa0 removed); executed: no
- Seen by: prose, correctness; refutation: confirmed (severity lowered to low by the refutation because the behavior is owner-ruled and pinned; kept at medium here because the public contract is false at an input the code handles deliberately and the pin's module doc points at "the documented behavior" that no longer exists); history: no rationale found for the removal (owner's doc passes b3f09baa0, a431eaf1)
- Owner-gated: no

The public rustdoc of `Forks`, `Party::forks`, `clock::Forks`, and `Clock::forks` promises `k` shares without qualification; `Forks::new` saturates `k + 1`, so `forks(u64::MAX)` yields `u64::MAX − 1`. The boundary is stated in a non-doc comment and in `tests/forks_max.rs`, whose module doc calls it "the documented behavior". Correct at all scales, for all inputs: the likelihood of `u64::MAX` carries no weight, and the contract as written is false at one input while a committed pin protects the behavior the contract contradicts.

Evidence:

        78	/// A lazy iterator of balanced [`Party`] shares, returned by [`Party::forks`].
        79	///
        80	/// Yields exactly `k` disjoint shares produced one at a time. The party it

       111	        // The count saturates: at `k == u64::MAX` the residual's headroom is
       112	        // spent and the iterator yields one share fewer than asked.

         1	//! `forks(u64::MAX)`: the documented behavior at the split count's
         2	//! saturation boundary, pinned in both profiles.

Resolution: One sentence on `Forks`' doc, with a pointer from `Party::forks`, and the same on `clock::Forks`/`Clock::forks`: "The count is `k` for every `k < u64::MAX`; at `u64::MAX` it saturates to `u64::MAX − 1`, because the borrowed party must keep one share." (cdad46060's wording is a usable reference.) Acceptance: `cargo doc` renders the saturation clause on both iterators and both methods; tests/forks_max.rs:1 cites public prose that exists.

### party-15: `Forks`' Complexity section promises a per-step cost that is never stated
- Where: crates/before/src/party/forks.rs:86-92 (related: crates/before/src/party.rs:258, crates/before/src/clock.rs:178, crates/before/src/clock/forks.rs:18-19, crates/before/src/party/forks.rs:48-63)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed (the proposed bound is plausible from `Split::next` but its constants were not derived); history: no rationale found (b3f09baa0's wording)
- Owner-gated: no

`Party::forks` and `Clock::forks` defer with "see [`Forks`] for the per-step and early-drop costs"; `Forks` gives the early-drop cost and describes a step only as "proportionate to its share of the drain", which is not a bound a caller can plan against. Public rustdoc carries complexity where it routes a decision, and lazy iteration exists so a caller can bound per-step latency; a pointer that lands on a non-answer costs two reads for nothing.

Evidence:

        90	/// Each `next` costs proportionate to its share of the drain; an early drop
        91	/// rejoins the unclaimed remainder in `O(|p| log k)`, with `|p|` the borrowed
        92	/// party's size in bytes.

       258	    /// Shares are built on demand; see [`Forks`] for the per-step and early-drop costs.

Resolution: State the per-step bound the code has: each `next` performs at most `⌈log₂ k⌉` forks along the spine of one pending region, each `O(|p| + log k)` bits, so a step is `O(|p| log k)` worst case and `O(|p| + log k)` amortized over a full drain; verify the constants against `Split::next` (forks.rs:48-63) before landing. Acceptance: a reader following the pointer from `Party::forks` or `Clock::forks` finds a per-step bound in `Forks`' Complexity section.

### party-16: `IdBuilder` serves two clients with disjoint method sets and documents only one discipline
- Where: crates/before/src/party/ops/build.rs:4-14 (related: crates/before/src/party/ops/build.rs:137-150, crates/before/src/party/ops/build.rs:222, crates/before/src/party/ops/build.rs:262, crates/before/src/party/ops/build.rs:266, crates/before/src/party/ops/build.rs:302, crates/before/src/party/ops/sum.rs:44, crates/before/src/party/ops/sum.rs:51, crates/before/src/party/ops/sum.rs:76, crates/before/src/party/ops/sum.rs:109)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep over build.rs and sum.rs: `open`/`close_node` only at build.rs:222, 262, 302; `splice` only at 266; `copy_reader` at sum.rs:44, 51; `push_tag` at 76; `collapse_terminal_pair` at 109); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate but expired (the type doc described every client at 1d77f4c82; da0f6a937 gave `sum` its own final-tag discipline and left the type doc unchanged)
- Owner-gated: no

The type doc describes the reserve/patch/close discipline (`open`, `close_node`, `splice`), which only `IdSkylineBuilder` uses; `sum` uses a disjoint subset (`push_tag`, `copy_reader`, `collapse_terminal_pair`) that never reserves or patches. The type carries two ways to perform the `(1, 1) → 1` collapse, the second self-described as "The truncation twin of `close_node`'s terminal-collapse arm". A reader of `sum` who opens `IdBuilder` is told about placeholders it never creates. Steelman: both are id emitters over `PackedBuilder` sharing `terminal`, `finish`, `with_capacity`, and the `Built` vocabulary, so one wrapper avoids a third type; the doc is the cheaper fix.

Evidence:

         4	/// Single-buffer builder for normalized id output.
         5	///
         6	/// A node reserves a 2-bit tag placeholder before its children are emitted;
         7	/// [`close_node`](Self::close_node) patches the tag from which children turned
         8	/// out present, collapsing `(1, 1) → 1` (both terminal) and `(0, 0) → 0` (both
         9	/// empty). The id instantiation of the crate's append-truncate discipline
        10	/// ([`PackedBuilder`] carries the shared move set): the per-node payload is
        11	/// only the tag bits, and both collapses are pure truncations.

Resolution: Either rewrite the type doc to name both disciplines and which kernel uses which (final tags at descent plus fixed-width collapse for `sum`; reserve/patch/close for the leaf-driven builder), or move `Open`, `open`, `close_node`, and the reserve into `IdSkylineBuilder`, their only client, keeping `IdBuilder` as the shared core. Acceptance: `IdBuilder`'s doc names no method that only one of its two clients uses without saying so; `sum`/`diff` differentials unchanged.

### party-17: `Open` token doc claims an unclosed node "cannot compile"; `#[must_use]` is a warn-level lint on discarded expressions, and the module destructures the token itself
- Where: crates/before/src/party/ops/build.rs:36-38 (related: crates/before/src/party/ops/build.rs:222, crates/before/src/party/ops/build.rs:262, crates/before/src/party/ops/build.rs:302, crates/before/src/lib.rs:412-413, justfile:127)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lib.rs carries only `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`; the gate's `cargo clippy ... -- -D warnings` rejects a bare discarded `open()` but `let _o = b.open();` compiles silently; the destructure/reconstruct sites are build.rs:222, 262, 302); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate but expired (1d77f4c82 introduced the claim for a design where every walk closed via `close_node` with the token in hand; 7b11b3ea's `IdSkylineBuilder` bypasses the token onto `PosStack`)
- Owner-gated: no

Documentation altitude: a described type-level guarantee must be one the compiler gives. `#[must_use]` fires `unused_must_use` (warn by default) only when the value is discarded as a statement; reuse is prevented by move semantics, not the borrow checker; and `IdSkylineBuilder` destructures `Open(at)` onto `PosStack` and rebuilds `Open(self.tags.pop())` at close, so "closed exactly once" is a convention this module keeps, not a property the type enforces. A maintainer trusting the sentence would expect the compiler to catch a missing `close_node`; it will not.

Evidence:

        36	/// `!Clone` and `#[must_use]`: the token must be closed exactly once, and the
        37	/// borrow checker stops it being reused or dropped silently — so an open with
        38	/// no matching close cannot compile.

       222	            let Open(at) = self.out.open();

       302	                    kind = self.out.close_node(Open(self.tags.pop()), left, kind);

Resolution: Rewrite to what holds: "The token is `!Clone` and `#[must_use]`, so a discarded `open()` result warns and a token cannot be closed twice by accident; `IdSkylineBuilder` stores the position on `PosStack` and reconstructs the token at close, so the pairing there is kept by `close_up`'s stack discipline, not by the type." Alternatively have `PosStack` hand back `Open` tokens behind a method so the reconstruction is confined to one place. Acceptance: the `Open` doc makes no claim of a compile error; a reader can find where the token discipline is bypassed from the doc alone.

### party-18: `#[allow(clippy::wrong_self_convention)]` on `covers` guards nothing
- Where: crates/before/src/party/ops/compare.rs:32-34 (related: crates/before/src/party/ops/compare.rs:13-15)
- Class / severity / confidence: vestigial / nit / high
- Provenance: assessed (clippy's `wrong_self_convention` inspects only methods named with the `as_`/`from_`/`into_`/`is_`/`to_` prefixes; `covers` carries none); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (f0b837336 copied the attribute and comment from `is_disjoint`, where it is live)
- Owner-gated: no

A lint allow that suppresses nothing is scaffolding whose only justification is its sibling; the comment above it explains a self-by-value choice the lint does not police for this name.

Evidence:

        32	    // Single-use by-value readers, as with `is_disjoint`.
        33	    #[allow(clippy::wrong_self_convention)]
        34	    pub(crate) fn covers(self, other: IdReader) -> bool {

Resolution: Delete the attribute at compare.rs:33; keep the one-line comment if the by-value rationale is wanted here. Acceptance: `just clippy` clean with the attribute removed.

### party-19: `compare`, `sum`, and `diff` keep their per-ancestor bit stacks on the output buffer `BitsBuf` where `BitStack` exists
- Where: crates/before/src/party/ops/compare.rs:113-115 (related: crates/before/src/party/ops/sum.rs:135-137, crates/before/src/party/ops/diff.rs:253, crates/before/src/party/ops/diff.rs:257, crates/before/src/codec/stack.rs:1-8, crates/before/src/codec/buf.rs:176-181, crates/before/src/codec/buf.rs:230-239, crates/before/src/party/ops/build.rs:181-184, crates/before/src/version/skyline/overlay.rs:474-477)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`git show --stat 56a3dfe2` lists party/ops/build.rs and the skyline modules, not compare.rs, sum.rs, or diff.rs; `BitsBuf::pop` is `get` (bounds assert plus byte index) plus `truncate` (assert, `Vec::truncate`, `mask_tail`); `BitStack::pop` is one shift with a word spill); executed: no
- Seen by: structure, correctness, claims; refutation: confirmed; history: deliberate but expired (da0f6a937 used the crate's only bit container then; 56a3dfe2 introduced `BitStack` "for every path, phase, and frame" and converted build.rs and the skyline walks; the three party sites were missed, and 83e61b4d renamed them to `BitsBuf` without moving them)
- Owner-gated: no

`Lockstep.pending`, `Frames.bits`, and the diff cursor's `path`/`pending_right` are `BitsBuf` used purely as LIFO bit stacks on the hot paths of `join`, `is_disjoint`, `covers`, and `without`, while `IdSkylineBuilder` and `overlay::IdLeafCursor` use `BitStack`, whose module doc calls it "the word-backed bit stack the deep walks keep their paths and phases on". Two spellings of "a stack of bits" in one crate, the purpose-built one documented as the walks' discipline and the ad hoc one paying an output buffer's invariants (exact bytes, zeroed dead bits) on every push and pop. Fixed sign; no meter counts these pushes.

Evidence:

       113	struct Lockstep {
       114	    /// Two presence bits per queued right child pair, innermost on top.
       115	    pending: BitsBuf,

       135	struct Frames {
       136	    bits: BitsBuf,
       137	}

Resolution: Replace the four fields with `BitStack` (same `push`/`pop` API; `len()` is `u64` as `depth()` already returns). Acceptance: `grep -n 'BitsBuf' crates/before/src/party/ops/compare.rs crates/before/src/party/ops/sum.rs` shows only the output-buffer uses (`sum`'s return type); diff.rs's cursor fields are `BitStack`; all party differentials and deep constructed tests pass; no scan or heap pin moves except the `ID_COVERS`/`ID_DISJOINT` heap readings, which may fall to zero on the divert pair (re-pin as a deliberate event; party-1's contract wording stays a bound either way).

### party-20: Two `IdLeafCursor` types walk the same id coding; the decision to keep them separate is recorded only in an agent note
- Where: crates/before/src/party/ops/diff.rs:229-244 (related: crates/before/src/version/skyline/overlay.rs:16-20, crates/before/src/version/skyline/overlay.rs:471-485, crates/before/src/version/skyline/overlay.rs:583-614, crates/before/src/party/ops/diff.rs:414-437, crates/before/src/version/skyline/shape.rs:78-85, crates/before/src/version/skyline/masked.rs:209-211, crates/before/src/version/skyline/query.rs:502)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep 'impl PlateauCursor for'` returns three impls: diff.rs:381, overlay.rs:411, overlay.rs:555; the two `step` bodies implement the same pop-trailing-rights/flip/pop-right-present law with the same `unreachable!` text; the survey's §8 amendment at `.agent-notes/2026-07-29-fold-unification-survey/fold-unification-survey.md:499-509` records "Phase C: measured at the boundary and dropped, by this survey's own criterion"); executed: no
- Seen by: structure, correctness, claims; refutation: confirmed (unification feasible); history: already known and dropped on record (the merged cursor was judged not smaller or clearer: ~12 lines of residual flip bookkeeping against genuinely different descent policies, plus a re-pin bill)
- Owner-gated: no

The duplication three lenses reported is a decided drop, not a fresh defect: the survey measured the merge and rejected it. What the tree does not carry is that decision. `diff`'s cursor doc names the event-side `LeafCursor` as its sibling and never mentions the existing id cursor of the same name one module away, so the next reader re-derives the same finding (three lenses did). Code may not cite `.agent-notes`, so the rationale must be restated inline. overlay.rs:17's "the two cursor instances" is a module-scoped count (deliberately trimmed to the module's own two at c6ba2208) that a crate-wide reader takes as a crate-wide count; saying whose count it is closes that.

Evidence:

       229	/// A cursor at the current item of one packed id, read as a boolean
       230	/// skyline.
       231	///
       232	/// The id-side sibling of the event sweep's leaf cursor: the tag stream is
       233	/// consumed forward at most once, the root-to-item path is the only per-depth

        16	//! inside each slot's step; the boundary bookkeeping below is their shared
        17	//! correctness argument. Above them sit the two cursor instances.

Resolution: At diff.rs:229-244 add one paragraph: "A second id cursor beside [`overlay::IdLeafCursor`], deliberately: that cursor settles eagerly inside its step (its plateaus are the stored regions), this one defers settlement so the covered-block scans can skip what the sweep never visits; the shared flip bookkeeping is a dozen lines, and a merge would wrap an eager adapter around the settle-driven cursor or import block machinery into the shape and masked walks." At overlay.rs:17 "Above them sit this module's two cursor instances" (or name the third with its home). Acceptance: a reader of either cursor's doc can find the other and the reason they are two.

### party-21: Index doc claims a wall-time measurement no committed artifact holds
- Where: crates/before/src/party/ops/index.rs:22-24 (related: tools/benchjudge-expected.json)
- Class / severity / confidence: claim / nit / high
- Provenance: verified (`grep -c -i 'party\|join_all' tools/benchjudge-expected.json` is 0); executed: no
- Seen by: claims; refutation: confirmed; history: deliberate and holds as a measurement (29d3c8f27's message records the wall readings) but nothing in the tree binds it
- Owner-gated: no

"the searches tie or win wall time on every committed fold population, by measurement" names a measurement with no run to bind to: no bench-judge expectation covers a party operation. Principle 8 (a measurement binds to its run) and Principle 5 (a past measurement without an artifact rots silently). The asymptotic argument two sentences earlier is the load-bearing justification and stands alone.

Evidence:

        22	//! (the overlap rows and the flatness pin in `meter::board`'s tests price that
        23	//! regression), while the searches tie or win wall time on every committed fold
        24	//! population, by measurement. Every probe records one table word in the scan

Resolution: Delete the clause, or add `party_join_all` to the bench-judge roster (`tools/benchjudge-expected.json`) and cite that cell by name. Acceptance: every measurement claim in the module doc names a committed instrument.
Construction: none needed beyond the grep: the claim names no artifact, and `tools/benchjudge-expected.json` has no party entry.

### party-22: `IdIndex` doc says its `u32` table is "strictly smaller than the operand itself"; it is up to eight times larger
- Where: crates/before/src/party/ops/index.rs:43-44 (related: crates/before/src/party/ops/index.rs:92, crates/before/src/party/ops/index.rs:189, crates/before/src/party.rs:324, crates/before/src/idbits.rs:5-10)
- Class / severity / confidence: claim / medium / high
- Provenance: assessed (arithmetic from the coding at idbits.rs:5-10: a both-present node's tag is 2 bits and each of its two present children is at least a 2-bit terminal, so `B` both-present nodes need at least `4B + 2` stream bits while the table is `32B` bits); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed; history: no rationale found (a99e8c8f7's sentence, rewrapped by b3f09baa0, no derivation)
- Owner-gated: no

The sentence is a space statement in a transient-state sentence, so "smaller" means bits, and it is false for every `B ≥ 1`. The crate's promise that transient space is "a small constant multiple of the input size" (lib.rs:333-340) still holds, and `join_all`'s `O(|self| + |iter|)` auxiliary space (party.rs:324) still covers it, so this is a false sentence, not a space bug; a maintainer sizing the fold's peak from it is misled by nearly an order of magnitude. The 24-byte pending entries (party-24) belong to the same sentence if the per-node figure is meant to be the space envelope.

Evidence:

        41	/// Build once with [`build`](IdIndex::build), then run
        42	/// [`is_disjoint`](IdIndex::is_disjoint) against any number of independent
        43	/// operands. Transient state is at most one `u32` per both-present node of the
        44	/// indexed operand — strictly smaller than the operand itself.

Resolution: State the bound the code has: "one machine word (32 bits) per both-present node of the indexed operand: up to eight times the operand's bit length (a both-present node and its forced subtree cost at least four bits), `O(|self|)` and freed with the fold", and add the pending stack's per-level cost if the sentence is meant as the space envelope. Acceptance: the smallest witness (`(1, (1, 0))`, 8 bits of operand, one 32-bit entry) satisfies the stated bound; no prose in the partition claims the index is smaller than its operand.
Construction: `Party::try_from((1u8, (1u8, 0u8)))` encodes as `11 00 10 00` (8 bits) with one both-present node; `IdIndex::build` allocates `vec![0u32; 1]` (32 bits). The left comb `L_0 = (1, 0)`, `L_{k+1} = (L_k, 1)` is `4d + 4` bits with `d` both-present nodes, so the table is `32d` bits and the ratio approaches 8.

### party-23: Past 2^32 packed bits the fold index falls back to the quadratic discipline; the public bound carries no size clause
- Where: crates/before/src/party/ops/index.rs:53-57 (related: crates/before/src/party/ops/index.rs:73-75, crates/before/src/party/ops/index.rs:156-159, crates/before/src/party/ops/index.rs:174-178, crates/before/src/party/tests.rs:373-470, crates/before-fuelscape/src/ops.rs:806, crates/before/src/lib.rs:350-353)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read `build`'s early return, `is_disjoint`'s fallback arm, the island contract string, and lib.rs:351-353 "for all input sizes, no matter how unlikely"); executed: no
- Seen by: claims; refutation: confirmed; history: the fallback is deliberate and documented at the field from birth (a99e8c8f7) and pinned via `build_unindexed` (522705cff); no record weighs it against the public contract
- Owner-gated: yes: a documented design decision, and the alternative is a public-contract clause

When the accumulator exceeds `u32::MAX` bits, `build` returns no table and `is_disjoint` cursor-walks the fixed side per input, making `join_all` `Θ(k·|self|)` against the public island contract `O((|self| + |iter|) log k + (|self| + |iter|) log |self|)` and the crate's all-sizes guarantee. A 512 MiB party is materializable, so this is not the tolerated 2^64-iteration corner. The `build_unindexed` test-only constructor and the two fallback differentials exist only to hold the arm the contract does not admit.

Evidence:

        53	    /// `None` when the stream's positions do not fit `u32` (≥ 512 MiB of packed
        54	    /// id); [`is_disjoint`](IdIndex::is_disjoint) then falls back to the
        55	    /// per-input cursor walk, which answers the same predicate at the fold's
        56	    /// unindexed cost.
        57	    rights: Option<Vec<u32>>,

       806	        contract: "`O((|self| + |iter|) log k + (|self| + |iter|) log |self|)` time, `k` the operand count",

Resolution: Owner's call between keeping the order at every size (`enum Rights { Narrow(Vec<u32>), Wide(Vec<u64>) }` chosen by `bits.len()`, or `Vec<u64>` unconditionally if the board's `party_join_all` heap reading tolerates it; measure at the parent first), dissolving the fallback arm, `build_unindexed`, and the fallback differentials or converting the latter to a wide-table differential, or carrying the size clause in the island contract and `join_all`'s rustdoc. Acceptance: either `is_disjoint` has no unindexed arm and the deep and arbitrary index differentials pass against a forced wide table, or the public contract names the threshold.
Construction: any accumulator over 2^32 bits with `k` one-byte overlapping probes reads `Θ(k·|self|)` scan bits under the current code where the contract predicts `Θ(k log |self|)`; unaffordable to materialize in a test (as the field doc says), which is why the finding is a contract clause rather than a failing test.

### party-24: `IdIndex::is_disjoint` spends 24 bytes per queued pair and `build` one byte per open frame, where every sibling walk spends bits
- Where: crates/before/src/party/ops/index.rs:185-189 (related: crates/before/src/party/ops/index.rs:73-75, crates/before/src/party/ops/index.rs:96, crates/before/src/party/ops/compare.rs:113-115, crates/before/src/party/ops/sum.rs:135-137, crates/before/src/party/ops/build.rs:328-335, crates/before/src/party/ops.rs:6-8)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read: `Vec<(Option<u64>, usize)>` is 24 bytes per entry on 64-bit; the table exists only when `bits.len() <= u32::MAX` so both stored words fit `u32`); executed: no
- Seen by: claims (filed), structure (asked as an open question); refutation: confirmed; history: no rationale found (a99e8c8f7 as written)
- Owner-gated: no

On a both-present chain the pending stack is about 32× the input operand's bytes, while `Lockstep` (2 bits), `Frames` (2-3 bits), and the diff cursor price the same depth in bits, and ops.rs:7-8 states the discipline "a deep operand costs bits, not stack frames or grown segments". The table guard already proves positions and entry indexes fit `u32`, so `(u32, u32)` with a sentinel is an 8-byte entry with no algorithmic change; two delta-coded `PopStack`s would match the siblings' bit pricing. `awaiting_left: Vec<bool>` in `build` is the same shape at one byte per frame.

Evidence:

       185	        // Queued right pairs, innermost last: the indexed side's position
       186	        // (`None` = absent child) and its entry bound. `other`'s right children
       187	        // need no bookkeeping — its cursor reaches each one in stream order,
       188	        // exactly as in the cursor walk.
       189	        let mut pending: Vec<(Option<u64>, usize)> = Vec::new();

Resolution: Store `(u32, u32)` with `u32::MAX` for the absent side (or two `PopStack`s of deltas), make `awaiting_left` a `BitStack`, and state the per-level transient in the method doc as the other walks do. Acceptance: peak heap of `IdIndex::build(acc).is_disjoint(input)` on a both-present chain at depth `d` falls from about `24d` to at most `8d` bytes under the `PeakAlloc` meter; `indexed_disjointness_matches_the_cursor_walk[_deep]` and `unindexed_fallback_*` stay green.

### party-25: `IdIndex::is_disjoint`'s stated `O(Σ inputs + B log n)` bound does not describe the code: a table search runs in the left-only arm and its result is discarded
- Where: crates/before/src/party/ops/index.rs:220-247 (related: crates/before/src/party/ops/index.rs:15-17, crates/before/src/party/ops/index.rs:279-291, crates/before/src/meter/board/ops.rs:1370-1377, crates/before/src/party/tests.rs:1130-1162)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read the arm order: the `if al && ar` search runs before `match (bl, br)`, and in the `(true, false)` arm `a_right`/`right_entry` are never read; the module doc defines `B` as "the inputs' both-present node count"; the construction below was traced by hand against the match arms); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (the arm structure is a99e8c8f7's from birth; 29d3c8f27 priced searches per input both-present node without noting the discarded case)
- Owner-gated: no

The module doc prices one `O(log n)` search "per node both sides own", with `B` the inputs' both-present node count. The implementation runs `metered_partition_point` whenever the indexed node is both-present and the input node is `Internal`, regardless of the input's presence bits, so searches fire on pairs the doc's `B` does not count and the code pays for a result it throws away. The argument and the implementation disagree in both directions, and the discarded searches inflate the scan-currency readings the board's fold cells and the parity-halves floor are calibrated against.

Evidence:

        15	//! costs `O(input)` node visits plus one `O(log n)` table search per node both
        16	//! sides own — so the fold's up-front tests total `O(Σ inputs + B log n)`, `B`
        17	//! the inputs' both-present node count. The search term is not bounded by the

       220	                            let (a_right, right_entry) = if al && ar {

       244	                                (true, false) => {
       245	                                    (node, entry) = (a_left, left_entry);
       246	                                    continue;
       247	                                }

Resolution: Compute the right-child position lazily: move the `if al && ar { … metered_partition_point … }` block into the `(true, true)` and `(false, true)` arms (or a closure invoked only there), leaving `(true, false)` at `O(1)`; then restate the bound with `B` defined as the visited pairs whose indexed node is both-present and whose input node has a right child. Acceptance: under `scan-meter` the construction below records zero 32-bit probes; `indexed_disjointness_matches_the_cursor_walk[_deep]`, `unindexed_fallback_matches_the_walk_on_constructed_pairs`, and the `join_all` oracle differentials stay green; `indexed_disjointness_search_bits_stay_metered` still passes (on the parity halves every input node is both-present); any board or envelope reading that moves is measured at the parent and attributed.
Construction: indexed operand `a` = the left comb `L_0 = (1, 0)`, `L_{k+1} = (L_k, 1)`, `a = L_d` (`d` both-present nodes whose right children are terminals). Input `b` = `d` left-only nodes over one right-only node over a terminal, which owns a cell inside the one region `a` leaves unowned, so the pair is disjoint. At depths `0..d-1` the pair is (both-present indexed node, left-only input node): the current code runs one `metered_partition_point` per level and discards it; `b` has zero both-present nodes, so the doc's bound predicts zero searches. Wrap `IdIndex::build(a).is_disjoint(IdReader::root(b))` in `scan::reset()`/`scan_bits()` under `scan-meter` and subtract the cursor walk's reading on the same pair: the difference is `d` searches' worth of 32-bit probes today and must be zero once the search is lazy.

### party-26: Table searches scan to the table's end instead of the enclosing subtree's entry bound
- Where: crates/before/src/party/ops/index.rs:226-229 (related: crates/before/src/party/ops/index.rs:279-291, crates/before/src/party/tests.rs:1139-1147, crates/before/src/meter/board/ops.rs:1373-1377)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read: the left subtree's entries follow `entry` with targets below `rights[entry]`, the right subtree's entries have larger targets and end at the parent's bound, so restricting the slice to `[entry + 1, end)` preserves the partition point and can only reduce probes); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (29d3c8f27 priced probes at `log2(table + 1)` per node, matching the whole-suffix search)
- Owner-gated: no

Each both-present node's search runs `partition_point` over `rights[entry + 1..]`, the whole remaining table, so every search is `log₂(table)` probes; carrying an end bound on the pending stack makes each search `log₂(subtree entries)`, and on a complete skeleton the total falls from about `B·log B` to about `2B` probes. The module doc calls the search term "a price this module pays knowingly"; this reduces the price without changing the mechanism. Construct and measure: the parity-halves floor (tests.rs:1147, a measured ×0.75) would trip, and the board's `probes_per_node` model (a ceiling) stays valid; see the open question on that floor.

Evidence:

       226	                                let target = rights[entry];
       227	                                let after_left = entry
       228	                                    + 1
       229	                                    + metered_partition_point(&rights[entry + 1..], target);

Resolution: Carry `end` beside `entry` (the left child's entries are `[left_entry, after_left)`, the right child's `[right_entry, parent_end)`), search `&rights[entry + 1..end]`, and re-derive the parity-halves floor and the board's `with_fold_search` allowance from the tighter per-node model. Acceptance: scan bits on `parity_halves(10)` drop by roughly 4× under the same verdict; the re-derived floor still sits an order above the cursor co-walk's reading; `party_join_all` board cells read at or under their declared model.

### party-27: `split`'s spine walk and output copies sit outside the scan meter; the exemption lives only in a `tests/meter.rs` comment, and `ops.rs` points at a rationale `build_split` does not carry
- Where: crates/before/src/party/ops/split.rs:45-53 (related: crates/before/src/party/ops.rs:38-40, crates/before/src/party/ops/split.rs:38-44, crates/before/src/party/ops/sum_split.rs:123-126, crates/before/src/party/ops/sum_split.rs:205-223, crates/before/src/codec/buf.rs:164-173, crates/before/src/codec/buf.rs:346-369, crates/before/src/codec/scan.rs:9-11, crates/before/tests/meter.rs:6397-6410)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'record_bits\|scan'` over split.rs and sum_split.rs returns only doc prose; `BitsBuf::push` and `extend_from_view` record nothing; the only statement of the exemption is tests/meter.rs:6400-6405; split.rs:38-44 describes the splice and never says why the builder is not used); executed: no
- Seen by: structure, claims ([57]); refutation: confirmed; history: deliberate and holds (fc862595d: "the scan pin records the split kernel's deliberately raw path, so wiring it into the metered primitives is a deliberate re-pin"), stated only in the test and in history
- Owner-gated: no (routing the walk through the meter would be, and is left as an open question)

`build_split` reads each spine tag with raw `bits.bit()` and writes both halves through `extend_from_view`/`BitsBuf::push`, none of which records, so `Party::fork`'s scan reading is a constant independent of spine depth; `sum_split`'s spine pushes and `half`/`splice` copies are outside the meter the same way. That is a recorded decision, but the record lives in one envelope's comment: nothing at the code says so, `scan.rs`'s coverage list reads as total, and ops.rs:40 sends the reader to `build_split` "for why it does not use the builder", where the doc states a property (verbatim copies of already-normal ranges) but never names the builder or the meter. Comments state what the code cannot show, at the code that creates the blind spot.

Evidence:

        45	fn build_split(bits: BitsView<'_>, start: u64) -> (BitsBuf, BitsBuf) {
        46	    let mut pos = start;
        47	    let (prefix_end, kind) = loop {
        48	        match (bits.bit(pos), bits.bit(pos + 1)) {
        49	            (false, false) => break (pos, SpineEnd::Terminal), // the `1` leaf
        50	            (true, true) => break (pos, SpineEnd::Branch),     // both-present branch
        51	            _ => pos += 2, // unary: descend the single present child (at pos + 2)
        52	        }
        53	    };

        38	//! normal form. Output is built by [`build::IdBuilder`] (`sum`), the
        39	//! leaf-driven [`build::IdSkylineBuilder`] (`diff`), or by direct bit-splice
        40	//! (`split`); see `split`'s `build_split` for why it does not use the builder.

      6400	/// The split kernel builds both halves by raw bit-slice writes and walks
      6401	/// the spine by raw indexing — deliberately outside the scan primitives —

Resolution: State the exemption where it lives: at `build_split` ("the halves are verbatim slices of already-normal ranges plus one retagged node, so no tag is reserved, patched, or collapsed and the builder's placeholder discipline buys nothing; the spine read and the copies are deliberately outside the scan meter, whose fork envelope pins the raw path's near-zero reading"), at `sum_split::half`/`splice`, and as an explicit bullet in scan.rs:9-11's coverage list naming the two kernels whose reads and writes it does not count. Acceptance: `grep -n 'outside the scan' crates/before/src/party/ops/split.rs crates/before/src/codec/scan.rs` hits; ops.rs:40's pointer lands on the answer it promises.

### party-28: `split` and `sum_split` rest on a stream-suffix precondition the type does not carry: a dead `start` parameter, a redundant right-child scan asserted equal to `bits.len()`, and an unasserted twin in `branch_children`
- Where: crates/before/src/party/ops/split.rs:62-68 (related: crates/before/src/party/ops/split.rs:20-27, crates/before/src/party/ops/split.rs:78, crates/before/src/party/ops/split.rs:82, crates/before/src/party/ops/split.rs:89-93, crates/before/src/party/ops/sum_split.rs:162-165, crates/before/src/party/ops/sum_split.rs:167-185, crates/before/src/idbits.rs:92-96, crates/before/src/party.rs:234, crates/before/src/party/ops/sum_split.rs:72, crates/before/src/party/ops/sum_split.rs:75, crates/before/src/party/ops/sum_split.rs:105)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn '\.split()'` excluding tests and the oracle: party.rs:234 `self.view()`, sum_split.rs:72/75 root operands, sum_split.rs:105 `IdReader::root(...)`, so `start` is always 0 and every split subtree is the stream's tail; `subtree_end(bits, right_child)` is computed in every profile and used at 78 and 82, then debug-asserted equal to `bits.len()`; `branch_children` states the suffix fact in prose and asserts nothing; `IdReader::at` makes a mid-stream reader constructible); executed: no
- Seen by: structure ([11]), correctness ([42]), claims ([50]); refutation: confirmed and reframed (the debug asserts pin a suffix precondition, not `start == 0`); history: no rationale found (all from 32a655438 and c7c9d3e8f as written)
- Owner-gated: no

Three faces of one unstated precondition. `build_split` takes a `start` that every caller passes as 0. It skips the whole right branch child to compute `branch_end` and then debug-asserts that value equals `bits.len()`, so in release the scan is redundant work with a fixed sign (2 bits per node of the right child through `IdReader::skip`, up to half of `fork`'s tag reads on a right-heavy branch) and in debug it is a precondition check dressed as a computation. `branch_children` relies on the same fact ("the last present child runs to the stream's end") in prose with no assert. A reader positioned mid-stream by `IdReader::at` would satisfy the code's types and violate its arithmetic; the precondition belongs in the doc or the type.

Evidence:

        61	            let left_child = prefix_end + 2;
        62	            let right_child = subtree_end(bits, left_child);
        63	            let branch_end = subtree_end(bits, right_child);
        64	            debug_assert_eq!(
        65	                branch_end,
        66	                bits.len(),
        67	                "the branch subtree is the spine's tail",
        68	            );

        25	        let start = self.pos();
        26	        build_split(self.bits(), start)

       162	/// The branch node's subtree is a suffix of its stream (the spine descent above
       163	/// it consumed only unary tags), so the last present child runs to the stream's
       164	/// end and only a both-present operand pays a skip — of its left child, to find
       165	/// the boundary between the two.

Resolution: Make `split` and `sum_split` whole-stream operations in name and doc (they are today in every caller): drop `start` (`build_split(bits)` from 0), use `bits.len()` for `branch_end` and the capacity hint, keep the relation as `debug_assert_eq!(subtree_end(bits, right_child), bits.len(), ..)` so debug builds still check the root-entry precondition while release builds do no scan, add the same debug assert in `branch_children`, and state the precondition in both method docs ("the reader must be at a stream root"). If the owner prefers structural enforcement, type both entries on a root `BitsView` instead of a positioned reader. Acceptance: fork of `node(Some(&full()), Some(&leftmost(k)))` records a constant number of scan bits at any `k` instead of `4 + 2k`; the board's `party_fork` scan readings on right-heavy families move down and are re-pinned as a deliberate event; `d_fork_join_roundtrip`, `split_arbitrary`, and `sum_split_is_sum_then_split` stay green.

### party-29: peek-then-read decodes every tag twice in `sum`, `sum_split`, and `copy_reader`, and counts it twice in the scan currency
- Where: crates/before/src/party/ops/sum.rs:38-39 (related: crates/before/src/party/ops/sum.rs:67-68, crates/before/src/party/ops/sum_split.rs:80, crates/before/src/party/ops/sum_split.rs:110-111, crates/before/src/party/ops/sum_split.rs:123-124, crates/before/src/party/ops/build.rs:89-91, crates/before/src/idbits.rs:126-140, crates/before/src/party/tests.rs:788-792)
- Class / severity / confidence: performance / nit / high
- Provenance: verified (read the peek and read sites; `peek` and `read` each record 2 bits at idbits.rs:118 and :136; `skip` re-decodes the top tag through `skip_subtree`'s header probe; tests.rs:788-791 spells the double count: "peeked, then read"); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (0077affbe introduced `peek` for the copy side; 24dff4f58 pinned "peeked, then read" = 8 as observed)
- Owner-gated: no

`sum` peeks both nodes, then in the both-internal arm reads each, re-decoding the tag it just decoded and recording another 2 bits; `sum_split` does the same at every spine node, and `copy_reader` peeks then `skip`s the same top tag. Fixed-sign deletion of one decode per node, and it removes a metering skew: a `sum` operand tag records 4 scan bits where `is_disjoint` records 2 for the same read, so the currency prices identical work differently across walks.

Evidence:

        38	            let a_node = if a_on { self.peek() } else { IdNode::Empty };
        39	            let b_node = if b_on { other.peek() } else { IdNode::Empty };

        67	                    self.read();
        68	                    other.read();

Resolution: Add `IdReader::advance_past_tag(&mut self)` (a 2-bit position advance, no decode, no record) and use it after every peek that is followed by a read of the same node; re-pin the exact scan witnesses (`sum_split_scan_never_exceeds_the_composition`'s 8-bit splice constant becomes 4) as a deliberate event. Acceptance: the splice witness reads 4 bits (one peek per operand); the sum_split scan test and the id differentials stay green.

### party-30: `sum_split` re-accumulates the union spine that is already present verbatim in either operand
- Where: crates/before/src/party/ops/sum_split.rs:77-78 (related: crates/before/src/party/ops/sum_split.rs:18-28, crates/before/src/party/ops/sum_split.rs:94-95, crates/before/src/party/ops/sum_split.rs:104, crates/before/src/party/ops/sum_split.rs:123-126, crates/before/src/party/ops/sum_split.rs:205-223, crates/before/src/party/ops/split.rs:72-76)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read: on a unary union node exactly one of `(al || bl, ar || br)` is set, so the other side's presence is false on both operands, and `IdNode::Internal` has at least one present child, so the present side is true on both; hence `a`'s tag equals `b`'s tag equals the union tag at every spine level, and the `spine` buffer duplicates `self.bits()[start..self.pos()]`); executed: no
- Seen by: structure; refutation: confirmed (including the caveat that line 104's `self.sum(other)` consumes `self`, so the range must be captured before it); history: no rationale found (c7c9d3e8f as written)
- Owner-gated: no

The spine loop pushes each union tag into a fresh `BitsBuf`, and `half`/`splice` copy it into both halves; the method's own spine argument (lines 18-28) proves the pushed bits equal the operand's prefix bit for bit. A buffer whose only content is a copy of bits the walk just read, plus two allocations and per-tag pushes to maintain it, is circular machinery. Splicing the operand prefix is the move `split.rs` already makes for its prefix, so the two kernels would spell the spine the same way.

Evidence:

        77	        // The union's spine tags, shared by both halves (split's prefix).
        78	        let mut spine = BitsBuf::new();

       123	            self.read();
       124	            other.read();
       125	            spine.push(left);
       126	            spine.push(right);

Resolution: Record `(self.bits(), self.pos())` before the loop and `self.pos()` after it (before the delegated `self.sum(other)` at 104 consumes `self`); `half` and `splice` take `(BitsView, Range<u64>)` and `extend_from_view` the range; delete `spine`; note in the method doc that the operand prefix is the union spine, which the spine argument already proves. Acceptance: `sum_split_is_sum_then_split`, `sum_split_collapsed_union_matches_terminal_split`, `sum_split_constructed::*`, and `sum_split_scan_never_exceeds_the_composition` pass with unchanged readings (spine pushes and `extend_from_view` are both unmetered, so `fused` does not move).

### party-31: `join_all_hands_back_aliased_inputs` builds its alias by seeding two extra universes instead of `dangerously_alias`
- Where: crates/before/src/party/tests.rs:39-53 (related: crates/before/src/party/tests.rs:113, crates/before/src/party/tests.rs:129, crates/before/src/party/tests.rs:143, crates/before/src/party/tests.rs:272)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read); executed: no
- Seen by: prose, structure (bundled in [15]); refutation: confirmed; history: no rationale found (4f12b8218; `dangerously_alias` existed since 6b93f0af1 and every sibling test uses it)
- Owner-gated: no

The test needs a duplicate of one share and its expected hand-back; it obtains them by seeding two more independent universes, forking each, and feeding a share from the second into the first's `join_all`. The crate's hard rule is that independently seeded universes never interact, and `dangerously_alias` exists for exactly this need; the body relies on two seeds splitting identically, which is true but is not what the doc ("a duplicated share") says it exercises, and it models the one thing the crate tells users never to do.

Evidence:

        40	    let mut acc = Party::seed();
        41	    let shares: Vec<Party> = acc.forks(3).collect();
        42	    let mut dup_seed = Party::seed();
        43	    let mut dups = dup_seed.forks(3);
        44	    let duplicate = dups.next().expect("three shares were requested");

Resolution: `let duplicate = shares[0].dangerously_alias(); let expected_back = shares[0].dangerously_alias();` and delete the two extra seeds. Acceptance: the test constructs one universe; its assertions are unchanged and still pass.

### party-32: `parse_bare_notation` claims the `TryFrom` literals agree with the string parser but never compares them
- Where: crates/before/src/party/tests.rs:347-359 (related: crates/before/src/party/tests.rs:454-465)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read); executed: no
- Seen by: structure, prose; refutation: confirmed; history: no rationale found (unchanged since eecf92295)
- Owner-gated: no

The doc's invariant is "build parties via the same paper notation as the string parser"; the body checks only that three literals construct and `0` is rejected. Nothing ties a literal to its parsed twin, and the name says "parse" while the body exercises `TryFrom`. Every test's doc comment must state the invariant the body checks; a doc promising agreement the assertions do not test is the "not wrong, but you could not tell if it were" gap. Alongside: `crate::codec::built_view` is spelled fully qualified about fourteen times in this file where one `use` would do.

Evidence:

       347	/// `TryFrom` numeric/tuple literals build parties via the same paper notation
       348	/// as the string parser.

       355	    let _party: Party = 1.try_into().unwrap();
       356	    assert!(Party::try_from(0).is_err());
       357	    let _party: Party = (1, 0).try_into().unwrap();
       358	    let _party: Party = ((0, 1), (1, (1, 0))).try_into().unwrap();

Resolution: Assert `Party::try_from(lit).unwrap() == text.parse::<Party>().unwrap()` for each literal/text pair (`(1, 0)` and `"(1, 0)"`, `((0, 1), (1, (1, 0)))` and `"((0, 1), (1, (1, 0)))"`) and rename to `literal_doors_agree_with_the_parser`; or narrow the doc to what the body checks. Add `use crate::codec::built_view;` at the top of the file. Acceptance: the doc and the body state the same invariant; a literal door producing a different tree from the parser fails the test; `grep -c 'crate::codec::built_view' crates/before/src/party/tests.rs` is 0.

### party-33: The deep `is_disjoint` differential compares production against production, and `covers` has no deep differential at all
- Where: crates/before/src/party/tests.rs:407-423 (related: crates/before/src/party/tests.rs:829-834, crates/before/src/testing/generators.rs:298, crates/before/src/testing/exhaustive.rs:85-91, crates/before/src/surface.rs:383-391, crates/before/tests/meter.rs:6135-6146)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (the deep test holds `IdIndex::is_disjoint` to `IdReader::is_disjoint` only; the oracle legs `party_disjointness_matches_the_oracle`/`party_covers_matches_the_oracle` run over the arbitrary population at `ARB_DEPTH = 4` and organic pairs; the exhaustive deep variant is `#[ignore]` at id depth 4; `id_covers_envelope` asserts one `false`; the bridge lowers parties at `ORACLE_SCALE_MAX = 4096`); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (522705cff added the deep leg as the mechanism seam; no record considers an oracle leg at the shape scales)
- Owner-gated: no

Two production mechanisms agreeing is not agreement with the paper. The shared normal-form precondition (an `Internal` node never covers a `Full` leaf, compare.rs:84-99) is exactly the kind of assumption both mechanisms share and the oracle does not, and the bridge already reaches these scales cheaply for `diff_constructed`.

Evidence:

       416	        for (x, y) in [(&a, &b), (&a, &a), (&sa, &sb)] {
       417	            let walk = x.is_disjoint(y);
       418	            prop_assert_eq!(IdIndex::build(x.as_bits()).is_disjoint(y.view()), walk);
       419	            prop_assert_eq!(IdIndex::build(y.as_bits()).is_disjoint(x.view()), walk);

Resolution: In the deep test add `prop_assert_eq!(walk, to_oracle_party(x).is_disjoint(&to_oracle_party(y)))` and a parallel `covers` leg in both roles over the same shape triples, keeping the scale under `ORACLE_SCALE_MAX`. Acceptance: the deep test's doc names the oracle as the reference; a `covers` verdict is checked against the oracle at scale at least 64 in both operand orders.
Construction: the shape pairs already built there suffice (`(&a, &a)` gives the overlapping verdict, the skip-stress pair the disjoint one); for `covers`, add `(shape_party(shape, scale), shape_party(shape, scale / 2))` and a `node(Some(&a), None)` wrapper so the `true` arm is reached. The test today would not fail if `IdIndex` and `IdReader` shared a wrong verdict on a deep pair.

### party-34: `sum_split_scan_never_exceeds_the_composition` compares mixed currencies: the composed side counts `sum`'s builder writes, the fused side counts reads only
- Where: crates/before/src/party/tests.rs:740-769 (related: crates/before/src/party/ops/sum_split.rs:62-66, crates/before/src/party/ops/sum_split.rs:205-223, crates/before/src/party/ops/split.rs:45-53, crates/before/src/codec/build.rs:82-83, crates/before/src/codec/build.rs:127-128, crates/before/src/codec/build.rs:175-180, crates/before/src/codec/buf.rs:164-173, crates/before/src/codec/buf.rs:346-369)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`PackedBuilder::{push_bit, reserve, splice}` call `record_bits`; `BitsBuf::push` and `extend_from_view` record nothing; `build_split`'s spine loop records nothing); executed: no
- Seen by: correctness; refutation: confirmed (nuance: the exact `spliced == 8` pin at 788-792 does convict a spliced-subtree scan; the doc clause the slack does not support is "re-reads a skipped child"); history: no rationale found (24dff4f58 pinned the ratios as observed)
- Owner-gated: no

The test asserts `fused <= composed` in scan bits, but `sum` records its output writes while `sum_split`'s half assembly (`BitsBuf::push`/`extend_from_view`) and `split`'s spine read record nothing, so the composed side carries about the union's write bits of slack (about `2k` bits on the committed "adjacent k" regime). The test doc says a fused walk that "re-reads a skipped child" moves the ratio above one; a re-read of one skipped child adds 2 bits and passes. A meter whose two sides count different things cannot fail for the regression it names.

Evidence:

       747	    /// derived around), and the lockstep spine to a targeted branch. A fused
       748	    /// walk that re-reads a skipped child or scans a spliced subtree moves the
       749	    /// ratio above one.

       768	            assert!(
       769	                0 < fused && fused <= composed,

Resolution: Either meter the fused side's writes and the spine read (the re-pin route of party-27, which also moves `ID_FORK`'s scan column and the `spliced == 8` constant) so both sides count reads plus writes, or keep the raw path and compare like with like (subtract `sum`'s write bits from `composed`, or compare against a reads-only composition) and correct the doc to name exactly which regressions the assertion convicts. Acceptance: the construction below fails the test; the doc's stated conviction matches what the assertion can refute.
Construction: on the committed "adjacent k" regime (`a = leftmost(k)`, `b = spine(k - 1, true, node(None, Some(&full())))`), inject the regression the doc names: in `branch_children` (sum_split.rs:175-179) add a second `IdReader::at(bits, start).skip()` after `probe.skip()`. `fused` rises by 2 bits and `fused <= composed` still holds at both scales; only a re-walk of at least about `2k` bits (re-scanning the spine) trips the assertion today.

## Positives

- `sum_split`'s method doc (sum_split.rs:13-66) is a real fusion argument written where the code lives: the spine-is-lockstep and branch-is-verbatim-or-merge lemmas, the no-double-read mode choice, and the one seam (`split`'s terminal arm) named with the differential that pins it. It is held by a total byte-equality oracle (`sum_split_is_sum_then_split`, `None` arm and empty-operand identities included) plus deterministic deep witnesses at 10 000 levels in every regime.
- `lockstep_holds` (compare.rs:59-101) parametrizes the two region predicates by the single `a_settles` node with the rest of the algebra fixed and shared; `Lockstep`'s doc (103-112) states the invariant that keeps the stack empty on unary chains and why, and each call site names its refuting mixes in one comment.
- `sum` writes each output tag final at descent and collapses `(1, 1) → 1` by retracting a fixed-width suffix (sum.rs:69-76, build.rs:137-150), so no position stack exists; the comment at 69-73 says exactly why the tag is knowable at first sight.
- `diff.rs`'s module doc (18-38) gives the covered-block taxonomy completely (all four pairings, which three are blocks and why the fourth is not, where lockstep descent takes over), and the private docs on `settle_a`/`settle_b`/`descend_pair` restate exactly the cover the caller guarantees.
- Every `unreachable!`/`expect` in the partition is a one-line proof that reads true against the surrounding code (diff.rs:126, 300, 419-421; sum_split.rs:87, 92, 183, 201; index.rs:253; sum.rs:183-190; build.rs:323; idbits.rs:190). None is reachable from decoded bytes, text, or the public API.
- `PosStack` (build.rs:328-370) delta-codes reserved tag positions so a deep `diff` output costs bits per open ancestor, with the off-by-one for delta 0 explained at the field.
- `IdIndex`'s module doc (index.rs:1-33) names the trade it makes (a search term not bounded by the operands), the instruments that price it, and why it exists outside itself (the fold's repeated test against one fixed side); `build_unindexed` pins the fallback arm without a 512 MiB input. That candor is what made party-22, party-23, and party-25 findable.
- `join_all_differential_convicts_the_dropped_group_oracle` (tests.rs:172-290) commits a known-bad reference and asserts the criterion rejects it at exactly the widths that reach the arm: the adequacy demonstration the doctrine asks for, spelled as a test.
- The fork orbit pins (tests.rs:1044-1101) are exact two-sided trajectories over 512 steps in both directions, closing the compounding-cost gap a per-call bound leaves open, and the constructed-shape module (586-649) builds deep witnesses tags-first in one allocation so `diff` and `sum_split` are byte-checked at 100 000 levels and oracle-checked at the scales the recursive oracle survives.
- `diff_block_scan_never_exceeds_the_complement_walk` (tests.rs:910-963) derives its floor (every operand tag once) and ceiling (plus the settled output once plus a named `BLOCK_SLACK`) from the mechanism rather than from a measurement, and states why a re-derived block would land far past it.
- `From<Party> for [Party; N]` rejects `N == 0` at monomorphization with a `const` assert and pins it with a paired `compile_fail,E0080` doctest against its compiling twin (forks.rs:164-172, 190). `Open` is `!Clone` and `#[must_use]` (build.rs:39-40); the doc overclaims what that buys (party-17), but the discipline itself is right.
- `Party::seed` documents the `static`-not-`const` choice at the declaration (party.rs:122-130): the one thing the code cannot show, said in five lines.

## Open questions for Finch

1. **The parity-halves search floor is a measured reading × 0.75** (tests.rs:1139-1147, `SEARCH_SCAN_FLOOR_BITS = 101_397`), following the envelope suite's stated convention (tests/meter.rs:6397-6398), while the board follows your derived-floor ruling. A galloping search from the left edge (about 2·log₂(left-subtree entries) + 1 probes per node) would do strictly less work and trip this floor; so would party-26's bounded search. Recommendation: derive the floor from the mechanism in the test (on the parity halves every skeleton node is both-present, so at least `2^d − 1` searches of at least one 32-bit probe each: `(2^d − 1) × 32` plus the tag reads, about 38 876 bits at `d = 10` against the 6 140-bit unmetered reading), and decide separately whether the envelope suite's × 0.75 convention should stay confined to `tests/meter.rs`. Not filed as a finding because the convention is recorded at the site and is yours to rule on.
2. **Should `split`'s raw spine walk and the `split`/`sum_split` copies be metered** (party-27's option (a): route through the shared tag helper and a `PackedBuilder`-shaped copy, re-pinning `ID_FORK`'s scan column from 3 to a spine-proportional number, and moving the `spliced == 8` constant), or stay raw with the exemption stated at the code (option (b), what party-27 resolves to)? The answer also decides party-34's fix (meter the fused side vs. compare reads only). Recommendation: (a), because `IdReader::read` plus `pos()` walks the same spine at the same cost and the "deliberately raw" record reads as an accident later ratified by a pin; but it is a re-pin, so it is your call.
3. **Three public-doc passages were replaced or removed in your 2026-08-04..08-07 doc passes** (a431eaf1, b3f09baa0) without messages, and in each case the earlier agent-written text is the correct reference for the repair: the `forks` saturation clause (party-14; cdad46060's wording), `Party::decode`'s Party-specific warning (party-7; e546b6d5e's text), and `join_all`'s coalescing clause in `# Errors` (party-8; 4f12b8218's text). Were the removals deliberate concision, or collateral? Recommendation: restore the substance of all three in your own words; the pin in `tests/forks_max.rs` and the laws roster both describe behavior the current prose contradicts.
4. **The `>2^32`-bit fold-index fallback** (party-23): widen the table (`Narrow`/`Wide` enum, or `Vec<u64>` if the `party_join_all` heap cell tolerates it) and dissolve `build_unindexed` and the fallback differentials, or carry the size clause in the island contract? Recommendation: widen; the crate's "for all input sizes" sentence is the one you chose to make load-bearing, and the fallback arm exists only to be held by tests that would then go away.
5. **`ExactSizeIterator::len` on 32-bit targets** (party-13): document a `# Panics`, override `len()` to saturate, or drop the impl? Recommendation: override `len()` to saturate at `usize::MAX` with one sentence saying so, and add the wasm32 pin; it keeps the public surface and removes the only caller-reachable panic in the partition.
6. **`IdIndex`'s per-ancestor state in machine words** (party-24) and the two `IdLeafCursor`s (party-20): the first is an exemption from the "depth costs bits" discipline nothing states; the second is a recorded drop nothing in the tree states. Recommendation: fix the first (the `u32` fit is already proven by the table guard) and state the second at diff.rs:229-244; if you would rather reopen the merge, the refutation pass confirmed a lazy-entry face on `overlay::IdLeafCursor` is additive for every eager client.

## Dropped

- [7] item 3, `IdReader::at`'s doc "see `split`'s `build_split`" is dangling: disputed by the refutation and by my read; `subtree_end` (split.rs:115-118) is `build_split`'s helper and does call `IdReader::at`, so the pointer is imprecise, not wrong. Below the bar.
- [7] item 4, overlay.rs:17's "two cursor instances" is a wrong count: module-scoped and deliberate (c6ba2208 trimmed the crate-wide roster); folded into party-20 as a scoping clarification.
- [29]/[44], the parity-halves floor is measured × 0.75: the rationale is recorded at the site and the convention is suite-wide; moved to open question 1.
- [40], `NA_SCAN_SEED_PARTY` says the seed's packed form is empty: the anchor (meter/board/floors.rs:143-145) is outside this partition. Route to the board partition; the false premise is real (the seed is the 2-bit terminal `00`, party.rs:122-123 and the doctest at 593-594; the anonymous id is the empty stream, party.rs:643-644).
- [38], [52]: duplicates of [0], merged into party-20.
- [43], [53]: duplicates of [3], merged into party-19.
- [27]'s `finish_id` half: duplicate of [8], merged into party-10.
- [24]: duplicate of [13], merged into party-5.
- [31]: duplicate of [15], merged into party-32; [15]'s bundled aliased-inputs item is party-31.
- [35]: duplicate of [19], merged into party-14.
- [39], [46]: duplicates of [18], merged into party-22.
- [41]: duplicate of [21]'s split half and [7]'s item 2, merged into party-3; [21]'s compare.rs half is in party-10.
- [42], [50]: merged with [11] into party-28 (one suffix precondition, three faces).
- [57]: merged with [2] into party-27 (the pointer and the exemption share one fix).
- [56]: merged into party-2 (a caller roster that drifted).
- [22], [25], [60]: merged into party-7 (public rustdoc slips in one file).
- [9], [12], [14]: merged into party-6 (idiom nits).
- [13]/[24], [32], [33]: merged into party-5 (register sweep).
