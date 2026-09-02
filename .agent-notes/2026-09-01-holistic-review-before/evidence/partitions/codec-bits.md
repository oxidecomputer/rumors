# Partition codec-bits: The bit-level substrate: bits, buf, build, code, cursor, dsi, gamma, int, literal, scan, stack

## Partition summary

This partition is the packed-bit kernel every coding in `before` stands on. `bits.rs` holds the frozen storage form (`Bits`, a marker-padded `Bytes` whose byte string is injective on bit streams, so `Eq` is one `memcmp` and `len` one `trailing_zeros`) and the borrowed `BitsView`; `buf.rs` holds the mutable `BitsBuf` under two invariants (exact bytes, zeroed dead bits) and the `seal_padding` step at the freeze; `build.rs` is `PackedBuilder`, the append/reserve-patch/splice/truncate move set the id and skyline emitters write through, with the scan meter applied at its primitives; `code.rs` and `int.rs` keep narrow payload codes and decoded integers in machine words end to end; `cursor.rs` defines `BitCursor` and the per-bit `SliceCursor`; `dsi.rs` is the word-parallel `DsiCursor` over `dsi-bitstream`'s buffered reader, with the accept/reject boundary kept in the wrapper; `gamma.rs` is the Elias-gamma code with a one-window fast path whose contract is that the per-bit loop remains the sole arbiter of every reject; `literal.rs` builds id streams for the `Party` literal door; `scan.rs` is the process-global scan meter hook; `stack.rs` is the word-backed `BitStack` and the bit-priced `PopStack` the deep walks hold their paths on. I read all fourteen files in full (2995 lines; `dsi/tests.rs` and `stack/tests.rs` are the two test files), plus the sibling `codec/tests.rs` ranges the findings cite, `codec/tree.rs`, the consumers in `party.rs`, `version.rs`, `clock.rs`, `borsh_impls.rs`, `version/skyline/{build,masked,overlay,query,signed}.rs`, the wasm32 pins, and the pinned sources of `dashu-int` 0.5.0, `bytes` 1.11.1, and `dsi-bitstream` 0.10.1.

The substrate is in good shape where it matters most. On 64-bit targets I found no reachable panic: every `expect` message is a one-line proof I could discharge, the window decoder declines every case it cannot prove, marker padding plus the one-bit-minimum grammars make the stored bytes injective, and the `BitsBuf` build-history family and the `DsiCursor` differential suite pin the two representations and the two readers against each other at every cut point. The prose is accurate at the contract level, with `# Panics` sections matching their asserts and the maintainer-facing "why" present where the code cannot show it (phantom zeros, the truncation-reject record, the `Truncated` ZST).

The dominant issues are three. First, a duplicated substrate: since the byte-backed `BitsBuf` landed, `PackedBuilder` keeps a second byte store plus a sub-byte staging register and reimplements six primitives `BitsBuf` already carries, at the price of two invariant sets and a per-bit copy in `extract_code`'s wide arm; the register's saving is a hypothesis nobody has measured. Second, verification gaps at the kernels: `PackedBuilder` and `BitsView::load_be` have no direct model test, and the `BitStack` differential names a method that does not exist, omits `set_last` and `trailing_ones`, and reaches the word spill it advertises with probability 4.0e-5 per case (computed exactly), so the multi-word `trailing_ones` loop appears to run under no oracle-checked test. Third, two claims contradicted by the code: `peek_flip` re-scans a parked cursor's trailing run on every masked or projection step, a term the derived linear bound omits and no meter sees; and on 32-bit targets the wide-gamma width guard admits 32 values past dashu's capacity cap, so a decoded stream can panic inside the backend where the comment promises a reject. The rest is prose: a false sentence about the builder wrapping a `BitsBuf`, a stale consumer roster in `cursor.rs`, ghost references to the retired `store_be` path, a drifted record-site roster in `scan.rs`, a mislabelled `k = 65` witness, and vocabulary the crate never anchors.

## Findings

### codec-bits-1: The identity-ladder essay decides for operations outside the module, without saying it is their home
- Where: crates/before/src/codec/bits.rs:8-55 (related: crates/before/src/version.rs:93, 431, 995, 1006, 1225)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (read bits.rs:8-55; `grep -n '\brung' crates/before/src/version.rs` shows the sites stating their own choice; `git log -1 306e2de0` message); executed: no
- Seen by: structure [6], prose [19]; refutation: confirmed; history: deliberate-and-holds (306e2de0: "The codec::bits module doc now carries the whole ladder rationale (which rung belongs where, and why)"), the rationale lives only in history
- Owner-gated: yes (a recorded design decision on where the policy lives)

The storage module's doc names which operations in `version.rs` and `party.rs` take which rung and says each site states its choice; the sites do (version.rs:93, 431, 995, 1006, 1225), so the per-operation outcomes are written twice with no mechanical tie between the copies, and nothing in `bits.rs` says the essay is the designated home. A rung adopted or dropped at a call site leaves the essay wrong with nothing to catch it (Principle 5: enumerations the code can change without touching the prose; documentation altitude: a module states its own contract).

Evidence:

        14	//! walking. The decision rule, applied per call site (each site cites its law
        15	//! and states its choice):
    ...
        27	//!   arithmetic and allocating walks take it: join/meet/span (an
        28	//!   emission plus its buffers), `distance`/`lag` (accumulator folds
        29	//!   and `Base` products), `Ranked`'s total order (the rank co-sweep).

Resolution: Either state in the essay's first paragraph that it is the ladder's single home for rung policy and reduce the call sites to a pointer ("rung choice: see `codec::bits`"), or keep the bullets as criteria only (free insurance; pays where the replaced walk is expensive and equality is common; not where the fallback is itself a cheap scan and unequal is the common case; not on linear predicates) and drop the named operations, letting each site carry its rationale as today. Acceptance: the per-operation rung rationale appears exactly once in the crate, and wherever it lives names itself as the home.

### codec-bits-2: The ladder essay's "order of magnitude" ratio is a number no instrument measures
- Where: crates/before/src/codec/bits.rs:22-29 (related: crates/before/src/codec/bits.rs:426-428; crates/before/tests/meter.rs:10552)
- Class / severity / confidence: claim / nit / medium
- Provenance: verified for the site (`grep -rn 'order of magnitude' crates/before/src` finds bits.rs:25 and the `canonical_hash` doc at 426-428 only); assessed for the instrument (the `identity_fast_paths` module pins zero walk work, not a ratio); executed: no
- Seen by: claims [40]; refutation: confirmed; history: already-known (3d976922 de-numbered `canonical_eq`/`canonical_hash` and deliberately kept `canonical_hash`'s phrase with a mechanism clause; the essay's phrase predates that commit and was not swept)
- Owner-gated: no (the `canonical_hash` phrase is the ruled site and is left alone here)

The rule deciding which sites take the `memcmp` rung rests on "roughly an order of magnitude cheaper per bit than a decoding walk"; the two named instruments pin zero walk work on adopted rungs and measure no ratio. A number in prose is a hypothesis unless it names its measurement, and 3d976922's own reasoning ("a plausible code change could falsify either without any committed check firing") applies here.

Evidence:

        22	//! - **The `memcmp` rung pays for itself exactly where the walk it
        23	//!   replaces is expensive relative to a byte scan.** A miss costs an
        24	//!   early-exiting byte compare over the operands' shared prefix —
        25	//!   byte-parallel, roughly an order of magnitude cheaper per bit than
        26	//!   a decoding walk — and a hit deletes the walk whole. The

Resolution: Restate as mechanism ("byte-parallel, with no decode per bit") or cite a bench cell comparing a `canonical_eq` miss to `causal_cmp` on byte-equal operands. Acceptance: the sentence makes no quantitative claim or names a committed measurement.

