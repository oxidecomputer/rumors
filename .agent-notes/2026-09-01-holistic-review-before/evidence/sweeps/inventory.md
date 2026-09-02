# Sweep inventory: Allow attributes, panic sites, and visibility inventory

## Method and coverage

The sweep ran three mechanical inventories over the shipped source of `before`
and `suanpan` at 9e5784fb (script: `sweeps/inventory/inventory.sh`, raw output
in `sweeps/inventory/out/`). Tier 1 is every `.rs` under `crates/before/src` and
`crates/suanpan/src` minus `tests.rs` files, `tests/` directories, and the
instrument modules (`meter*`, `surface.rs`, `laws*`, `oracle*`, `testing*`);
tier 2 is the instrument modules minus their tests. Over each tier it grepped
for `#[allow]`/`#[expect]` attributes (14 in tier 1, 13 in tier 2), `unwrap()`
(0 shipped non-doctest sites), `expect(` (143 shipped sites), `panic!` (1
shipped), `unreachable!` (34), hard and debug asserts (24 and 174 lines),
narrowing `as` casts (80 sites), shifts, division, and `checked_`/`saturating_`
families. `visibility.py` listed bare `pub` items in private modules and
`pub(crate)` items with at most one other referencing file; its first part is
noisy (it lists every method of a public type that lives in a private module),
and the sweep refined it by hand.

This verification pass opened every cited site with line numbers, grepped the
use sites each claim rests on, and checked `git log`/`git show`, the
before-prefixed `.agent-notes/`, `crates/before/AGENTS.md`, and
`.cargo/mutants.toml` for recorded rationales. No cargo, just, or test command
ran: no finding turned on a runtime outcome. Two of the sweep's findings
reverse on evidence (the `Shl<i32>` impl has callers; the segments column's
gap is wider than the sweep stated) and one is dropped.

What this pass could not see: the tier-2 instrument modules beyond `meter.rs`
were covered by grep only, as in the sweep; the `Shl<i32>` integer-fallback
claim rests on the language rule and the operand types read at the two sites,
not on a compile check (a compile check would require editing the tree).

## Findings