### codec-bits-3: Register tells: "honest", "genuinely", and "real" where the anchored term is "live"
- Where: crates/before/src/codec/bits.rs:50-50 (related: crates/before/src/codec/int.rs:10; crates/before/src/codec/buf.rs:57; crates/before/src/codec/cursor.rs:58; crates/before/src/codec/gamma.rs:162, 214; crates/before/src/codec/stack.rs:40; crates/before/src/codec/stack/tests.rs:6)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n 'honest\|genuinely\|\breal\b'` over the partition files returns exactly these sites); executed: no
- Seen by: prose [24]; refutation: confirmed; history: no-rationale-found (the owner's writing doctrine: where "real", "genuine", or "honest" beckons, write the property instead)
- Owner-gated: no

"hold the ladder honest" moralizes an instrument; "genuinely wide" is a significance adverb; "real memory", "real stream", "real input", "real value bit" contrast with phantom or padding bits, for which the crate's anchored word is "live" (`BitsView::live`, "live bits" throughout).

Evidence:

        50	//! Two instruments hold the ladder honest: the `identity_fast_paths` pins in
    (int.rs)
        10	//! this form; a consumer with genuinely wide arithmetic converts through
    (gamma.rs)
       162	    // Bits of real stream between `pos` and the window's end.

Resolution: "Two instruments pin the ladder"; "a consumer with wide arithmetic"; "live stream" / "live input" / "allocated memory" at the "real" sites. Acceptance: the grep above returns nothing in `crates/before/src/codec`.

### codec-bits-4: Em-dashes in `//` comments, and a doc line broken mid-clause
- Where: crates/before/src/codec/bits.rs:60-60 (related: crates/before/src/codec/cursor.rs:124-125; crates/before/src/codec/dsi.rs:229-230, 253, 257, 259, 296-298; crates/before/src/codec/gamma.rs:35-36, 186-187, 246)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—'` over the partition files: thirteen lines, exactly those listed); executed: no
- Seen by: prose [23]; refutation: confirmed; history: no-rationale-found (the comments predate the doctrine's capture; no gate leg checks dash register)
- Owner-gated: no

Thirteen `//` comment lines use true em-dashes where the doctrine's register for code comments is the spaced double-hyphen, and dsi.rs:296-298 breaks a doc sentence mid-clause, an editing artifact.

Evidence:

        60	// code spans — the items are private — which this allow accepts.
    (dsi.rs)
       296	/// arrives with its dead bits already masked, the view's `body_tail`
       297	/// destructuring), which
       298	/// parallels the slice cursor's zero-filled decode window: the phantom zeros

Resolution: Replace the em-dashes in `//` comments with ` -- `; reflow dsi.rs:295-298. Acceptance: the grep above is empty over `crates/before/src/codec`.

### codec-bits-5: Door, seam, gate: three unanchored words for one boundary; "denomination" and "currency" each carry two senses
- Where: crates/before/src/codec/bits.rs:107-119 (related: crates/before/src/codec/bits.rs:5-6, 90, 135; crates/before/src/codec.rs:46; crates/before/src/codec/buf.rs:26, 30; crates/before/src/codec/cursor.rs:44, 93; crates/before/src/codec/dsi.rs:61, 68-70; crates/before/src/codec/stack.rs:4-6, 39, 192; crates/before/src/clock.rs:836; crates/before/src/meter/board/currency.rs)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (per-file counts over the partition: "door" 16, "seam" 16, "gate" 1; `grep -in door crates/before/src/lib.rs` is empty; the "denominat" sites read); executed: no
- Seen by: prose [20]; refutation: confirmed, with a correction to the evidence (the non-codec "denominate" sites read "denominate readings in exact encoded bit lengths", the unit-of-measure sense, not bytes-per-cost); history: no-rationale-found (no definition site exists anywhere; the vocabulary predates the writing doctrine's capture)
- Owner-gated: yes (crate-wide vocabulary; the definition site is the owner's call)

`Bits::freeze` is "the seam" (5-6), "the single gate" (108-109), and "the door" (119) within one file; neither "door" nor "seam" is anchored to an identifier or defined by contrast anywhere in the crate. "denomination" in this partition means the integer width a count is expressed in (bits.rs:90, 135; buf.rs:26, 30; cursor.rs:44, 93; dsi.rs:61; stack.rs:39), while clock.rs:836 and its siblings use it for the unit a reading is expressed in; "currency" at stack.rs:5 and 192 means the unit a transient is priced in, while `meter/board/currency.rs` names one deterministic meter. The vocabulary rule: a coined term is an identifier or is defined once by contrast; a reader meeting three words for one thing must decide whether they differ.

Evidence:

       107	    /// Freeze a built stream into the at-rest form, canonicalizing its storage:
       108	    /// the single gate between the mutable build-side world and the shared
       109	    /// frozen one.
    ...
       119	    /// buffer is allocatable — the door imposes no bound of its own.
    (bits.rs:5-6)
         5	//! build-side form lives in the sibling `buf` module; [`Bits::freeze`] is
         6	//! the seam between the two.

Resolution: Define "door" and "seam" once by contrast in `codec.rs`'s module doc (a door admits untrusted bytes or text into a stored value and owns their validation: decode, parse, literal; a seam is a hand-off between two in-crate representations at which an invariant is established: freeze, `built_view`, `into_base`); retire "gate" in the boundary sense (`just gate` already owns the word). Write "width" or "`u64`" for the integer-width sense of "denomination" and "unit" for stack.rs's "currency". Acceptance: `codec.rs` defines the two terms; "gate" as a boundary noun does not appear in the partition; "denominat" in `codec/` refers to a unit of measure or does not appear.

### codec-bits-6: The u64-width refrain is restated at entries that neither convert nor compute, and the read_gamma rejection is stated twice
- Where: crates/before/src/codec/bits.rs:117-119 (related: crates/before/src/codec/bits.rs:135-138, 173-175; crates/before/src/codec/build.rs:230-232; crates/before/src/codec/cursor.rs:44-45; crates/before/src/codec/gamma.rs:150-152; crates/before/src/codec/dsi.rs:14-18, 205-208; crates/before/src/codec/buf.rs:23-34)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read every restatement site and checked its body for width arithmetic or a `usize` conversion); executed: no
- Seen by: prose [13]; refutation: confirmed; history: deliberate-and-holds in part (7ea3df58 and 05d87e1b record the per-site ledger, and the owner's writing doctrine restates a breakable rationale at every site capable of breaking it), so the finding narrows to sites with nothing to break
- Owner-gated: no

"Exact at every size on every target" is restated at `freeze` (117-119), `from_canonical` (135-138), `live` (173-175), `finish` (230-232), the trait's `position` (44-45), and `decode_int_window` (150-152), whose bodies hold no width arithmetic and no `usize` conversion: the signature already says `u64`, and the argument's homes (buf.rs "# Widths", and the arithmetic sites such as `Bits::len`, `BitsBuf::live`, `PackedBuilder::len`, `read_unary`) carry it. `dsi.rs` states why `read_gamma` is refused at 14-18 and again at 205-208. Documentation altitude: never document what the types prevent; every sentence competes with the contract the reader came for.

Evidence:

       117	    /// Exact at every size on every target: lengths and positions are `u64`
       118	    /// on both sides of this seam, so an emission is storable whenever its
       119	    /// buffer is allocatable — the door imposes no bound of its own.
    ...
       135	    /// Exact at every size on every target: the stored form denominates its
       136	    /// bit positions in `u64` ([`len`](Self::len), [`live`](Self::live)), so
       137	    /// any buffer the validator admits is adoptable whole — the door imposes
       138	    /// no bound of its own.
    (dsi.rs)
       205	    /// - the composed unary-prefix + mantissa read for machine-word
       206	    ///   codes (`k < 64`) — `dsi-bitstream`'s own `read_gamma` is
       207	    ///   unusable here because its supported range caps at `u64` while
       208	    ///   this coding has no value cap;

Resolution: Delete the refrain at the six non-arithmetic sites, or reduce each to one clause pointing at buf.rs "# Widths"; keep the ledger where a conversion or wrap-freedom argument sits (bits.rs:156-159, buf.rs:56-58, build.rs:43-47, cursor.rs:58-60, dsi.rs:56-59 and 113-115, gamma.rs:214-216, scan.rs:63-65, stack.rs:39-42). Reduce dsi.rs:205-208 to "(`read_gamma` is refused: module doc)". Acceptance: `grep -rn 'every size on every target' crates/before/src/codec` hits no entry whose body has no `as u64`, `usize::try_from`, or width arithmetic; the `read_gamma` rejection is argued once in dsi.rs.

### codec-bits-7: ptr_eq's doc misstates why independently frozen empty streams alias
- Where: crates/before/src/codec/bits.rs:209-211 (related: crates/before/src/codec/tests.rs:168-172, 183-188; bytes-1.11.1 src/bytes.rs:138-143, 960-971, 995-1002)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read `bytes` 1.11.1: `From<Vec<u8>>` routes `len == cap` through `From<Box<[u8]>>`, which returns `Bytes::new()`, a static `EMPTY` slice, for an empty slice; a vector with `len != cap` takes the `Shared` path and keeps its own pointer); executed: no
- Seen by: prose [18]; refutation: confirmed, adding the test doc at codec/tests.rs:168-172 as a second site; history: no-rationale-found (f9973730 asserted the mechanism in its message and doc)
- Owner-gated: no

The pointer two empty freezes share is `Bytes::new()`'s static empty slice, not a dangling zero-byte allocation; and only a capacity-free empty vector reaches it. `Bits::freeze(BitsBuf::with_capacity(n))` for `n > 0` on an empty buffer hands `Bytes::from` a vector with `len 0 != cap`, which keeps its own heap pointer, so two such freezes read `ptr_eq` false. The safety conclusion (a rung derives only equality) holds; the mechanism a maintainer would reason from is wrong on both counts (statement faithfulness).

Evidence:

       209	    /// provenance is the *production* source of sharing, not the predicate's
       210	    /// meaning: every zero-byte allocation carries the same dangling pointer,
       211	    /// so two independently frozen **empty** streams also read `ptr_eq` true. A
    (codec/tests.rs)
       168	/// substitute for it. The empty stream is the deliberate exception the
       169	/// predicate's docs carry: every zero-byte allocation shares one dangling
       170	/// pointer, so two independent empty freezes read `ptr_eq` true *without* clone

Resolution: Rewrite both sites: `Bytes::new()` (which `Bytes::from` of a capacity-free empty vector reaches) shares one static empty slice, so independently frozen empty streams *may* read `ptr_eq` true; clone provenance is therefore not what the predicate certifies, only value equality. Acceptance: both sentences use "may" and name the shared static; no claim about dangling pointers remains.
Construction: in `codec/tests.rs`, `let e1 = Bits::freeze(BitsBuf::with_capacity(8)); let e2 = Bits::freeze(BitsBuf::with_capacity(8)); assert!(e1.ptr_eq(&e2));` fails under `bytes` 1.11.1 (two live one-byte allocations have distinct pointers), while the committed test's `BitsBuf::new()` pair passes.

### codec-bits-8: require_marker_padding at pos 0 accepts `[0x80]`, which padding_is_canonical rejects
- Where: crates/before/src/codec/bits.rs:490-506 (related: crates/before/src/codec/bits.rs:446-453, 139-146; crates/before/src/party.rs:630-631; crates/before/src/version.rs:1117-1118; crates/before/src/clock.rs:801-816; crates/before/src/span/wire.rs:147, 155)
- Class / severity / confidence: correctness / nit / high
- Provenance: assessed (traced the arithmetic; read all six callers, each of which parses at least one production before judging the padding); executed: no
- Seen by: correctness [30], claims [35]; refutation: confirmed, 35 lowered to nit (unreachable from every caller); history: no-rationale-found (the disagreement dates from d800957e's slice-form judge and survived 78c65371's byte-form rewrite)
- Owner-gated: no

For `bytes = [0x80]`, `pos = 0`: `total = 8`, `remainder = 8`, `mask = 0xFF`, `0x80 == 1 << 7`, so the judge returns `Ok(())`; `padding_is_canonical` names `[0x80]` non-canonical (449) and `from_canonical` debug-asserts on it. The two judges agree in production only because every door's grammar consumes a bit before the padding judge runs, a premise that lives in the callers, not at the judge whose doc claims "every stream has exactly one padded spelling" over any `pos <= total`.

Evidence:

       492	        1..=8 => {
       493	            // The remainder lives entirely in the final byte: its low
       494	            // `remainder` bits must be a `1` followed by zeros.
       495	            let last = bytes[bytes.len() - 1];
       496	            let mask = if remainder == 8 {
       497	                0xFF
       498	            } else {
       499	                (1u8 << remainder) - 1
       500	            };
       501	            if last & mask == 1 << (remainder - 1) {
       502	                Ok(())

Resolution: Either add an arm rejecting the corner (`pos == 0 && !bytes.is_empty()` is `TrailingBits`, with the doc sentence "the empty stream's only spelling is the empty buffer"), or narrow the contract to `pos >= 1 || bytes.is_empty()` under `# Panics` with a `debug_assert!`. Either way add the totality law to `codec/tests.rs`: for random `(bytes, pos)`, `require_marker_padding(b, p).is_ok()` implies `padding_is_canonical(&Bits::from_canonical(b))`. Acceptance: `require_marker_padding(&[0x80], 0)` is no longer `Ok(())`, and the proptest is committed.
Construction: `assert!(require_marker_padding(&[0x80], 0).is_ok())` passes today; `padding_is_canonical(&Bits::from_canonical(Bytes::from_static(&[0x80])))` trips the debug assertion, and in release yields a `Bits` with `len() == 0` that is byte-unequal to `Bits::empty()`.

### codec-bits-9: BitsBuf's type doc says the packed-stream builder wraps a BitsBuf; it does not
- Where: crates/before/src/codec/buf.rs:41-43 (related: crates/before/src/codec/build.rs:48-58, 233-240)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the `PackedBuilder` struct and `finish`); executed: no
- Seen by: structure [1]; refutation: confirmed; history: no-rationale-found (the sentence and the builder that contradicts it landed together in 83e61b4d)
- Owner-gated: no

`PackedBuilder` (build.rs:48-58) has fields `bytes: Vec<u8>`, `staged: u64`, `staged_len: u32` and constructs a `BitsBuf` only at `finish` via `from_raw_parts` (233-240). The parenthetical tells a maintainer to look for `BitsBuf` methods to explain builder behavior that lives in a parallel implementation (Principle 5: prose states what is). Independently actionable whether or not codec-bits-12 is taken.

Evidence:

        41	/// Every emitter and builder writes into one of these (the crate's
        42	/// packed-stream builder wraps one with the metered move set); a finished
        43	/// stream freezes into the at-rest `Bits` at the storage seam. The module doc

Resolution: If codec-bits-12 lands, the sentence becomes true as written. Otherwise re-state: "the packed-stream builder hands its finished bytes to one at `finish`", or delete the parenthetical. Acceptance: the sentence describes the builder's actual relationship to `BitsBuf`.

### codec-bits-10: with_capacity docs promise a past-address-space hint "allocates nothing up front", which holds only where usize::try_from fails
- Where: crates/before/src/codec/buf.rs:68-78 (related: crates/before/src/codec/build.rs:61-72)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read both bodies; `usize::try_from::<u64>` is total on 64-bit targets, and `Vec::with_capacity` panics on capacity overflow or aborts on allocation failure); executed: no
- Seen by: claims [41]; refutation: confirmed (every production hint is a sum of live lengths, so no input reaches the corner); history: no-rationale-found (83e61b4d wrote both sentences for the 32-bit boundary it cured)
- Owner-gated: no

On a 64-bit target the `unwrap_or(0)` never fires, and an unallocatable request panics or aborts rather than allocating nothing; even on 32-bit a byte count in `(isize::MAX, usize::MAX]` panics with a capacity overflow. The sentence overstates the mechanism.

Evidence:

        68	    /// An empty buffer with room for `bits` bits before reallocation.
        69	    ///
        70	    /// The capacity is a hint: a request past the target's address space
        71	    /// allocates nothing up front, and the buffer still grows to whatever
        72	    /// the pushes actually demand.
        73	    pub(crate) fn with_capacity(bits: u64) -> Self {
        74	        BitsBuf {
        75	            bytes: Vec::with_capacity(usize::try_from(bits.div_ceil(8)).unwrap_or(0)),

Resolution: State what the code does ("a request that does not fit `usize` allocates nothing up front; callers pass hints bounded by their operands' live lengths"), or clamp the hint if the no-op semantics are wanted. Acceptance: both sentences match the code's behavior on both target widths.

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

### codec-bits-12: PackedBuilder reimplements BitsBuf's append-truncate substrate behind a staging register whose saving is unmeasured
- Where: crates/before/src/codec/build.rs:48-58 (related: crates/before/src/codec/build.rs:102-106, 144-167, 175-200, 208-228, 245-266, 270-282, 284-325; crates/before/src/codec/buf.rs:93-102, 142-150, 185-214, 230-239, 259-273, 346-369; crates/before/src/codec/code.rs:42-59)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (compared each primitive pair line by line; `grep -rln PackedBuilder`: the wrappers are party/ops/build.rs and version/skyline/build.rs; `from_raw_parts` has one caller, build.rs:239; `git show 83e61b4d`); executed: no
- Seen by: structure [0]; refutation: confirmed with one correction (the register is a live design choice: `BitsBuf::push_bits` pops and re-pushes the partial tail byte on every append, buf.rs:198-211, which the register avoids on sub-byte appends; 83e61b4d kept it deliberately without a measurement, so the sign is not fixed); history: deliberate-but-expired (525e7324 introduced the register against the `bitvec` buffer; 83e61b4d gave `BitsBuf` the same byte-backed representation and recorded keeping the register with no reason: "PackedBuilder keeps its staged-register internals and word-parallel moves")
- Owner-gated: no (crate-private; acceptance runs the bench judge)

`PackedBuilder` owns a `Vec<u8>` plus a sub-byte `staged`/`staged_len` register and reimplements every primitive `BitsBuf` provides: `append_bits` (245-266) is the same u128-merge/`to_be_bytes`/extend body as `push_bits` (buf.rs:185-214); `append_bytes` (270-282) is `extend_bytes` (buf.rs:259-273); `splice` (175-200) is `extend_from_view` (buf.rs:346-369) plus a meter tap; `truncate` (208-228) is `BitsBuf::truncate` (buf.rs:230-239) with a committed/staged split; `patch_bit` (144-167) is `BitsBuf::set` (buf.rs:142-150) with the same split; `read_bits`/`bit_at` (284-325) exist only because the staged bits are outside `bytes`, so the builder cannot hand out a `BitsView` of itself, which is why `extract_code`'s wide arm copies one bit at a time where `Code::from_range` is byte-parallel. Two invariant sets and two-branch readers for one discipline (Principle 3: machinery outlives the constraint that justified it; legibility).

Evidence:

        48	pub(crate) struct PackedBuilder {
        49	    /// The committed prefix: whole bytes, most-significant bit first.
        50	    bytes: Vec<u8>,
        51	    /// The trailing not-yet-committed bits, value-packed at the low end
        52	    /// (the stream's next bit is the register's most significant live
        53	    /// bit). Always fewer than eight: appends flush whole bytes
        54	    /// greedily.
        55	    staged: u64,
        56	    /// Live bits in `staged`, `0..8`.
        57	    staged_len: u32,
        58	}
    (build.rs:259-262 beside buf.rs:204-205)
       259	        let acc = (u128::from(self.staged) << len) | u128::from(value);
       260	        let rem = total % 8;
       261	        let whole = (total / 8) as usize;
       262	        let aligned = (acc << (128 - total)).to_be_bytes();
       204	        let acc = (u128::from(staged) << len) | u128::from(value);
       205	        let aligned = (acc << (128 - total)).to_be_bytes();
    (build.rs)
       102	        let mut out = BitsBuf::with_capacity(n);
       103	        for i in start..start + n {
       104	            out.push(self.bit_at(i));
       105	        }
       106	        Code::Wide(out)

Resolution: Construct `PackedBuilder { out: BitsBuf }` as the metered move set over the one build buffer: `push_bit` = record + `out.push`; `push_code` Small = record + `out.push_bits(bits, len)`, Wide = `splice`; `reserve(w)` = record + `out.push_bits(0, w)`; `patch_bit` = record + `out.set`; `splice` = record + `extend_from_view`; `truncate` = `out.truncate`; `extract_code(start)` = record + `Code::from_range(built_view(&self.out), start, self.len())`; `finish` = `self.out`. Delete `append_bits`, `append_bytes`, `read_bits`, `bit_at`, and `BitsBuf::from_raw_parts`. Measure at the parent and at the change on a quiet machine (`just bench-judge`); if the register measurably wins, invert the direction and give `BitsBuf` the register form so one implementation serves both wrappers. Acceptance: one implementation of the byte-backed append/truncate/patch discipline exists in the crate; `just test-all` green; `tests/meter.rs` envelopes and scan floors unchanged; the bench judge within band with both numbers recorded in the commit; buf.rs:41-43 becomes true.

### codec-bits-13: extract_code copies wide codes one bit at a time, and has no guard for an empty range
- Where: crates/before/src/codec/build.rs:93-107 (related: crates/before/src/codec/build.rs:24-33; crates/before/src/codec/code.rs:16, 42-59; crates/before/src/version/skyline/build.rs:333-334)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read `extract_code`, `Code::from_range`, and the cascade caller); executed: no
- Seen by: claims [36] (the per-bit wide arm); refutation: confirmed, and raised the `n == 0` corner as new; history: no-rationale-found (the per-bit arm is unchanged since 525e7324; code.rs frames wide codes as the cold path but the module doc promises word ops for every primitive)
- Owner-gated: no

For `n > SMALL_CODE_BITS` the read-back is `out.push(self.bit_at(i))` per bit, each a committed/staged branch plus a `BitsBuf::push`, against the module doc's "bit-granular work costs word ops, not one buffer operation per bit" (26-30); it runs on the join/meet cascade (skyline/build.rs:334) whenever the kept left leaf's code is wide. Linear, so a fixed-sign constant-factor deletion, not an asymptotic breach. Separately, `n == 0` (`start == self.len()`) returns `Code::Small { bits: 0, len: 0 }`, outside `Code::Small`'s documented `1..=63` (code.rs:16) and the precondition `Code::from_range` debug-asserts (`start < end`, code.rs:43); unreachable today (`lens.pop()` is a code length of at least one bit) but the two constructors disagree on the empty-range contract.

Evidence:

        93	    pub(crate) fn extract_code(&self, start: u64) -> Code {
        94	        let n = self.len() - start;
        95	        super::scan::record_bits_u64(n);
        96	        if n <= SMALL_CODE_BITS {
        97	            return Code::Small {
        98	                bits: self.read_bits(start, n as u32),
        99	                len: n as u8,
        100	            };
        101	        }
        102	        let mut out = BitsBuf::with_capacity(n);
        103	        for i in start..start + n {
        104	            out.push(self.bit_at(i));
        105	        }
        106	        Code::Wide(out)

Resolution: Read in `<= 63`-bit chunks with `read_bits` and append with `BitsBuf::push_bits`; or, after codec-bits-12, `Code::from_range(built_view(&self.out), start, self.len())`. Add `debug_assert!(n > 0, "a payload code is never empty")` matching `from_range`. Acceptance: no per-bit loop remains in `extract_code`; the wide-payload envelopes in `tests/meter.rs` read unchanged or lower on heap with scan bits identical.

### codec-bits-14: reserve loops over 32-bit chunks for a width that is always 2
- Where: crates/before/src/codec/build.rs:127-137 (related: crates/before/src/party/ops/build.rs:43, 47, 76)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`grep -rn '\.reserve('` over crates/before/src: the one `PackedBuilder` caller is party/ops/build.rs:76 with `const TAG_BITS: usize = 2`); executed: no
- Seen by: structure [8]; refutation: confirmed (notes `TERMINAL_PAIR_BITS` at party/ops/build.rs:47 casts from `TAG_BITS`); history: no-rationale-found (525e7324 rewrote c0e06dcd's per-bit loop as the 32-chunk loop with no stated reason)
- Owner-gated: no

The loop chunks at 32 while `append_bits` accepts 63, for a width no caller has ever set to anything but 2; the `usize` width is also the builder's one non-`u32` bit count. Generality with no consumer, and a magic 32 that asks the reader to imagine wide header slots no coding has.

Evidence:

       127	    pub(crate) fn reserve(&mut self, width: usize) -> u64 {
       128	        super::scan::record_bits(width);
       129	        let at = self.len();
       130	        let mut remaining = width;
       131	        while remaining > 0 {
       132	            let chunk = remaining.min(32);
       133	            self.append_bits(0, chunk as u32);
       134	            remaining -= chunk;
       135	        }
       136	        at
       137	    }

Resolution: `reserve(width: u32)` with `debug_assert!(width <= 63)` and a single `append_bits(0, width)` (or `push_bits` after codec-bits-12); `TAG_BITS` becomes `u32` and `TERMINAL_PAIR_BITS`'s cast follows. Acceptance: `reserve` has no loop; the party differential suites pass.

### codec-bits-15: PackedBuilder, the kernel every emitter writes through, has no direct model test; neither does BitsView::load_be
- Where: crates/before/src/codec/build.rs:208-228 (related: crates/before/src/codec/build.rs:144-167, 245-266, 270-282, 286-312; crates/before/src/codec/bits.rs:321-343; crates/before/src/codec/tests.rs:327-395; .cargo/mutants.toml; tools/covcheck-expected.json:2)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rln PackedBuilder` over src, tests, fuzz, wasm32-pins, examples names no test file; `grep -rn load_be` finds the definition and code.rs:51 only; `grep -i codec .cargo/mutants.toml tools/mutantcheck-expected.json` is empty; covcheck's scope is `crates/before/src/version/skyline/`); executed: no
- Seen by: correctness [26], claims [34]; refutation: confirmed; history: no-rationale-found (525e7324 pinned the builder transitively by design; 83e61b4d added the build-history model family for `BitsBuf` only)
- Owner-gated: no

The builder's move set carries the partition's most intricate bit arithmetic (`truncate` re-deriving `staged` from a committed byte at 216-223; `read_bits` chunking across the committed/staged boundary at 292-310; the `append_bytes` carry at 275-282; `patch_bit` in two regimes at 147-166; the 70-bit merge at 259-265), and `load_be`'s shift merge (bits.rs:337-341) feeds every `Code::from_range`. Both are pinned only through the party-ops and skyline differentials, whose failure would point at the wrong layer, and nothing committed says whether those suites kill a mutant here. The sibling `BitsBuf` has exactly the family this lacks (`build_history_spelling_is_a_function_of_content`, `build_history_spellings_are_injective`). Doctrine: property tests where the claim is a family; "not wrong, but you couldn't tell if it were" is repairable.

Evidence:

       214	        let whole = len / 8;
       215	        let rem = (len % 8) as u32;
       216	        if whole < self.bytes.len() as u64 {
       217	            self.staged = if rem > 0 {
       218	                u64::from(self.bytes[whole as usize] >> (8 - rem))
       219	            } else {
       220	                0
       221	            };
       222	            self.staged_len = rem;
       223	            self.bytes.truncate(whole as usize);
       224	        } else {
       225	            self.staged >>= self.staged_len - rem;
       226	            self.staged_len = rem;
       227	        }

Resolution: Add `codec/build/tests.rs` mirroring the `BitsBuf` family: an arbitrary interleaving of `push_bit`, `push_code` (Small at every `len` in 1..=63, and Wide), `reserve` then `patch_bit` at committed and staged positions, `splice` from a view at every source alignment into every output alignment, `truncate` to byte-aligned, mid-byte, and empty targets, and `extract_code` at ranges below and above 63 bits spanning the committed/staged boundary; assert `len()` and `finish()` equal a clean `BitsBuf` rebuild at every step, and `extract_code`'s bits equal the model's slice. Add a `load_be` differential: for random buffers and every `(start, len <= 64)` within the live length, `load_be(start, len)` equals the fold of `bit(start + i)`. Acceptance: both suites are committed and red under each hand mutation: build.rs:218 `>> (8 - rem)` to `>> rem`; 297 `(8 - within - take)` to `(8 - within)`; 278 `carry << (8 - r)` to `carry << r`; bits.rs:340's `| (u64::from(buf[8]) >> (8 - shift))` dropped.
Construction: apply the mutation at build.rs:218 and run `just test-all`; if the party-ops and skyline suites stay green the gap is demonstrated (record which test catches it otherwise); repeat at 297 and 278.

### codec-bits-16: cursor.rs misdescribes SliceCursor's consumers and read_int's overrides
- Where: crates/before/src/codec/cursor.rs:71-78 (related: crates/before/src/codec/cursor.rs:110-113; crates/before/src/codec/tree.rs:35-39; crates/before/src/codec/dsi.rs:218-289; crates/before/src/borsh_impls.rs:110-126; crates/before/src/version/skyline/overlay.rs:504; crates/before/src/borsh_impls/tests.rs:311)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'impl.*BitCursor for'`: cursor.rs:104, dsi.rs:166, borsh_impls.rs:88, plus the test-only borsh_impls/tests.rs:311; `grep -rn 'fn read_int'`: the default at cursor.rs:79 and overrides at cursor.rs:123, dsi.rs:218, borsh_impls.rs:110; tree.rs:36 opens `super::DsiCursor::new_at(bits, pos)`; production `SliceCursor::new` sites are gamma.rs:117 and overlay.rs:504); executed: no
- Seen by: structure [4], prose [15]; refutation: confirmed; history: deliberate-but-expired for 110-113 (b3f09baa wrote it when tree.rs opened a `SliceCursor`; 5d167a63 moved `parse_id` onto `DsiCursor` and left the comment), no-rationale-found for 74 ("Both" was already an undercount when written: `DsiCursor` has overridden `read_int` since 3e5b95df23)
- Owner-gated: no

The trait doc counts two overriding cursors where three production implementors override (`SliceCursor`, `DsiCursor`, `ReaderCursor`) and says an override routes through `gamma::decode_int_window`, which `DsiCursor` does not use (it composes a table tier, a word arm, and a wide arm of its own); the default body serves only the test-only `BitwiseReaderCursor`. The `read_bit` comment at 110-113 says `SliceCursor` sits under the id-tree parsers; `parse_id` reads through `DsiCursor`, and `SliceCursor`'s production users are `decode_int` and the masked walks' `IdLeafCursor`. A maintainer pricing scan-meter work for the id parsers would look at the wrong cursor (Principle 5: no ghost references, no hand-maintained counts).

Evidence:

        71	    /// The provided default is the per-bit loop ([`decode_int_from`]); a cursor
        72	    /// with cheap access to its byte-backed window overrides it to route
        73	    /// through the word decoder ([`gamma::decode_int_window`]), which reads a
        74	    /// whole code in `O(1)` words. Both cursors override: [`SliceCursor`]
    ...
       110	        // One live bit scanned: this cursor is the sequential read primitive
       111	        // under the id-tree parsers and the per-bit gamma decode path, so the
       112	        // scan meter records here once for both. The skyline kernels read
       113	        // through `DsiCursor`, which carries its own records.

Resolution: Replace 71-78 with the contract every override must honor (accept and reject on exactly the inputs the per-bit loop does; the mechanism belongs at each override, where `SliceCursor::read_int` and `DsiCursor::read_int` already state theirs). Rewrite 110-113 to name the actual consumers: the per-bit primitive under `decode_int` and the masked id leaf cursor; the id parsers and skyline kernels read through `DsiCursor`. Optionally make `read_int` a required method and move the one-line default into `BitwiseReaderCursor`. Acceptance: no sentence in cursor.rs names a consumer that does not construct a `SliceCursor` or counts implementors; `grep -n 'Both cursors' crates/before/src/codec/cursor.rs` is empty.

### codec-bits-17: Idiom nits in the word-parallel cursor and the padding judge
- Where: crates/before/src/codec/dsi.rs:227-227 (related: crates/before/src/codec/dsi.rs:21, 152-159, 202, 219, 235-236, 364-380; crates/before/src/codec/bits.rs:496-500; crates/before/src/codec/cursor.rs:20-24)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the sites; `dsi-bitstream` 0.10.1 src/traits/words.rs:114-117 shows `read_word_opt`'s default returns `None`, src/impls/buf_bit_reader.rs:388-412 shows `skip_bits` looping over words for arbitrary counts, and src/codes/gamma_tables.rs:8 defines `READ_BITS = 9`); executed: no
- Seen by: structure [9]; refutation: confirmed; history: no-rationale-found (`From<Truncated> for Decode` already existed when the `map_err` was written; the chunk loop predates the `u64` widening it now incidentally serves)
- Owner-gated: no

Five places where the code spells more than it needs or less than the reader needs: (1) `map_err(|_| Decode::Truncated)` at 227 and the explicit `Err(Decode::Truncated)` at 235-236 re-spell the `From<Truncated> for Decode` impl; (2) `read_word` and `read_word_opt` (364-380) duplicate the bounds check and gather where `read_word` could be `self.read_word_opt().ok_or(OutOfBytes)` (overriding both is right, since the dependency's default returns `None`; duplicating the body is not); (3) bits.rs:496-500 branches on `remainder == 8` to avoid `1u8 << 8` where `u8::MAX >> (8 - remainder)` is total on `1..=8`; (4) `skip_int` chunks `skip_bits` at 64 (152-159) although the dependency skips arbitrary counts, and the one reason to keep the loop (the `as usize` cast must be exact on 32-bit targets, where a proven run can exceed `usize::MAX`) is unstated; (5) the prose hard-codes "9-bit" (21, 202, 219) for a dependency constant the code reads by name at 220 (`gamma_tables::READ_BITS`).

Evidence:

       227	        let k = self.unary_raw().map_err(|_| Decode::Truncated)?;
    ...
       235	            self.truncated();
       236	            return Err(Decode::Truncated);
    (bits.rs)
       496	            let mask = if remainder == 8 {
       497	                0xFF
       498	            } else {
       499	                (1u8 << remainder) - 1
       500	            };
    (dsi.rs)
       152	        let mut remaining = k;
       153	        while remaining > 0 {
       154	            let chunk = remaining.min(u64::from(u64::BITS));
       155	            self.reader
       156	                .skip_bits(chunk as usize)
       157	                .expect("the mantissa was proven to fit the live length");
       158	            remaining -= chunk;
       159	        }

Resolution: (1) `let k = self.unary_raw()?;` and `return Err(self.truncated().into());` (2) `fn read_word(&mut self) -> Result<u32, OutOfBytes> { self.read_word_opt().ok_or(OutOfBytes) }` (3) `let mask = u8::MAX >> (8 - remainder);` (4) one comment: "chunked so the `usize` cast is exact on 32-bit targets, where a proven run can exceed `usize::MAX` bits" (5) write "`READ_BITS`-wide table tier" or cite the constant once. Acceptance: `dsi/tests.rs` and `codec/tests.rs` pass unchanged; each cited site is one expression or carries its reason.

### codec-bits-18: ByteWords::gather takes four bounds-tested byte reads per refill, and the u32 word width is unargued
- Where: crates/before/src/codec/dsi.rs:338-345 (related: crates/before/src/codec/dsi.rs:292-302, 325-333, 360-380; dsi-bitstream-0.10.1 src/impls/buf_bit_reader.rs:18, 179-215)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read `gather`, `byte_at`, and the `BufBitReader` refill paths); executed: no
- Seen by: claims [37]; refutation: reframed (the word-width half misread the dependency: `read_bits_refill_word64` exists to spare 64-bit-word readers the cross-half shifts of their 128-bit buffer; `u32` words give a native `u64` buffer and never pay that cost); history: no-rationale-found (3e5b95df23 records "BufBitReader (BE, u32 words)" as a fact)
- Owner-gated: no

Every word refill under every skyline walk calls `byte_at` four times, two comparisons each; when `next + 4 <= body.len()` (every word but the last) one `u32::from_ne_bytes(body[next..next + 4].try_into())` suffices, a fixed-sign deletion of branches on the hottest read path, unmeasured. The `ByteWords` doc states why the tail zero-fills but not why the word is `u32` (a `u64` word would make the reader's buffer `u128`), so the next reader re-derives the choice.

Evidence:

       338	    fn gather(&self) -> u32 {
       339	        u32::from_ne_bytes([
       340	            self.byte_at(self.next),
       341	            self.byte_at(self.next + 1),
       342	            self.byte_at(self.next + 2),
       343	            self.byte_at(self.next + 3),
       344	        ])
       345	    }

Resolution: Add the aligned fast path in `gather` and state the `u32` rationale in the `ByteWords` doc (one sentence). Acceptance: the `dsi/tests.rs` differential suite and scan-bit envelopes unchanged; the doc names the word width's reason.

### codec-bits-19: The claimed k = 65 gamma witness is a second k = 64 row
- Where: crates/before/src/codec/dsi/tests.rs:30-33 (related: crates/before/src/codec/dsi/tests.rs:15-17; crates/before/src/codec/dsi.rs:23-25, 268-281; crates/before/src/codec/gamma.rs:25-26)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (arithmetic against the coding at gamma.rs:25-26, `k = floor(log2(n + 1))`: `n = u64::MAX` gives `m = 2^64`, `k = 64`; `n = 2^64` gives `m = 2^64 + 1`, `k = 64`); executed: no
- Seen by: prose [17]; refutation: confirmed; history: no-rationale-found (3e5b95df23 intended "word-seam witnesses at k = 63/64/65 and ~100"; the value chosen never delivered k = 65)
- Owner-gated: no

`wide(64)` is `2^64`, whose mantissa has the same 64-zero prefix as the `u64::MAX` row; the `// k = 65` comment, the testdoc's "the next (`k = 65`)", and dsi.rs's module doc "(`k = 63, 64, 65, ~100`)" all name a row the value list lacks. The row that is missing is a boundary: `k = 65` is the smallest wide code whose mantissa needs a second, one-bit chunk in `DsiCursor::read_int`'s loop (dsi.rs:268-281), where `k = 64` takes one 64-bit chunk. A test's doc must be accurate to its body; a production module doc that transcribes test parameters inherits the error.

Evidence:

        30	        Base::from(u64::MAX - 1), // k = 63: the machine arm's ceiling
        31	        Base::from(u64::MAX),     // k = 64: the first wide-arm code
        32	        wide(64),                 // k = 65
        33	        wide(100),                // far wide

Resolution: Use `wide(65)` (`m = 2^65 + 1`, `k = 65`) and fix the comment and the testdoc at 15-17; drop the parameter list from dsi.rs:23-25 ("at and across the word seam", letting the test carry the values). Acceptance: for every row comment `k = N`, `(value + 1).bits() - 1 == N`; dsi.rs's module doc names no specific `k`.
Construction: `(UBig::ONE << 64) + 1u32` has bit length 65, so `k = 64`; `(UBig::ONE << 65) + 1u32` has bit length 66, so `k = 65`.

### codec-bits-20: Ghost references to the retired store_be emit path
- Where: crates/before/src/codec/gamma.rs:13-16 (related: crates/before/src/codec/tests.rs:386-395, 514-519; crates/before/src/codec/gamma.rs:39-45)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn store_be crates/before --include='*.rs'` returns only codec/tests.rs:389 and 517; `git show 83e61b4d` removed the `store_be` calls and its message says the gamma word path emits through `BitsBuf::push_bits`; `encode_int`'s word arm is two `push_bits` appends at gamma.rs:43-44); executed: no
- Seen by: structure [5]; refutation: confirmed, adding gamma.rs:15's "emitted with one store" as a third site; history: contradicts-hard-rule (root AGENTS.md: nothing in the codebase refers to code that no longer exists)
- Owner-gated: no

Two test comments name a `store_be` helper that no longer exists, and gamma.rs's module doc describes the word path as "emitted with one store", the shape of the deleted `bitvec` store; the path is two `push_bits` appends. A hard-rule breach with a trivial fix: a reader hunting for the fast path under test greps for a name that is not there.

Evidence:

        13	//! Both directions keep the coding's cost word-scale: the stream is
        14	//! byte-backed, so a whole code is decoded from one 64-bit window
        15	//! ([`decode_int_window`]) and emitted with one store, with per-bit loops as
        16	//! the fallback — and, on decode, the sole arbiter of every reject.
    (codec/tests.rs)
       389	// `gamma::decode_int_window` / `store_be`, and the word-parallel cursor's
       517	    /// Holds for every value — `u64`-range codes (the `store_be` path) and

Resolution: "emitted as two word appends (`BitsBuf::push_bits`)" at gamma.rs:15; re-denominate both test mentions to `BitsBuf::push_bits`. Acceptance: `grep -rn store_be crates/before` returns nothing; gamma.rs:15 matches `encode_int`'s word arm.

### codec-bits-21: code_int and code_int_small share a body
- Where: crates/before/src/codec/gamma.rs:69-101 (related: crates/before/src/version/skyline/signed.rs:219, 225-226, 254; crates/before/src/version/skyline/build/tests.rs:14)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`grep -rn 'code_int('` callers: signed.rs:219, 226, 254, build/tests.rs:14; `code_int_small` at signed.rs:225); executed: no
- Seen by: structure [7]; refutation: confirmed (the proposed rewrite is behavior-identical, including `w = u64::MAX`, where `checked_add` fails and both take the wide path); history: no-rationale-found (e4c9b083ee added the word-scale entry; the duplication is a by-product)
- Owner-gated: no

`code_int(n: &Base)` computes `n.to_u64().and_then(checked_add(1))` and then runs the same `k` / `len <= SMALL_CODE_BITS` / `Code::Small` body as `code_int_small(n: u64)`; the only difference is the wide fallback's argument. Two copies of the small-code arithmetic to keep in step (a third, fused with zigzag, sits in signed.rs:241-252 and 261-271).

Evidence:

        69	pub(crate) fn code_int(n: &Base) -> Code {
        70	    if let Some(m) = n.to_u64().and_then(|n| n.checked_add(1)) {
        71	        let k = u64::BITS - 1 - m.leading_zeros();
        72	        let len = u64::from(2 * k + 1);
        73	        if len <= SMALL_CODE_BITS {
        74	            return Code::Small {
        75	                bits: m,
        76	                len: len as u8,
        77	            };
        78	        }
        79	    }
    ...
        87	pub(crate) fn code_int_small(n: u64) -> Code {
        88	    if let Some(m) = n.checked_add(1) {
        89	        let k = u64::BITS - 1 - m.leading_zeros();
        90	        let len = u64::from(2 * k + 1);

Resolution: `code_int(n) = match n.to_u64() { Some(w) => code_int_small(w), None => { let mut out = BitsBuf::new(); encode_int(&mut out, n); Code::Wide(out) } }`; or factor a private `small_code(m: u64) -> Option<Code>` both use. Acceptance: one function contains the `2 * k + 1 <= SMALL_CODE_BITS` test; the gamma and skyline build suites pass.

### codec-bits-22: gamma::load_window duplicates BitsView::load_be behind a raw-parts indirection with one caller
- Where: crates/before/src/codec/gamma.rs:153-207 (related: crates/before/src/codec/bits.rs:321-343; crates/before/src/codec/code.rs:51; crates/before/src/codec/dsi.rs:86; crates/before/src/borsh_impls.rs:119-121)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn load_be`: definition and code.rs:51 only; `window_int` and `load_window` are called only inside gamma.rs; `.body_tail()` at dsi.rs:86 and gamma.rs:154; the equivalence of `load_be(pos, proven) << (64 - proven)` to `load_window`'s zero-filled window derived from both bodies, including `load_be`'s discard of every bit past `len`, so a `Bits::live()` view's padding never leaks); executed: no
- Seen by: structure [2]; refutation: confirmed (re-derived the equivalence); history: deliberate-but-expired (78c65371 moved the window onto raw `(body, tail)` parts so the doors and the borsh reader could window without the 32-bit-capped borrowed view; 5d167a63 introduced `BitsView` with `load_be` and removed that constraint, leaving `window_int`/`load_window` in place)
- Owner-gated: no

`decode_int_window` destructures its view into `(body, tail)` solely to call `window_int`, whose only caller it is; `window_int` calls `load_window`, whose only caller it is; `load_window` gathers up to nine bytes and shifts a big-endian window exactly as `load_be` does, with clamps that exist only because it works on raw parts. Two places to get a shift-by-8 wrong, and `window_int`'s phantom-zeros prose restates `body_tail`'s contract a second time.

Evidence:

       153	pub(crate) fn decode_int_window(bits: BitsView<'_>, pos: u64) -> Option<(u64, u64)> {
       154	    let (body, tail) = bits.body_tail();
       155	    window_int(body, tail, bits.len(), pos)
       156	}
    ...
       189	    let mut buf = [0u8; 9];
       190	    let start = byte.min(body.len());
       191	    let end = byte.saturating_add(buf.len()).min(body.len());
       192	    buf[..end - start].copy_from_slice(&body[start..end]);
    (bits.rs)
       333	        let mut buf = [0u8; 9];
       334	        let end = (byte + buf.len()).min(self.bytes.len());
       335	        buf[..end - byte].copy_from_slice(&self.bytes[byte..end]);

Resolution: Fold `window_int` and `load_window` into `decode_int_window`: `let proven = bits.len().checked_sub(pos)?.min(WINDOW_BITS); if proven == 0 { return None; } let window = bits.load_be(pos, proven as u32) << (WINDOW_BITS - proven); let k = u64::from(window.leading_zeros()); let code_len = 2 * k + 1; if code_len > proven { return None; } let m = window >> (WINDOW_BITS - code_len); Some((m - 1, pos + code_len))`. `load_be`'s debug assert holds because `pos + proven <= len`; `body_tail` keeps its one remaining caller. Acceptance: gamma.rs has one window function; `gamma_window_edge`, `gamma_window_declines_conservatively`, `gamma_word_decode_matches_bit_loop`, `gamma_word_paths_match_on_arbitrary_bytes`, and the borsh differentials pass unchanged.

### codec-bits-23: The wide-gamma width guard sits at usize::MAX, above dashu's capacity cap: 32-bit decode can panic inside dashu
- Where: crates/before/src/codec/gamma.rs:243-253 (related: crates/before/src/codec/dsi.rs:248-267; crates/before/src/borsh_impls.rs:119-125; crates/before/wasm32-pins/harness/tests/pins.rs:130-146; dashu-int-0.5.0 src/buffer.rs:48, 57-60, 121-129, 205-206, 230-231; src/bits.rs:428-434, 551-561; src/arch/mod.rs:64-66; src/arch/generic_32_bit/word.rs:2)
- Class / severity / confidence: correctness / medium / medium
- Provenance: assessed (traced `dashu-int` 0.5.0 by reading: `Word = u32` under `target_pointer_width = "32"`; `Buffer::MAX_CAPACITY = usize::MAX / WORD_BITS_USIZE`; `default_capacity` clamps with `.min(Self::MAX_CAPACITY)` behind a `debug_assert`; `UBig::ZERO.set_bit(k)` for `k >= 64` takes `with_bit_dword_spilled`, which allocates `idx + 1` words and pushes `idx + 1` words; `push` asserts `self.len < self.capacity` in release); executed: no
- Seen by: correctness [25]; refutation: confirmed, severity raised low to medium; history: already-known in part (wasm32-pins/harness/tests/pins.rs:138-141 records the backend capacity as unreachable "through the doors", pricing `Version::decode`'s working set only; the borsh `ReaderCursor` path is not priced there, and 05d87e1b's stated intent was to reject at the same width with `NotCanonical` on both paths)
- Owner-gated: no (the fix is crate-private; the pins.rs record wants amending alongside)
- Witness (witness/results.md, `## 32-bit pass`): demonstrated (run in the `wasm32-pins` executor; the main pass was inconclusive on the 64-bit host). Borsh-deserializing a `Version` in the wasm32 guest from a stream of the leaf flag, `2^32 - 32` zero bits, and the mantissa's leading `1` traps with no mantissa byte read; the lower adjacency at `2^32 - 33` zeros allocates the identical clamped buffer, sets the bit, reads to the stream's end, and returns a typed error, so the trap is the guard's, not memory exhaustion. Each run took about 29.7 s. The pins workspace's lock resolves dashu-int 0.5.1 (the entry quotes 0.5.0; the cited lines are identical). Not exercised: the word-parallel twin at dsi.rs:255-267. The trap's site inside `Buffer::push` is attributed by reading: the guest has no stderr.

On a 32-bit target `usize::try_from(k)` admits `k` in `[2^32 - 32, 2^32 - 1]`, where `set_bit(k)` needs `2^27` words but the backend's buffer holds at most `2^27 - 1`: `default_capacity` clamps instead of rejecting, `push_zeros(idx - 2)` fills the buffer exactly, and the final `push` fails a release `assert!`. The comment says the backend "caps magnitudes below `usize::MAX` bits", which is true but not what the guard implements; the guard is 32 values loose. Reachable from decoded bytes on a supported target (rust-toolchain.toml installs `wasm32-unknown-unknown`): the borsh reader's per-bit path calls `set_bit(k)` after the prefix alone, about 512 MiB buffered, before any mantissa read. Hard rule: a panic never reachable from decoded bytes, whatever the input's likelihood. The same guard appears at dsi.rs:255. On 64-bit targets the arm is dead, as the comment says.

Evidence:

       243	    // A mantissa at or past `usize` bits names a value the big-integer
       244	    // backend cannot hold on this target (it caps magnitudes below
       245	    // `usize::MAX` bits), so the reject genre is the value's, not the
       246	    // machine's — the word-parallel reader (`DsiCursor::read_int`) rejects
       247	    // at the same width with the same genre. On 64-bit targets the arm is
       248	    // dead: reading 2^64 prefix bits first needs an unallocatable input.
       249	    let Ok(k) = usize::try_from(k) else {
       250	        return Err(Decode::NotCanonical);
       251	    };
       252	    let mut m = UBig::ZERO;
       253	    m.set_bit(k);
    (dashu-int-0.5.0 src/buffer.rs)
        48	    pub const MAX_CAPACITY: usize = usize::MAX / WORD_BITS_USIZE;
    ...
        59	        (num_words + num_words / 8 + 2).min(Self::MAX_CAPACITY)
    ...
       206	        assert!(self.len < self.capacity);

Witness output (32-bit pass; `cargo nextest run --cargo-profile release --no-fail-fast -E 'test(/_checked_guest$/)'` in `crates/before/wasm32-pins`):

    ```text
            PASS [  29.695s] (10/11) wasm32-pins-harness::zz_witness version_borsh_wide_gamma_below_loose_guard_checked_guest
            PASS [  29.744s] (11/11) wasm32-pins-harness::zz_witness version_borsh_wide_gamma_at_loose_guard_checked_guest
         Summary [  29.745s] 11 tests run: 11 passed, 51 skipped
    ```

    (`..._at_loose_guard_...` asserts `Outcome::Trapped(Trap::UnreachableCodeReached)` at `(1u64 << 32) - 32` zeros; `..._below_loose_guard_...` asserts `Outcome::Value(-1)`, the export's typed-error code, at `(1u64 << 32) - 33`.)

Resolution: Replace both `usize::try_from(k)` guards with one shared predicate at the backend's cap: with `W = usize::BITS as u64` (dashu's `Word` is `usize`-wide on 32- and 64-bit targets), a `k`-bit mantissa needs `k / W + 1` words and the backend holds at most `usize::MAX / W`, so reject when `k / W >= usize::MAX as u64 / W` (which subsumes the `try_from` failure); state the derivation inline and the dependence on dashu's `Word` width beside the existing "bumping that dependency is a breaking change" rule; amend pins.rs:138-141 to name the borsh path. Pin it under `cfg(target_pointer_width = "32")` (or in the wasm32-pins guest) with a test-only `BitCursor` that yields `2^32 - 32` zero bits then a `1` without materializing them, asserting `Err(Decode::NotCanonical)`. Acceptance: on wasm32 the pin returns `Err(Decode::NotCanonical)` where today it panics with dashu's `assertion failed: self.len < self.capacity`; both arms route through the one predicate (`grep` shows a single `usize::MAX` division site); the 64-bit differential suites are unchanged.
Construction: target wasm32 (`usize` 32 bits, `Word = u32`, `MAX_CAPACITY = 2^27 - 1`). Borsh-deserialize a `Version` from a reader yielding the skyline stream `1` then `k = 2^32 - 32` zero bits then a `1` (about 512 MiB of `0x80, 0x00 ...`): `ReaderCursor::read_int` (borsh_impls.rs:125) falls to `decode_int_from`; `k < 64` is false; `usize::try_from(2^32 - 32)` is `Ok`; `m.set_bit(k)`: `idx = 2^27 - 1`, `Buffer::allocate(2^27)` clamps capacity to `2^27 - 1`, `push(lo)`, `push(hi)`, `push_zeros(2^27 - 3)` passes its `assert!(n <= capacity - len)` exactly, then `push(1 << (k % 32))` fails `assert!(self.len < self.capacity)`. Any `k` in `[2^32 - 32, 2^32 - 1]` works; `k = 2^32 - 33` and below allocate and succeed. The word-parallel path reaches dsi.rs:267 identically given a stream of `2k + 1` live bits.

### codec-bits-24: literal.rs is the one production module in the partition without a module doc
- Where: crates/before/src/codec/literal.rs:1-3 (related: crates/before/src/codec.rs:9-10; crates/before/src/codec/stack/tests.rs:1)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the file; every other production module here opens with `//!`); executed: no
- Seen by: structure [12], prose [21]; refutation: confirmed; history: no-rationale-found (no commit ever added a `//!` doc)
- Owner-gated: no

The file opens with imports; codec.rs:9-10 sends the reader here for the id tree's literal door and the file does not say what it is. A module doc's first sentence stands alone in a listing.

Evidence:

         1	use crate::error::Parse;
         2	
         3	use super::{validate_id, BitsBuf, BitsView};

Resolution: Add a one- or two-sentence `//!` doc: the id tree's in-memory constructors (`id_leaf`, `id_node`) and the anonymous-id predicate (`id_is_empty`) that the `Party` literal and text doors assemble normal-form ids from. `stack/tests.rs` likewise opens with imports. Acceptance: both files begin with `//!`.

### codec-bits-25: id_node re-parses a node it has proved normal by construction
- Where: crates/before/src/codec/literal.rs:51-65 (related: crates/before/src/party.rs:809-844; crates/before/src/codec/tree.rs:21-22, 57-102, 114-124)
- Class / severity / confidence: vestigial / nit / medium
- Provenance: verified (`grep -rn 'id_node('`: the one production caller is party.rs:842, inside the sealed `PartyLiteral` whose children come from `id_leaf`/`id_node`; `validate_id` wraps `parse_id`); executed: no
- Seen by: structure [10], claims [38]; refutation: confirmed (38 lowered to nit: the literal door is cold and depth is bounded by source nesting); history: no-rationale-found (32a655438f wrote the local rejections and the `validate_id` call together)
- Owner-gated: no

Given normal children and the two local rejections (`(0, 0)` and `(1, 1)`), the assembled stream is normal by induction (tree.rs:21-22 defines normal form as exactly these rules), so `validate_id`'s `Err` arms are unreachable from the sealed trait and each nesting level re-walks its whole subtree (O(n * depth) over a literal of n nodes). A guard earns its place by naming a constructible failure the committed tests cannot catch; this one names none, and the caller's comment "assembles + validates normal form" tells a reader the validation is load-bearing.

Evidence:

        51	/// Rejects a collapsible `(0, 0)` or `(1, 1)`, then validates the result.
    ...
        62	    b.extend_from_buf(l);
        63	    b.extend_from_buf(r);
        64	    validate_id(super::buf::built_view(&b))?;
        65	    Ok(b)

Resolution: Delete the `validate_id` call and state the induction in the doc (children are normal outputs of `id_leaf`/`id_node`; excluding `(0, 0)` and `(1, 1)` is the whole of normalization), or move a single `validate_id` to `finish_id` (party.rs) so the whole literal is validated once, or keep it as a `debug_assert!`. Acceptance: the literal door's tests in `party/tests.rs` pass; a nested literal validates at most once, or the doc names the failure the re-parse catches.

### codec-bits-26: scan.rs's record-site roster has drifted
- Where: crates/before/src/codec/scan.rs:9-17 (related: crates/before/src/party/ops/diff.rs:312, 352, 363; crates/before/src/party/ops/index.rs:77, 91, 200, 282; crates/before/src/version/skyline/grow.rs:295, 302; crates/before/src/codec.rs:46-49, 63-65; crates/before/src/codec/buf.rs:380-382)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rc record_bits crates/before/src`: idbits.rs 3, party/ops/index.rs 4, party/ops/diff.rs 3, codec/dsi.rs 8, codec/build.rs 7, codec/cursor.rs 2, version/skyline/grow.rs 2; index.rs:282 records 32 bits per binary-search step; grow.rs:295 and 302 record 2); executed: no
- Seen by: prose [16]; refutation: confirmed; history: no-rationale-found (the roster was the initial inventory in 197dfd9e16, extended once for `DsiCursor`; later record sites in grow.rs, diff.rs, and index.rs landed without touching it)
- Owner-gated: no

The module doc enumerates where `record_bits` fires and omits three files that record directly (`party/ops/diff.rs`, `party/ops/index.rs`, `version/skyline/grow.rs`), and attributes builder writes to "`party::ops`' builder" although `PackedBuilder` serves the skyline builder too. The `seal_padding` consumer list is stated twice (codec.rs:46-49 and buf.rs:380-382). Hand-maintained caller rosters rot silently, and this one has; a meter's doc should state the rule for where records happen, so a reviewer can judge a new kernel against it.

Evidence:

         9	//! - id tag reads and skip steps (`idbits::IdReader`), 2 bits per node;
        10	//! - id-builder bit writes and verbatim splice lengths
        11	//!   (`party::ops`' builder);
        12	//! - event topology cursor advances and gamma code-skips (the skyline
        13	//!   walks' word-parallel `codec::DsiCursor` — unary runs and code
        14	//!   skips record their full bit widths, however the reads batch);
        15	//! - every sequential decoder/validator bit read (`codec::SliceCursor`
        16	//!   and `codec::DsiCursor`, which carry `decode`, the gamma decoder,
        17	//!   and the skyline validator/decoder cursors).

Resolution: State the rule: records fire at the packed-stream primitives (builder appends and splices, cursor bit, unary, and code reads), and any kernel that examines packed bits outside those primitives records at its own site at the width it examined. If a roster is wanted, put it in a test that greps for `record_bits` call sites. Collapse the `seal_padding` consumer list to one sentence at `seal_padding` and let codec.rs point there. Acceptance: scan.rs's module doc names no file or type as a record site; the `seal_padding` enumeration appears at most once.

### codec-bits-27: BitStack's type doc claims every operation is O(1); all_set and trailing_ones are scans
- Where: crates/before/src/codec/stack.rs:13-17 (related: crates/before/src/codec/stack.rs:104-123, 176-184; crates/before/src/version/skyline/build.rs:301-304)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read both scans; `all_set`'s only production use is the debug assert at skyline/build.rs:302); executed: no
- Seen by: correctness [29], claims [33]; refutation: confirmed; history: no-rationale-found (b3f09baa wrote the sentence after both scans existed)
- Owner-gated: no

`all_set` iterates every spilled word, O(len / 64); `trailing_ones` reads one word per 64 bits of the run and its own doc prices it so. Complexity statements in this crate are hard guarantees, and a false O(1) on a primitive is what lets a caller treat a run-proportional read as free (codec-bits-29).

Evidence:

        13	/// A pop-able stack of bits over machine words.
        14	///
        15	/// The newest bit lives at the low end of the top register; a filled register
        16	/// spills whole into the word vector and refills on the pop that crosses back.
        17	/// Every operation is O(1) with no bit-addressing arithmetic.

Resolution: "`push`, `pop`, `last`, and `set_last` are O(1); the run scans (`trailing_ones`, `all_set`) are priced where they are declared." Acceptance: the type doc names no operation as O(1) that is not.

### codec-bits-28: Dead `len == 64` arm in BitStack::push_bits, the sibling of the disjunct 35a09c5b swept from PackedBuilder::append_bits
- Where: crates/before/src/codec/stack.rs:61-69 (related: crates/before/src/codec/stack.rs:87-88, 213-231, 256-262; crates/before/src/codec/build.rs:245-250)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 35a09c5b -- crates/before/src/codec/build.rs` shows the hunk `-            len == 64 || value >> len == 0,` / `+            value >> len == 0,`; `git show d07fc20e:crates/before/src/codec/stack.rs` lines 58-69 hold the identical arm with the `len <= 63` assert already present; callers at stack.rs:221, 223, 227, 229 pass 63 or `width < 64`); executed: no
- Seen by: structure [3], prose [22], correctness [28], claims [32]; refutation: confirmed; history: no-rationale-found (landed dead in d07fc20e; 35a09c5b's sweep named the disposition for the builder and did not reach this sibling)
- Owner-gated: no

The debug assert requires `len <= 63`, so its `len == 64 ||` disjunct is unsatisfiable and the `if len == 64 { value }` arm can never execute; `pop_bits` (87-88) already has the clean form, and `PopStack::push`/`pop` carry four explicit `width == 64` splits precisely because 64-bit batched moves are not supported. A branch the preceding assert excludes makes the reader check whether the contract or the code is wrong.

Evidence:

        61	    fn push_bits(&mut self, value: u64, len: u32) {
        62	        debug_assert!(len <= 63 && (len == 64 || value >> len == 0));
        63	        let total = self.top_len + len;
        64	        if total <= 64 {
        65	            self.top = if len == 64 {
        66	                value
        67	            } else {
        68	                (self.top << len) | value
        69	            };

Resolution: Mirror 35a09c5b: `debug_assert!(len <= 63 && value >> len == 0);` and `self.top = (self.top << len) | value;`. (The larger alternative, making `push_bits`/`pop_bits` total on `1..=64` and deleting `PopStack`'s four `width == 64` splits, is an open question below.) Acceptance: no `len == 64` text remains in `push_bits`; `bit_stack_matches_a_vec_of_bools` and `pop_stack_matches_a_vec_model_across_all_widths` pass.

### codec-bits-29: peek_flip re-scans the path's trailing run on every call; parked cursors in the masked and projection walks pay O(run / 64) per step of the other operand, outside every meter
- Where: crates/before/src/codec/stack.rs:104-108 (related: crates/before/src/version/skyline/overlay.rs:347-354, 377-382; crates/before/src/version/skyline/masked.rs:48-59, 317-357; crates/before/src/version/skyline/query.rs:533-549; crates/before/tests/meter.rs:13-33; crates/before/src/meter/registry.rs)
- Class / severity / confidence: claim / medium / medium
- Provenance: assessed (read the call path: `masked::advance` (349-357) calls `block_skip` on every advance; `block_skip` (317-345) evaluates `self.a.peek_flip()` whenever the a-mask is unowned, before the `> a_bound` compare can fail; `peek_flip` (overlay.rs:352-354) is `path.len() - path.trailing_ones()`; `trailing_ones` (stack.rs:109-123) reads one spilled word per 64 trailing ones; the projection loop (query.rs:536-549) peeks once per unowned outer iteration; `grep record_bits crates/before/src/codec/stack.rs` is empty); executed: no
- Seen by: claims [31]; refutation: confirmed (the construction was not run; the term is established from the code); history: no-rationale-found (63b2f1b5 introduced `peek_flip` as "read without moving" with no amortization argument; masked.rs's cost derivation counts pushes and pops only)
- Owner-gated: no

`trailing_ones` prices itself as the run the caller is about to pop, but `peek_flip` reads it without moving, and two walks evaluate it once per step while the cursor is parked. A cursor parked at a leaf whose path ends in `r` right branches (`r > 64`) costs about `r / 64` word reads per peek; if the other operand takes `n` steps inside that leaf's interval, the walk does `Theta(n * r / 64)` word reads on inputs of `Theta(r + n)` bits. masked.rs:52-55 derives `O(|v| + |p| + |w|)` from "every path bit pushed and popped at most once", which does not cover these non-popping reads; none of the board's deterministic meters (scan bits, peak heap, stack segments, limb ops) counts a stack-word read, and the `FamilyId` roster has no parked-deep-run times wide-neighbor family (the `MaskedHole` shape is the amortized case: peek, then pop). Crate docs (lib.rs:351-353): any asymptotic claim is a hard guarantee for all input shapes.

Evidence:

       104	    /// The exact run of set bits at the top of the stack.
       105	    ///
       106	    /// One word read per 64 bits of the run: the cost is the run the caller is
       107	    /// about to pop (or has decided not to), never the whole stack. `u64`,
       108	    /// as [`len`](Self::len): the run is bounded by the stack's own height.
    (masked.rs)
       319	            let a_bound = self.others_deepest(Self::A);
       320	            if self.a_mask.as_ref().is_some_and(|mask| !mask.owned())
       321	                && self.a.peek_flip() > a_bound
    (overlay.rs)
       352	    pub(super) fn peek_flip(&self) -> u64 {
       353	        self.path.len() - self.path.trailing_ones()
       354	    }

Resolution: Cache the flip level in `LeafCursor`: recompute `len - trailing_ones()` once after each `descend`/`step` (that one scan is bounded by the run the next `step` pops, so it amortizes to O(1) per plateau) and have `peek_flip` return the cached `u64`. Then (a) add the peek to masked.rs's cost argument and to `trailing_ones`'s doc ("callers that peek repeatedly must cache"); (b) register the dual family (a parked right run of depth `r` in one operand, `n` plateaus inside its interval in the other, the a-mask unowned there) for `masked_cmp` and `project`; (c) give the board a currency that sees it, either `scan::record_bits_u64(64 * words)` inside `trailing_ones` or the bench judge's two-scale ratio on the new cells. Acceptance: a committed test builds the dual shape at `(r, n)` and `(2r, 2n)` and asserts the walk's stack-word reads stay within a flat per-input-bit band across the doubling (about 4x today, about 2x with the cached flip level); the masked and projection cost arguments name the peek and its amortization.
Construction: version `v`: root = node(left = L1, right = leaf h = 0); L_k = node(left = leaf h = 1, right = L_{k+1}) for k = 1..r, with L_{r+1} = leaf h = 0 (every sibling pair is leaf/internal or of distinct heights, so canonical). v's preorder leaves have paths 00, 010, ..., 0 1^(r-1) 0, then the parked leaf 0 1^r (r trailing ones), then 1. Party `p` = "(0, 1)" (unowned on [0, 1/2)). Version `w`: the same spine to depth r + 1 with L_{r+1} replaced by a subtree of n leaves with alternating heights 0/1, all inside v's parked leaf's interval. Run `(&v / &p).partial_cmp(&w)`: after r lockstep advances `a` is parked and `b` advances n - 1 times; each advance runs `block_skip`, whose first conjunct is true, so `self.a.peek_flip()` scans about r / 64 words and then compares false against `a_bound >= r + 1`. Count words read in `trailing_ones` (a temporary counter) at r = n = 8192 and r = n = 16384 against input bits; expect the ratio to approach 4 rather than 2. For `project`, replace `w` by a party alternating owned/unowned across n leaves inside the same interval and call `(&v / &p2).to_version()`.

### codec-bits-30: The BitStack model test names a method that does not exist, omits set_last and trailing_ones, and reaches the spill it advertises about once in a hundred runs
- Where: crates/before/src/codec/stack/tests.rs:21-27 (related: crates/before/src/codec/stack.rs:47-56, 109-123, 141-151, 158-165; crates/before/src/version/skyline/overlay.rs:352-354; crates/before/src/version/skyline/fill.rs:1264; crates/before/src/version/skyline/fill/prescan.rs:676; crates/before/src/version/skyline/fill/fuse.rs:434; crates/before/src/meter/tier2/tests.rs:625-635)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (stack.rs:28-185 defines `new`, `len`, `push`, `push_bits`, `pop_bits`, `trailing_ones`, `trailing_ones_capped`, `pop`, `set_last`, `last`, `all_set`, no `is_empty`; the body checks `pop`, `last`, `len`, `all_set` only; `grep -rn 'set_last\|trailing_ones'` outside stack.rs finds production sites only, no test); executed: yes: `spill.py` (an exact dynamic program over the generator's distribution: lengths uniform in 1..=299, push and pop each 1/2, pop on empty a no-op) gives P(height reaches 64 within a case) = 4.0135e-05 and P(reaches 65, the spill) = 3.0997e-05, so P(any of 256 cases spills) = 0.79%; the roster claims are grep-verified, not run
- Seen by: prose [14], correctness [27], claims [34]; refutation: confirmed (independently reproduced 4.0135e-05); history: no-rationale-found (56a3dfe2 landed the doc and test together; `is_empty` never existed on `BitStack`; `set_last` and `trailing_ones` were never added to the model)
- Owner-gated: no

The testdoc promises `is_empty` (nonexistent) and coverage "across the word-spill boundary at 64 bits"; with a symmetric 50/50 walk of at most 299 steps the spill is reached in about one run in a hundred, so the single-bit spill/refill arms (stack.rs:49-53, 143-146) and `last`/`set_last` on an empty top register go unexercised by the one test that drives `BitStack` directly. `set_last` (fill.rs:1264, prescan.rs:676, fuse.rs:434, whose `top_len == 0` arm rewrites `words.last_mut()`) and `trailing_ones` (`peek_flip`, whose multi-word loop at stack.rs:114-121 fires only for a right-branch run longer than 64) are not modeled at all, and I found no oracle-checked test reaching that loop: the deep-shape join/meet differential caps at scale 48 (tier2/tests.rs:628-629), the depth-100k proof is a left spine, and the `RightSpine` generator feeds only party and fill tests. A test's doc must be accurate; a model that omits two of the type's methods leaves the branches only deep right spines reach unpinned, and `peek_flip` decides which plateaus the ownership-gated walks skip, so a wrong run count is a wrong answer.

Evidence:

        21	    /// The word-backed bit stack agrees with a plain `Vec<bool>` on any
        22	    /// interleaving of pushes and pops — `last`, `len`, `is_empty`, and
        23	    /// `all_set` included — across the word-spill boundary at 64 bits.
        24	    #[test]
        25	    fn bit_stack_matches_a_vec_of_bools(
        26	        ops in proptest::collection::vec((any::<bool>(), any::<bool>()), 1..300),
        27	    ) {

Resolution: Drop `is_empty` from the doc. Make the spill reachable by construction (bias pushes, e.g. `prop::bool::weighted(0.75)`, or prefix each case with a deterministic ramp of at least 65 pushes) and pin the reach with `prop_assert!(max_height >= 65)` per case. Add `set_last` as a third op kind (model: overwrite `model.last_mut()`) and assert `stack.trailing_ones() == model.iter().rev().take_while(|b| **b).count() as u64` at every step, with runs long enough to cross two spilled words. Acceptance: the extended test is red under each of: stack.rs:118 `if w < 64` to `if w <= 64` (caps the run at one spilled word); 117 `run += u64::from(w)` to `run = u64::from(w)`; 162-163's `words.last_mut()` arm replaced with a no-op; and the testdoc names only methods the body checks.
Construction: for the multi-word loop specifically, `let mut s = BitStack::new(); for _ in 0..70 { s.push(true); } assert_eq!(s.trailing_ones(), 70); s.push(false); for _ in 0..3 { s.push(true); } assert_eq!(s.trailing_ones(), 3);` is exercised by no committed test; the `w <= 64` mutation above passes every current suite unless some walk builds a right run deeper than 64 and checks its value.

## Positives

- `BitsBuf`'s two representation invariants (exact bytes, zeroed dead bits; buf.rs:5-21) make byte equality one `memcmp`, sealing a single push, and the freeze a move; the build-history family in `codec/tests.rs` (`build_history_spelling_is_a_function_of_content`, `build_history_spellings_are_injective`) drives the whole move set against a clean rebuild at every intermediate state. This is the model codec-bits-15 asks `PackedBuilder` to inherit.
- `DsiCursor` keeps the accept/reject boundary in the wrapper, never the dependency (dsi.rs:9-12), refuses `dsi-bitstream`'s capped `read_gamma` with the reason stated (14-18), and is pinned to the per-bit loop at every cut point, across word seams, and on arbitrary interleavings (`dsi/tests.rs`). `gamma.rs:142-148`'s "the per-bit loop is the sole arbiter of every reject" is exactly the discipline that makes a fast path reviewable, and `gamma_word_paths_match_on_arbitrary_bytes` pins it on arbitrary bytes.
- `DsiCursor::truncated` (dsi.rs:124-136) records the examined tail before a reject surfaces so the truncation-reject scan floors stay live; the comment names the instrument it serves. The same instrument-aware prose sits at dsi/tests.rs:63-73 (`skip_int`'s two-sided floor witness).
- `Truncated` (cursor.rs:7-24) is a ZST whose doc names the concrete cost it removes (drop glue per successful bit read) and where the rich error is still paid.
- Every `expect` message in the partition reads as a one-line proof I could discharge (`a partial byte exists`, `the mantissa was proven to fit the live length`, `a mid-byte start has at least its own byte to skip within`, `buf holds 8 whole bytes`), and every asserting `pub(crate)` entry carries a `# Panics` section matching its assert.
- `Bits::ptr_eq`'s contract (bits.rs:200-216) states precisely what a rung may derive (equality, never clone provenance), names the counterexample that constrains it (`is_disjoint` on the empty share), and points at the committed seed; the injectivity argument is stated once at the type and reused by `canonical_eq`/`canonical_hash` without restatement.
- `padding_is_canonical`'s slice patterns (bits.rs:446-453) and `require_marker_padding`'s genre split (467-473) read as evidently correct; the flush-boundary `[0x80]` corner is handled by pattern, not arithmetic.
- `scan::record_bits` compiles to nothing without its feature (scan.rs:49-59), so every primitive calls it unconditionally, and the pricing convention (bits examined, however the path batches them) is stated at each batched record (cursor.rs:128-131, dsi.rs:178-179).
- `PopStack` prices depth in bits with the cost model stated (stack.rs:187-195) and is pinned against a `Vec` model across all `1..=64` widths, the testdoc explaining why uniform widths matter for the 62-continuation cap (stack/tests.rs:43-51).
- `Code` and `Int` keep narrow payloads in machine words end to end with the parking rule spelled out (int.rs:22-24, 56-62); `DsiCursor::read_int`'s wide arm routes through `Int::from_base`, so a `k = 64, rest = 0` value that fits a word lands `Small`, matching the per-bit path.
- `id_is_empty`'s debug assert (literal.rs:8-14) is justified by naming what a fuller check would break: dev builds would meter a different program than the release board.

## Open questions for Finch

1. Is the staging register's saving (no `Vec` traffic on sub-byte appends) measurable on the bench corpus? codec-bits-12's resolution is construct-and-measure. Recommendation: build `PackedBuilder { out: BitsBuf }`, run the judge at parent and change on a quiet machine, and take the consolidation unless a cell leaves its band; if the register wins, move the register form into `BitsBuf` instead so one implementation remains.
2. codec-bits-28 offers two shapes: delete the dead arm (minimal), or make `push_bits`/`pop_bits` total on `1..=64` and delete `PopStack`'s four `width == 64` splits. Recommendation: the deletion now; the larger change only if the splits bother you when you next touch `PopStack`, with `pop_stack_matches_a_vec_model_across_all_widths` as the oracle.
3. Is bits.rs meant as the single home of the rung policy (306e2de0 says so, the file does not)? Recommendation: keep it the home, say so in the essay's first line, and reduce the sites to pointers, so the policy lives once.
4. codec-bits-23's shared predicate encodes dashu's `Word` width. The alternative bounds the mantissa width by the input's own byte length (a `k`-bit mantissa cannot exceed `8 * bytes` on any path), which `DsiCursor` already knows and the borsh reader could learn. Recommendation: the shared predicate, with the derivation stated beside the existing dashu-pinning rule; it is the smaller change and matches 05d87e1b's intent.
5. Should covcheck's scope extend from `version/skyline/` to `codec/` so the kernels' branches (`load_be`'s shift merge, `trailing_ones`'s spilled-word arm, `PackedBuilder::truncate`'s two arms) are pinned covered by name? Recommendation: yes, once codec-bits-15 and codec-bits-30 land, since the roster then has tests to point at.
6. Should the board carry a currency for stack-word reads? Recommendation: charge 64 scan bits per word read inside `trailing_ones` (cheap, and it makes the peek visible to every existing floor and ceiling), plus the dual family from codec-bits-29.
7. Is an `Int::Wide` parking a word-scale value constructible in production? `Int::from_base`, `Int::from_ubig`, and both readers park such values as `Small`; if the signed arithmetic never leaves one parked wide, the `Some(b) => a.cmp(&b)` arms of `cmp_magnitude` (int.rs:70-77) are dead and unpinned. Recommendation: a committed witness or dissolve the arms; I did not trace the arithmetic and report it as a question, not a finding.
8. `crates/before/src/codec/tests.rs` (1976 lines) is the `mod tests` of codec.rs but is in no partition's file list; this review checked only the ranges its findings cite. Recommendation: assign it explicitly.
9. dsi.rs:255-265 and gamma.rs:249-251 classify a mantissa width that does not fit the target as `Decode::NotCanonical`, arguing the reject genre is the value's, not the machine's. On a 32-bit target that labels a canonically coded but unrepresentable value non-canonical. Recommendation: keep the genre (a public-API variant is the alternative) but say in `Decode::NotCanonical`'s doc that representability on the target is part of what the doors admit.

## Dropped

- [39] SliceCursor has no multi-bit read, so the id parser pays two reads per tag: refuted; `parse_id` reads through `DsiCursor` (tree.rs:36), so the proposed `SliceCursor::read_bits` would not touch `Party::decode`.
- [37] u64 word width for `ByteWords`: reframed; a 64-bit word forces `dsi-bitstream`'s 128-bit buffer, which its specialized refill exists to work around; the gather fast path survives as codec-bits-18.
- [13] injectivity and pricing-rule restatements: dropped from codec-bits-6; they sit at the sites that establish or rely on the argument, which the owner's writing doctrine and 7ea3df58/05d87e1b sanction.
- [19] duplicate of codec-bits-1 (ladder essay placement).
- [15] merged into codec-bits-16 (cursor.rs prose).
- [21] duplicate of codec-bits-24 (literal.rs module doc).
- [22], [28], [32] duplicates of codec-bits-28 (dead `len == 64` arm; 32 supplied the 35a09c5b provenance).
- [27], [34] merged into codec-bits-30 (BitStack model test) and codec-bits-15 (PackedBuilder and `load_be` model tests).
- [33] duplicate of codec-bits-27 (O(1) claim).
- [35] duplicate of codec-bits-8 (`[0x80]` at pos 0), at nit.
- [38] duplicate of codec-bits-25 (id_node re-validation), at nit.
- Refutation new 1 (gamma.rs:15 "one store") merged into codec-bits-20; new 2 (codec/tests.rs:168-172 mechanism) merged into codec-bits-7; new 3 (`extract_code` at `n == 0`) merged into codec-bits-13; new 4 was a correction to codec-bits-5's evidence, applied.