### inventory-1: Frame-ledger link index caps at u32 and panics on a valid, large decoded input
- Where: crates/before/src/version/skyline/fill/memo.rs:133-139 (related: crates/before/src/version/skyline/fill/prescan.rs:43-45 and 691-693, crates/before/src/version/skyline/fill/prescan.rs:344 and 392, crates/before/src/version.rs:161-182, crates/before/src/party/ops/index.rs:53-57 and 73-75, crates/before/src/version/skyline/query/web.rs:370-372)
- Class / severity / confidence: correctness / medium / high
- Provenance: assessed (read); executed: no
- Verification: confirmed; history: no-rationale-found (prescan.rs:43-45 names the cap; prescan.rs:691-692 defers to it; neither states why 2^32 nonzero links in one scan is unreachable, and the before-prefixed notes do not mention the cap)
- Owner-gated: no (the index width is internal; which resolution to take is the owner's call, see open questions)

`Memo::set_link` converts the running count of nonzero ledger links to `u32`
with an `expect` whose message states the capacity contract, not a proof.
Every link is cleared per fresh scan (`begin_scan`, lines 127-128), so the
trigger is one pre-scan spanning 2^32 or more left-full sites with pairwise
distinct sibling minima. Both decode doors accept inputs of any allocatable
size, so on a 64-bit host this is a panic reachable from caller input, and
neither `Version::tick`/`ticks` nor the `Party`/`Clock` doors carry a
`# Panics` section (a grep for `# Panics` over version.rs, party.rs, and
clock.rs returns nothing). The crate's own choice at the same scale is graceful
fallback, not a panic (`IdIndex::build`, index.rs:73-75).

Evidence:

       133	    pub(super) fn set_link(&mut self, slot: usize, link: Accumulator) {
       134	        // Push first: the store's length is then provably a valid, nonzero
       135	        // 1-based index (the `expect` is the u32 capacity contract alone).
       136	        self.links.push(link);
       137	        let index = u32::try_from(self.links.len()).expect("site count fits u32");
       138	        self.queue[slot] = NonZeroU32::new(index);
       139	    }

    prescan.rs:
        43	//! containers. The ledger's capacity contract is different in kind:
        44	//! stored-link indices fail loudly at their `u32` cap
        45	//! ([`Memo::set_link`]). The fill walk's near-synonymous `depth` (the

       691	        // Ledger slots are queue indices, capped far below `u32::MAX` by the
       692	        // ledger's own link-storage contract.

    index.rs:
        73	        if bits.len() > u64::from(u32::MAX) {
        74	            return IdIndex { bits, rights: None };
        75	        }

    lib.rs:
       351	//! pathological input shapes. Any asymptotic claim is a hard guarantee that the
       352	//! operation will perform in time proportionate to that bound, for all input
       353	//! sizes, no matter how unlikely and contorted the shape of the input.

Resolution: widen the link index so the ledger is bounded only by memory like
every other structure on the walk (`Option<NonZeroUsize>` or `NonZeroU64` in
`queue`; the const assert at memo.rs:97 moves with it), or keep the cap and add
a `# Panics` section to `Version::tick`/`ticks`, `Party::tick`/`ticks`, and
`Clock::tick`/`ticks` stating the bound. Acceptance: either the `expect` is
gone and the contract is memory alone, or the public docs state the cap. The
same u32 shape recurs at web.rs:371 (`expect("freeze count fits u32")`); the
sweep estimates its input requirement at roughly 128 GiB, which I did not
verify.

Construction: build a Party whose id is a right-nested chain of N left-full
nodes (each level: tag `11`, left child the terminal `00`, right child the next
level; 4 bits per level, about 2 GiB for N = 2^32), decoded through
`Party::decode`. Build a Version with an internal node at every chain level
whose right-sibling subtree minima are pairwise distinct (unit-step deltas), so
every site's ledger link is nonzero and the covering root site launches one
fresh pre-scan over the whole chain. `version.tick(&party)` on a 64-bit host
with enough memory reaches `Memo::set_link` once per site
(prescan.rs:344), and the 2^32-th push fails the `u32::try_from` at
memo.rs:137 with `site count fits u32`.

### inventory-2: The segments column cannot fail on any library artifact in any build
- Where: crates/before/src/recurse.rs:100-109 and 118-129 (related: crates/before/src/recurse.rs:16-20 and 71-76, crates/before/Cargo.toml:33-44, crates/before/src/meter.rs:3543-3559, crates/before/tests/meter.rs:22-26 and 387-391, crates/before/src/meter/board/measure.rs:84-92, crates/before/src/meter/board/floors.rs:627-636, crates/before/src/meter/tests.rs:392-454, crates/before/src/clock/tests.rs:565-566)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read recurse.rs in full; read Cargo.toml's dependency tables; grep of `descend!`/`recurse::grow`/`stack_segments` users over src, tests, examples, benches returns only meter.rs's readers, the cfg(test) users in testing/bridge.rs, grow/tests.rs, meter/tests.rs, and the readers in tests/meter.rs and board/measure.rs; every `envelope`/`query_envelope`/`sweep_envelope`/`touch_envelope` constructor call in tests/meter.rs (84 at HEAD) passes 0 in its segments column, checked by extracting the second positional argument); executed: yes (the greps settle that no library kernel references the guard and that every pin is 0)
- Verification: reframed: the sweep found no writer under `--features meter`; the gap is wider. `descend!` and `grow` are `#[cfg(test)]` because `stacker` is a dev-dependency (1ddb5a483), so no library kernel can route through the guard in any build, and a library kernel that recursed natively would never touch the counter either. The counter therefore reads zero over library kernels by construction in every build, including the cfg(test) unit test; history: deliberate-but-expired: the pin was recorded as "the committed ratchet" (`.agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:555-558`) when library kernels still had guarded call sites; once every library walk became iterative and the guard test-only, no library artifact can move the counter, so the ratchet has nothing to catch
- Owner-gated: no (the meter surface is bench/test-only per Cargo.toml's feature comment)

Every envelope in `tests/meter.rs` asserts `segments <= env.segments` with the
pin at 0, the board carries a segments currency declared ceiling-only, and the
crate doc calls the zero "the measured fact the boards' segments column pins".
The only increment sits in `grow` under `#[cfg(test)]`, and `descend!`, the
only route to `grow`, is also `#[cfg(test)]`, so non-test library code cannot
reference it (it would not compile without cfg(test)). The unit test
`stack_segment_meter_counts_deterministically_and_resets` proves the counter
mechanism works over a test-local `dive`, but its library leg (`t.tick(&id)`
reads 0) is true by construction, not by measurement. The committed proof that
library walks do not recurse is `deep_tree_stack_safety` (clock/tests.rs:565,
depth 100 000), which would fail on a native recursion; the segments column
would not.

Evidence:

    recurse.rs:
        16	//! The paper-shaped oracle is clearest written recursively, and the guard is
        17	//! what lets it meet deep inputs safely. The segment counter below stays
        18	//! compiled for the meters: it is the deterministic stand-in for
        19	//! recursion-driven stack consumption, and its zero reading over the library
        20	//! kernels is the measured fact the boards' segments column pins.

        68	#[cfg(any(test, feature = "meter"))]
        69	static SEGMENTS_GROWN: AtomicU64 = AtomicU64::new(0);

       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
       107	        stacker::grow(STACK_GROWTH, f)
       108	    }
       109	}

       118	#[cfg(test)]
       119	macro_rules! descend {
       ...
       128	#[cfg(test)]
       129	pub(crate) use descend;

    Cargo.toml:
        33	[dev-dependencies]
        ...
        44	stacker = { workspace = true }

    tests/meter.rs:
       387	    assert!(
       388	        segments <= env.segments,
       389	        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
       390	        env.segments,
       391	    );

    floors.rs:
       629	const NA_SEG_CEILING_ONLY: &str = "ceiling-only by policy: the target is walks that never grow \
       630	     the stack, so the honest floor is zero and a zero floor asserts nothing";

    meter/tests.rs:
       399	/// Every library walk is iterative, so the meter's liveness needs its own
       400	/// witness: a test-local descent routed through `recurse::descend!` (the same
       401	/// guard the test-only oracle bridge walks use) deep enough to outrun the
       402	/// thread stack. Without that leg, the boards' all-zero segments column could
       403	/// be a dead counter instead of a measured fact.

Resolution: retire the segments column: drop the `segments` field and its
assert from the `Envelope`/`TouchEnvelope` harnesses in `tests/meter.rs`, the
`stack_segments` read in `board/measure.rs`, the segments currency and its
ceiling-only declaration in the board, the `meter::stack_segments` and
`reset_stack_segments` readers, and the `cfg(any(test, feature = "meter"))`
on the static and its accessors (leaving them `cfg(test)`); keep `grow`,
`descend!`, and the counter as the test-surface guard for the oracle bridge,
with `stack_segment_meter_counts_deterministically_and_resets` as that guard's
own liveness test; re-word recurse.rs:16-20 to state that the guard is
test-surface machinery and that `deep_tree_stack_safety` is the committed
no-recursion proof. The alternative, making the column able to fail, requires
promoting `stacker` to an optional dependency under `meter` and routing a
committed known-bad recursive library shape through it, which contradicts the
deliberate dev-dependency placement. Acceptance: no envelope, board cell, or
doc reports a segments reading; `deep_tree_stack_safety` remains the depth
proof; the guard's unit test still runs under `cargo nextest run -p before`.

### inventory-3: AGENTS.md points readers at the retired `implementation` module
- Where: crates/before/AGENTS.md:5-7 (related: crates/before/src/lib.rs:415-454)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'mod implementation' crates/before/src` returns nothing; `find crates/before -name 'implementation*'` returns nothing; `git log -S'pub mod implementation' -- crates/before/src/lib.rs` shows 67970b75 adding it and 22cdfbe1 removing it, whose message reads "The design-essay implementation module is retired."); executed: yes (the greps and git log settle it)
- Verification: confirmed; history: no-rationale-found (a miss in 22cdfbe1's doc sweep)
- Owner-gated: no

The crate guidepost routes readers to a public `implementation` module for
the design essay; lib.rs declares no such module.

Evidence:

         5	crate docs for the model (`Party`/`Version`/`Clock`, the Law of
         6	Disjointness) and the public `implementation` module for the design essay,
         7	`version/skyline.rs` for the stored coding and its operation kernels, and

Resolution: remove the `implementation` clause, or point at where the essay's
content lives now (22cdfbe1 says the `Span` type docs carry the operation
table, algebra, and wire form). Acceptance: every module the guidepost names
exists in lib.rs's module list.

### inventory-4: O(n) debug asserts recompute invariants the exhaustive ledger suite already checks, and one of them is limb-metered
- Where: crates/suanpan/src/accumulator.rs:1391-1395 (related: crates/suanpan/src/accumulator.rs:1293-1296 and 1356-1368, crates/suanpan/src/accumulator/tests/ledger.rs:31-60 and 227-239, crates/before/src/codec/base.rs:416-424 and 284-289, justfile:866-871)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read accumulator.rs's `add_at` and `enter_digit_engine`, ledger.rs's `assert_ledger_invariants` and the exhaustive driver, base.rs's `Sub` and `Ord` impls, and the justfile's board recipe comment); executed: no
- Verification: confirmed, with one sharpening at base.rs:421; history: deliberate (the comment at 1356-1368 argues the assert guards future exits added inside the carry loop) but the exhaustive driver checks the same clauses after every step of every schedule, which is the doctrine's condition for retiring a recompute assert
- Owner-gated: no

`add_at` ends with a scan of every digit above `top`, and `enter_digit_engine`
scans the whole buffer; both restate clauses the committed exhaustive driver
`ledger_invariants_hold_exhaustively` asserts after each step (ledger.rs:34,
52, 57). They make each write O(held digits) in dev-profile builds. The
sibling site in `before`, `debug_assert!(self >= *rhs)` in `Sub for Base`,
duplicates the backend's own underflow panic and, because `Ord for Base`
records limbs (base.rs:286), it is itself metered work: `just test-all` runs
`tests/meter.rs` in the dev profile, so every pinned limb envelope over a
subtraction-heavy scenario counts the assert's compare as well as the
subtraction. The justfile already names this hazard for the board (release is
the profile of record because "debug assertions perform metered work (Base
comparisons through the limb shim...)"), but the integration envelopes are
pinned under exactly that scaffolding; removing the assert later would move
them.

Evidence:

      1391	        debug_assert!(
      1392	            (self.top == 0 || self.digits[self.top] != 0)
      1393	                && self.digits[self.top + 1..].iter().all(|&digit| digit == 0),
      1394	            "top must rest on the highest nonzero digit at add_at exit"
      1395	        );

      1293	        debug_assert!(
      1294	            self.digits.iter().all(|&digit| digit == 0),
      1295	            "a retired register leaves the digit engine idle"
      1296	        );

    ledger.rs:
        51	    assert!(
        52	        acc.digits[acc.top + 1..].iter().all(|&digit| digit == 0),
        53	        "nonzero digit above top {} after {schedule:?}",
        54	        acc.top
        55	    );
        56	    assert!(
        57	        acc.top == 0 || acc.digits[acc.top] != 0,
        58	        "top {} rests on a zero digit after {schedule:?}",
        59	        acc.top
        60	    );

    base.rs:
       419	    fn sub(self, rhs: &Base) -> Base {
       420	        meter_limbs2(&self, rhs);
       421	        debug_assert!(self >= *rhs, "Base subtraction underflow");
       422	        Base(self.0 - &rhs.0)
       423	    }

       284	impl Ord for Base {
       285	    fn cmp(&self, other: &Self) -> Ordering {
       286	        meter_limbs2(self, other);

    justfile:
       866	# The board runs at the release profile, the profile of record: debug
       867	# assertions perform metered work (Base comparisons through the limb shim,
       868	# metered probe cursors), so a dev board measures algorithm plus
       869	# verification scaffolding while release measures the production work
       870	# alone. A dev run (`cargo run -p before --example amp_board ...`) remains
       871	# a legitimate debugging view; its numbers must never be pinned anywhere.

Resolution: drop the two O(n) accumulator asserts, or reduce each to its O(1)
clause (`self.top == 0 || self.digits[self.top] != 0`) if a local probe is
wanted; drop the `Sub for Base` assert (the backend panics on underflow with
its own message). Any limb envelope in `tests/meter.rs` that moves is a
measured improvement to re-pin, attributed to the assert's removal.
Acceptance: the exhaustive ledger driver still passes; no dev-profile assert
performs a metered `Base` comparison.

### inventory-5: `Shl<i32> for Base` exists only to accept unsuffixed literal shifts in two tests, and says nothing about it
- Where: crates/before/src/codec/base.rs:448-455 (related: crates/before/src/codec/tests.rs:52-54 and 526-528, crates/before/src/codec/base.rs:439-446 and 480)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `i32` over every `.rs` under crates/before and crates/suanpan, all detached workspaces included, finds no explicit `i32` shift; a second grep for unsuffixed literal shifts finds `n << 64` at codec/tests.rs:54 with `n: Base` from line 52 and `value << 64` at :528 with `value: Base` from line 526); executed: yes (the greps settle where the callers are; the selection rule is read, not compiled)
- Verification: reframed: the sweep reported no caller anywhere; with `Shl<u32>`, `Shl<i32>`, and `Shl<u64>` all implemented, an unsuffixed literal operand leaves `Base: Shl<{integer}>` ambiguous, integer fallback resolves it to `i32`, and those two sites select this impl; deleting it would fail them with E0277; history: the impl predates the crate rename (eecf92295); no rationale recorded
- Owner-gated: no (`Base` is crate-private)

The impl carries a debug-only guard and, in release, wraps a negative operand
through `as u32` into a huge shift. Nothing at the impl says its purpose is to
let unsuffixed literals compile, so a reader cannot tell whether it is dead,
load-bearing, or a hazard.

Evidence:

       448	impl Shl<i32> for Base {
       449	    type Output = Base;
       450	
       451	    fn shl(self, rhs: i32) -> Base {
       452	        debug_assert!(rhs >= 0, "Base left shift must be non-negative");
       453	        self << rhs as u32
       454	    }
       455	}

    codec/tests.rs:
        52	        let mut n = Base::ZERO;
        53	        for limb in limbs {
        54	            n = (n << 64) | Base::from(limb);
        55	        }

       526	        let mut value = Base::from(n);
       527	        for limb in limbs {
       528	            value = (value << 64) | Base::from(limb);
       529	        }

Resolution: suffix the two literals (`64u32`) and delete the impl, leaving only
the unsigned shift forms; or keep the impl with a one-line comment naming the
literal-fallback purpose and turn the guard into a hard assert so the release
wrap cannot happen. Acceptance: no `Shl<i32>` impl, or one whose comment
states why it exists and whose negative operand panics in every profile.

### inventory-6: `id_node` re-validates the whole subtree at every tuple level
- Where: crates/before/src/codec/literal.rs:52-66 (related: crates/before/src/party.rs:838-844 and 897-899, crates/before/src/codec/text.rs:75-84, crates/before/src/version/skyline/literal.rs:32-50, crates/before/src/version.rs:1486-1488, crates/before/src/clock.rs:960-962)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (read); executed: no
- Verification: reframed: the sweep filed this as a broken O(n) claim; the per-level `validate_id` is redundant work (the two collapse tests establish normal form over normal children, so the only non-normal nodes over normal children are `(0, 0)` and `(1, 1)`), but the doors remain O(n·d) from the per-level child copy at lines 59-63 and the per-level rescan in `skyline::literal::node`, where d is the literal's nesting depth, a compile-time constant bounded by the compiler's recursion limit, so the documented O(n) stands with a compile-time constant factor; history: no-rationale-found
- Owner-gated: no

The text door validates once at the top (`parse_id_str`, text.rs:82); the
tuple door validates once per nesting level, a full re-parse of bits the
collapse tests already established as normal.

Evidence:

        59	    let mut b = BitsBuf::with_capacity(2 + l.len() + r.len());
        60	    b.push(!l.is_empty()); // bit 0 = left present
        61	    b.push(!r.is_empty()); // bit 1 = right present
        62	    b.extend_from_buf(l);
        63	    b.extend_from_buf(r);
        64	    validate_id(super::buf::built_view(&b))?;
        65	    Ok(b)

    text.rs:
        78	    parse_id_tree(&mut cur, &mut bits)?;
        79	    if cur.peek().is_some() {
        80	        return Err(Parse::Syntax); // trailing junk
        81	    }
        82	    validate_id(super::built_view(&bits))?;

Resolution: drop the `validate_id` call from `id_node` and validate once in
the tuple `TryFrom` impl at party.rs:911 (the text path already validates in
`parse_id_str`, so `finish_id` is not the place); update `id_node`'s doc line
51 ("then validates the result"). Acceptance: the literal-door tests and the
differential suites pass unchanged; `validate_id` runs once per top-level
literal.

### inventory-7: Cast to u32 precedes the range assert it is meant to be guarded by
- Where: crates/before/src/codec/build.rs:154-159 (related: crates/before/src/codec/build.rs:141-143, 302, 321-322)
- Class / severity / confidence: correctness / nit / high
- Provenance: assessed (read); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`patch_bit` truncates `at - committed` to `u32` before asserting it is below
`staged_len` (at most 64), so a programmer-error position of the form
`committed + k·2^32 + small` passes the assert and patches a staged bit,
while the function's `# Panics` promises a panic for any `at` at or past the
output.

Evidence:

       141	    /// # Panics
       142	    ///
       143	    /// Panics if `at` is at or past the current output length.
       ...
       154	        } else {
       155	            let offset = (at - committed) as u32;
       156	            assert!(
       157	                offset < self.staged_len,
       158	                "patch position {at} is past the output"
       159	            );

Resolution: compare at u64 width first (`assert!(at - committed <
u64::from(self.staged_len), ...)`) and cast after; `bit_at` (321-322) and
`read_bits` (302) can take the same shape. Acceptance: the assert's operand is
never narrowed before the comparison.

### inventory-8: `#[allow(clippy::result_large_err)]` on `Clock::join_all` carries no local justification
- Where: crates/before/src/clock.rs:257 (related: crates/before/src/clock.rs:373-376)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (the sweep's allow inventory with context shows every other tier-1 allow carries an adjacent rationale; I read both clock.rs sites); executed: yes (grep)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The rationale lives on the sibling at 373-375 and refers back ("as in
`join_all`"); a reader at 257 has to find it.

Evidence:

       257	    #[allow(clippy::result_large_err)]
       258	    pub fn join_all<I: IntoIterator<Item = Clock>>(

       373	    // The combine closure's `Err` is the operand pair the counter keeps (the
       374	    // fold's drop-nothing policy, as in `join_all`); boxing it would spend an
       375	    // allocation on every refusal to dodge a by-value move.
       376	    #[allow(clippy::result_large_err)]

Resolution: place the one-line rationale above line 257 (and let 373-375 say
"as in `join_all`" or drop the back-reference). Acceptance: every allow in
the crate has its reason at the site.

### inventory-9: Dead `len == 64` arm under a `len <= 63` assert in `BitStack::push_bits`
- Where: crates/before/src/codec/stack.rs:61-71
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (read; callers at gamma.rs:43-44 and stack.rs:221-229 pass at most 63); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

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

Resolution: two asserts with messages (`len <= 63`; `value >> len == 0`) and
`self.top = (self.top << len) | value;`. Acceptance: no branch on `len == 64`
remains.

### inventory-10: Two doc slips: `span_all` links "span" to `Version::meet`; "seach" typo
- Where: crates/before/src/version.rs:603-604 (related: crates/before/src/span.rs:413-415)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read both sites); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Evidence:

       603	    /// Prefer this to iteratively [`span`](Version::meet)ing [`Version`]s
       604	    /// one-at-a-time, as it is more efficient.

    span.rs:
       413	    /// A [`Span`] requires two causal comparisons: one to compare the two `lo`
       414	    /// endpoints and a second to compare the two `hi` endpoint, where seach
       415	    /// comparison costs:

Resolution: link to `Version::span`; "seach" to "each" and "endpoint" to
"endpoints". Acceptance: the rendered docs read correctly.

### inventory-11: Capacity hint spelled as an `expect` where the crate elsewhere degrades to zero
- Where: crates/before/src/version/rank.rs:838-843 (related: crates/before/src/version/rank.rs:628-632, crates/before/src/codec/buf.rs:68-78, crates/before/src/codec/build.rs:61-72)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read the three constructors and the one caller); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The hint at the caller is a bit count derived from an in-memory rank's width,
so the `expect` is unreachable by a memory argument the message does not make;
the sibling constructors chose the total form and documented the hint as a
hint.

Evidence:

       838	    fn with_capacity_bits(bits: u64) -> BitSink {
       839	        BitSink {
       840	            bytes: Vec::with_capacity(usize::try_from(bits.div_ceil(8)).expect("output fits")),

    buf.rs:
        73	    pub(crate) fn with_capacity(bits: u64) -> Self {
        74	        BitsBuf {
        75	            bytes: Vec::with_capacity(usize::try_from(bits.div_ceil(8)).unwrap_or(0)),

Resolution: `usize::try_from(bits.div_ceil(8)).unwrap_or(0)`, with the hint
sentence from buf.rs:70-72. Acceptance: no capacity constructor can panic.

### inventory-12: Visibility: `pub fn` inside `pub(crate)` modules, and an unnameable `Base` in a feature-public signature
- Where: crates/before/src/version/skyline/query.rs:422 (related: crates/before/src/codec.rs:41, crates/before/src/lib.rs:417, crates/before/src/version/skyline.rs:157-160, 177, 182, 186, crates/before/src/version/skyline/fill.rs:209 and 247, crates/before/src/version/skyline/masked.rs:104 and 127, crates/before/src/meter.rs:73)
- Class / severity / confidence: modularity / nit / high
- Provenance: verified (grep for bare `pub` items over every `pub(crate)` module under skyline, version, and codec finds exactly `fill::tick`, `fill::ticks`, `masked::causal_cmp`, `masked::eq`, and the two traffic structs that meter.rs:77 and 81 re-export; `Base` is re-exported only at codec.rs:41 inside the private `codec` module, and skyline.rs:157-160 re-exports `Bits`, `BitsBuf`, `BitsView` but not `Base`); executed: yes (grep)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Under `--features meter`, `before::meter::skyline::query::min_ticks` is
reachable and returns a type no caller can name. Separately, four functions
read as public API while being crate-visible.

Evidence:

    query.rs:
       422	pub fn min_ticks(bits: BitsView<'_>) -> Base {

    skyline.rs:
       177	pub(crate) mod fill;
       ...
       182	pub(crate) mod masked;

    fill.rs:
       209	pub fn tick(event: BitsView<'_>, id: &crate::Party) -> BitsBuf {
       247	pub fn ticks(event: BitsView<'_>, id: &crate::Party, n: &Base) -> BitsBuf {

    masked.rs:
       104	pub fn causal_cmp(
       127	pub fn eq(

Resolution: re-export `Base` beside `Bits`/`BitsBuf`/`BitsView` at
skyline.rs:157-160, or return `Ticks` from the meter-facing `min_ticks`;
narrow `fill::{tick, ticks}` and `masked::{causal_cmp, eq}` to `pub(crate)`.
Acceptance: every reachable public signature under `meter` names only
reachable types; no bare `pub` item sits in a `pub(crate)` module without a
re-export.

### inventory-13: `Party::forks(u64::MAX)` yields one share fewer than the public doc promises
- Where: crates/before/src/party/forks.rs:111-114 (related: crates/before/src/party/forks.rs:78-84 and 122-132, crates/before/src/party.rs:239-249 and 274-276, crates/before/src/clock.rs:192-194)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Verification: confirmed; history: deliberate-and-holds for the saturation (the internal comment records it); the public contract does not state it
- Owner-gated: yes (a public doc clause, or a parameter type change)

`Forks::new` saturates `k + 1`, so at `k == u64::MAX` the iterator yields
`u64::MAX - 1` shares and `len()` (an `ExactSizeIterator`, lines 127-132)
reports that in O(1), while the public `Forks` doc says "Yields exactly `k`
disjoint shares".

Evidence:

        80	/// Yields exactly `k` disjoint shares produced one at a time. The party it

       111	        // The count saturates: at `k == u64::MAX` the residual's headroom is
       112	        // spent and the iterator yields one share fewer than asked.
       113	        let whole = mem::replace(party, Party::anonymous());
       114	        let mut split = Split::new(whole, k.saturating_add(1));

Resolution: add the saturation clause to the `Forks`, `Party::forks`, and
`Clock::forks` docs, or take a count type that excludes the edge.
Acceptance: the public docs and `len()` agree at every `k`.

### inventory-14: `add_at` doc claims "any i128 magnitude" but the carry arithmetic overflows near the extremes
- Where: crates/suanpan/src/accumulator.rs:1333-1334 (related: crates/suanpan/src/accumulator.rs:39-43, 1374, 1382, 1235-1237, 1473-1476, 1247-1251)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read the constants, the loop, and the callers at 556-570, 872-877, 1226-1237, 1314-1326, 1472-1488); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

With `LAZY_LIMIT = 2^33` and `RECENTER_BIAS = 2^31`, `i128::from(digit) +
value` and `total + RECENTER_BIAS` overflow for `value` within 2^33 of
`i128::MAX` (a debug panic, a release wrap). Every caller I read stays far
inside: line 1235 states "At most 96 bits after the sub-digit shift",
1473-1474 states "At most 33 + 31 bits per contribution", and 564 and 1318
shift a digit below 2^33 by less than 32. The register path's own doc
(1247-1251) states its bound precisely; this one overstates.

Evidence:

      1333	    /// Add `value` (any sign, any `i128` magnitude) into the digit at
      1334	    /// `pos`, carrying upward until every touched digit is in the zone.
      ...
      1374	            let total = i128::from(self.digits[pos]) + value;
      ...
      1382	                let carry = (total + RECENTER_BIAS) >> DIGIT_BITS;

Resolution: state the real precondition (`|value| < 2^127 - 2^33`, or the
tighter bound the callers obey) and, if wanted, debug-assert it. Acceptance:
the doc's stated domain is one the loop handles without overflow.

## Positives

- Zero `unwrap()` in shipped `before`/`suanpan` code: the five grep hits are
  doctest lines (lib.rs:68, 73; shape.rs:48, 55, 56). The one shipped `panic!`
  (skyline/build.rs:172) carries its proof in the message.
- `stacker` is a dev-dependency (Cargo.toml:44), so downstream graphs build
  no platform stack-manipulation code; the `#![forbid(unsafe_code)]` claim
  in AGENTS.md:24-25 is accurate to the dependency table.
- The debug asserts on hot decode paths are O(1) by stated policy and the
  policy names its reason: literal.rs:11-14 ("asserted work here would make
  dev builds meter a different program than the release board of record"),
  and the justfile (866-871) carries the matching profile-of-record decision
  for the board.
- The capacity-hint constructors in buf.rs:68-78 and build.rs:61-72 document
  the hint as a hint and take the total form; finding inventory-11 is the one
  straggler.
- Every `#[allow]` in the shipped tiers but one (inventory-8) names what the
  lint would flag and why it is accepted at the site, and the five
  `rustdoc::private_intra_doc_links` allows scope the lint per module so it
  stays live everywhere else.
- `IdIndex::build` (index.rs:53-57, 73-75) degrades to the unindexed walk past
  `u32::MAX` bits instead of panicking; it is the pattern inventory-1 asks the
  frame ledger to follow.
- The `Forks` saturation edge is at least recorded at the code (forks.rs:111-
  112); inventory-13 is a doc-surface gap, not a hidden behavior.

## Open questions for Finch

1. inventory-1: widen the frame-ledger link index (8 bytes per site instead
   of 4, the ledger then bounded by memory alone, matching `IdIndex`'s
   choice), or keep the cap and surface it in the public `tick`/`ticks`
   `# Panics` sections? I recommend widening: the crate docs promise every
   bound for all input sizes, and the memory trade is a constant factor.
2. inventory-2: retire the segments column outright (my recommendation), or
   promote `stacker` to an optional dependency under `meter` and give the
   column a committed known-bad shape it can fail? The second contradicts
   1ddb5a483's deliberate dev-dependency placement and still could not see a
   natively recursive kernel; `deep_tree_stack_safety` already does.
3. inventory-12: the sweep suggests `#![warn(unreachable_pub)]`. Under
   default features `pub mod skyline` becomes `pub(crate)` (version.rs:26-29),
   so the lint would fire on every `pub` item under skyline in the non-meter
   build; adopting it means either cfg-conditional allows or accepting it only
   in the meter build. Worth it, or leave this to review?
4. inventory-4: is the `tests/meter.rs` limb envelope's inclusion of
   debug-assert compares (the dev profile is the one `just test-all` runs)
   an accepted property of that suite, in which case the justfile's
   "must never be pinned anywhere" for dev numbers deserves a caveat, or a
   gap the board's release-only rule was meant to close everywhere?

## Dropped

- Sweep [12] (five near-identical `rustdoc::private_intra_doc_links` allows
  could be one): per-module scoping keeps the lint live in every other module;
  the repetition is the price of narrow scope, not a maintenance cascade, and
  the sweep itself rated it taste-level with no cost beyond the repeated
  paragraph.
