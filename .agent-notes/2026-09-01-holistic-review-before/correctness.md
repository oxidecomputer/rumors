# Correctness

This document collects every finding of the `before` and `suanpan` review whose primary class is correctness: what is wrong, or could be wrong under some input, in the two crates, their detached workspaces (`fuzz/`, `fuzzfit/`, `wasm32-pins/`, `surfacecheck/`, `before-fuelscape`), `surface-scan`, and the recipes and CI workflow that verify them, all at commit `9e5784fb4dce977cfbdfd1619886d1482b5ce764`. Harness bugs that could mask a production failure belong here alongside production defects; findings of other classes that bear on an entry are cross-referenced by id and never restated. Ids are `<partition or sweep key>-<n>` (for example `suanpan-24`, `gate-legs-1`); the full record of each, including the lens reports it merged and the refutation and history verdicts, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. Severity is high (a stated contract is broken on a reachable input, or a gate of record is red), medium (a contract clause is breached on a constructible input, or an instrument passes an artifact it exists to refuse), low (a narrower breach, a check that proves less than its sentence, or a defect a gate would catch), and nit (a corner no committed path reaches, with a one-line fix). Provenance uses four words: *demonstrated* means a constructed test or program was run by the witness pass and produced the claimed behavior; *executed* means a run settled the claim (a test, a script over the exact constants, a CI log); *verified* means the claim was mechanically checked or re-derived (grep, git, a hand trace against the pinned dependency source, arithmetic over the quoted lines); *assessed* means read. Each entry carries the finalizer's provenance line and, where the witness pass constructed it, a `Witness` block quoting the run verbatim. Every anchor below was re-read against the tree for this document: the working tree at writing time is `7440d1a3`, two commits past `9e5784fb`, and the diff between them is confined to `.agent-notes/`, so the in-scope files are byte-identical at both commits and every cited line matched its quoted text. No anchor needed correction. The three `tools/` entries (tools-17, tools-22, tools-25) were merged after the partition's own finalization; every line-numbered excerpt in them was checked mechanically against `9e5784fb` at merge time and matched.

## Highest-value items

1. `suanpan`'s wide-operand entry points compute digit landing positions with unchecked `usize` addition after a checked `shift / 32`, and skip zero limbs before any buffer growth, so on a 32-bit release build a shift near `usize::MAX` digits deposits the operand at digit 0 with no panic and no allocation failure, against the `# Panics` contract that promises one or the other; the crate names 32-bit targets as supported, and no first-party 32-bit run of suanpan exists; the review's 32-bit pass demonstrated the wrong value in the tree's `wasm32-pins` executor, where the unchecked guest returns 5 and 1 with no trap and the checked guest traps (suanpan-24).
2. The `coverage` CI job is red at HEAD on a `before` tree byte-identical to the last green run: `masked_cmp_hole_envelope` reads a peak heap of 1156 B under `cargo llvm-cov` against a 480 B pin that the uninstrumented run meets at 384 B, so the heap column is not a function of the tree under that leg and the gate, which never runs the leg, cannot tell a committer main is red (gate-legs-1).
3. `Forks`, `party::Forks`, and `Split` implement `ExactSizeIterator` while their `size_hint` deliberately returns `(usize::MAX, None)` for counts past `usize`, so on the pinned `wasm32-unknown-unknown` target `Clock::seed().forks(1u64 << 32).len()` panics from caller input through std's default `len`; the one committed boundary test is 64-bit-only and no wasm32 pin covers `forks`; the 32-bit pass demonstrated the trap at exactly `k = 2^32`, with `k = 2^32 - 1` returning `usize::MAX` (clock-17, party-13; owner-gated, public trait impls).
4. The fill walk's frame ledger indexes its nonzero links behind a `NonZeroU32` and panics through `expect("site count fits u32")` once one fresh pre-scan records 2^32 links, a canonical input the decode doors admit on a 64-bit host, while `Version::tick`, `Party::tick`, and `Clock::tick` carry no `# Panics` section and the crate's own `IdIndex` degrades gracefully at the same width (inventory-1, with skyline-fill-grow-23 and recursion-5 recording the same defect at other severities).
5. The shared `surface-scan` extractor recognizes exactly two line shapes and silently drops `pub const fn`, `pub async fn`, `pub unsafe fn`, and any `pub fn` at an indent other than 0 or 4, contradicting its one stated invariant that an unexpected `pub fn` panics rather than vanishing; `before` is rescued by surfacecheck, but suanpan's claims roster has no second extractor, so a `const` constructor there leaves the roster total and wrong (surface-roster-28; demonstrated).
6. The verdict matrix's `weave_pair()` joins events over sibling quarters of one half, which ITC normal form collapses to exactly `scatter_pair()`'s value, so the `Eq`/`Hash` intern dedups the Weave family away and the coverage floor, which checks only nonemptiness, cannot see that the family contributes nothing (tests-other-27; demonstrated).
7. Two comments in the fuzz seed derivation describe the sibling's version as "concurrent to the clock's own history" after `clock.sync(&mut sibling)` has already made the two versions equal, so the `clock_then_msg` seed compares a clock against its own version and `laws_family` seeds `{v, v, empty}`; the corpus never represented the relation its comments claim (tests-other-26; demonstrated).
8. The fuelscape overlay's "dense × self" and "hugeleaf × self" points feed byte-identical operands to eight rows whose kernel opens with `codec::canonical_eq`, so the marked adversarial frontier on `version_join`, `version_meet`, `version_span`, `distance`, `lag`, `ranked_cmp`, and both conjoin rows is the cheapest path of each operation, and the mislabel is already in the committed compact datasets (fuelscape-pipeline-23).
9. The fuelscape widget's `typesetDocMath` runs unconditionally at load on every rustdoc page the workspace-wide header reaches, replacing every lone-lowercase-letter code span in `rumors`, `suanpan`, and `before-viz` docs with an italic math variable, while the justfile says the script is inert off `.fuelscape` elements (fuelscape-render-27; demonstrated under node with a stub DOM).
10. `tools/mutantcheck` refuses any `cargo mutants --version` other than the pinned `27.1.0`, but CI installs `cargo-mutants` unpinned beside a pinned `cargo-rdme@2.1.0`, so the `mutants-list` leg turns red on an untouched tree the day the install action's manifest advances, and the step comment's rationale for not pinning is false for this tool (gate-legs-2).
11. Every serde `Serialize` impl emits the data model's `bytes` type while every `Deserialize` requests a `seq` through `<Vec<u8>>::deserialize`, whose visitor implements only `visit_seq`; a conforming strict Deserializer rejects what `before` itself wrote with `invalid type: byte array, expected a sequence`, which the three tested formats cannot show because each bridges the two types (crate-root-34; demonstrated with a twenty-line Deserializer).
12. The wide-gamma width guard rejects a mantissa width only when it fails `usize::try_from`, but dashu's buffer holds at most `usize::MAX / WORD_BITS` words, so on a 32-bit target a decoded stream with `k` in `[2^32 - 32, 2^32 - 1]` reaches a release `assert!` inside the backend where the comment promises a `NotCanonical` reject; the borsh reader's per-bit path reaches it after about 512 MiB of prefix; the 32-bit pass demonstrated the trap at `2^32 - 32` zeros and a typed error at `2^32 - 33` (codec-bits-23).

## Crate-wide patterns

- **Narrowing conversions and fixed-width indices whose bound is asserted rather than derived.** Seven entries share one mechanism: a `u32` or `usize` boundary is crossed with `expect`, an `as` cast, or an unchecked addition, and the sentence beside it states the bound as a fact of the input rather than deriving it. The production instances are the `u32` link index (inventory-1, skyline-fill-grow-23, recursion-5), the `u32` epoch (skyline-query-24), the `usize` mantissa guard against dashu's smaller cap (codec-bits-23), the `ExactSizeIterator` default `len` over a `u64` count (clock-17, party-13), and suanpan's digit positions (suanpan-24); the codec's `patch_bit` truncates before it asserts (clippy-pedantic-2, inventory-7), and the wasm32 guest's own synthesizer wraps at 2^30 bytes (fuzz-guests-pins-27). Every 32-bit instance is unpinned in the tree because no first-party 32-bit run reaches `forks`, suanpan, or the borsh reader; the review's 32-bit pass ran each construction in the `wasm32-pins` guest and observed the predicted behavior (the Witness lines below), and that guest is the natural home for each red-first pin.
- **The marker-bit size law was not carried into the board's adapters.** `d800957e` made `encode().len() == (encoded_bits() + 1).div_ceil(8)`; `version_output_bytes`, four ops.rs denominators, and `truncated_bytes`'s rationale still compute or argue from the unmarked law (board-families-floors-judge-27, board-frame-15, board-frame-21). Each is one token from correct and each disagrees with the `as_bytes().len()` the input side already uses.
- **Instruments that measure a short-circuit under an adversarial label.** The overlay self-pairs (fuelscape-pipeline-23), the collapsed weave pair (tests-other-27), the already-synced "concurrent" seed (tests-other-26), and the text defect placed a dozen bytes into the text (board-frame-23) each present the cheapest path of an operation as its frontier. The same genre recurs outside this class in the bench's `partial_cmp` equal row (benches-examples-17) and the board's `version_eq` row (meter-adequacy-7). The repair is uniform: assert the relation the label claims at the derivation site, as `meet_shade` and the `version_eq` fuelscape row already do.
- **Checks that prove less than the sentence beside them.** The line-scan extractor's never-under-report invariant (surface-roster-28, -29, -21), `build.rs`'s untyped JSON comparisons and its `expect("validated")` (fuelscape-render-31, deps-5), positivity enforced three ways for fuel and none for the size axis (fuelscape-render-6, -8), CLI parsers whose messages name constraints the code does not check (benches-examples-8, -24), and the fit floor's two-sample fallback (fuzzfit-bands-7, -16) all share the form. None masks a failure today; each is a validator whose worst passing artifact is not the intended one.
- **Documented panics reachable from input, and documented silence that panics.** `aggregate`'s "programmer error" assert is reached by `--max-bytes` (fuelscape-render-3); the ledger cap, the epoch cap, the `len()` panic, and the dashu assert are reached by decoded bytes or caller counts. In the dual direction, `masked`'s `# Panics` promises that a negative running height sweeps silently while two `debug_assert_ne!` lines fire on exactly that input (skyline-sweep-place-masked-3), and the oracle bridge asserts normal form as a fact while checking nothing (testing-oracles-4).
- **Provenance of the gate and CI is not bound to the pins it judges.** The coverage leg's heap reading moves on an identical tree (gate-legs-1), `cargo-mutants` is version-pinned by the roster but not by the install step (gate-legs-2), and the `instruments` job installs a floating nightly while the recipe it runs names the dated one (surface-roster-1). Each is a red-on-an-untouched-tree path, which is the failure the repository's pinning discipline exists to remove.

## Crate root and public types

No correctness finding survived review in `lib.rs`, `version.rs`, `rank.rs`, `span.rs`, or `causally`: the version-core, rank, and span-causally partitions each report the production kernels correct on every arm traced. The three entries here sit on the public types' edges: the fork iterators' trait impl, and one error variant.

### clock-17: `Forks::len()` panics on 32-bit targets for counts past `usize` (an `ExactSizeIterator` over a `u64` count)
- Where: crates/before/src/clock/forks.rs:52-57 (related: crates/before/src/party/forks.rs:65-76, 122-132; crates/before/tests/forks_max.rs:26, 49; rust-toolchain.toml:31; crates/before/src/iter.rs:16; crates/before/wasm32-pins/)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read the pinned toolchain's `core/src/iter/traits/exact_size.rs:116-123`: default `len` is `let (lower, upper) = self.size_hint(); assert_eq!(upper, Some(lower)); lower`; read `Split::size_hint` returning `(usize::MAX, None)` past `usize`; `rust-toolchain.toml:31` pins `wasm32-unknown-unknown`; grep of `wasm32-pins/guest` and `harness` for `forks`, `size_hint`, `.len()` finds no forks pin; tests/forks_max.rs guards its `len()` checks with `expect("64-bit test host")`); executed: no (no 32-bit host under the review's rules)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (cdad4606 chose `(usize::MAX, None)` as "the standard spelling for counts past usize" and said "the overflow panic dissolves"; nothing addresses the `ExactSizeIterator` interaction)
- Owner-gated: yes (every resolution touches the public API: the `ExactSizeIterator` impls, the count's type, or a documented panic/saturation)
- Witness: demonstrated (32-bit pass, run in the `wasm32-pins` executor; the main pass was inconclusive on the 64-bit host and re-confirmed the mechanism from the toolchain's `core` source, quoted below). `Clock::seed().forks(1u64 << 32).len()` traps in the wasm32 guest, `size_hint()` at that `k` reports an upper bound of `None`, and the adjacency `forks((1u64 << 32) - 1).len()` returns 4294967295 without panicking, so `k = 2^32` is the minimal panicking input.

`Forks<'_>` (and the `party::Forks`/`Split` it delegates to) implements `ExactSizeIterator` while its `size_hint` deliberately returns `(usize::MAX, None)` once the remaining `u64` count exceeds `usize`. On the pinned `wasm32-unknown-unknown` target, `Clock::seed().forks(1u64 << 32).len()` therefore panics from caller input in `O(1)`, and the `size_hint` alone already breaks `ExactSizeIterator`'s exact-bounds contract there. This breaches "panics only for programmer error, never reachable from caller input"; the comment's "standard spelling" is true of `Iterator::size_hint` and incompatible with also implementing `ExactSizeIterator`. The one committed boundary test is explicitly 64-bit-only, and no wasm32 pin covers `forks`.

Evidence:

        52	    fn size_hint(&self) -> (usize, Option<usize>) {
        53	        self.parties.size_hint()
        54	    }
        55	}
        56	
        57	impl ExactSizeIterator for Forks<'_> {}

    (party/forks.rs)
        66	        // The count is u64 and `usize` may be narrower: past its range the hint
        67	        // is `(usize::MAX, None)`, the standard spelling for an iterator of
        68	        // more than `usize::MAX` items.
        69	        (
        70	            usize::try_from(self.remaining).unwrap_or(usize::MAX),
        71	            usize::try_from(self.remaining).ok(),
        72	        )
        76	impl ExactSizeIterator for Split {}

    (core/src/iter/traits/exact_size.rs, toolchain 1.97.1)
       116	    fn len(&self) -> usize {
       117	        let (lower, upper) = self.size_hint();
       122	        assert_eq!(upper, Some(lower));
       123	        lower

    (tests/forks_max.rs)
        26	        let expected = usize::try_from(u64::MAX - 1).expect("64-bit test host");

Witness output (agent 4, mechanical check of the pinned toolchain's core source):

    ```text
       116	    fn len(&self) -> usize {
       117	        let (lower, upper) = self.size_hint();
       118	        // Note: This assertion is overly defensive, but it checks the invariant
       119	        // guaranteed by the trait. If this trait were rust-internal,
       120	        // we could use debug_assert!; assert_eq! will check all Rust user
       121	        // implementations too.
       122	        assert_eq!(upper, Some(lower));
       123	        lower
       124	    }
    rustc 1.97.1 (8bab26f4f 2026-07-14)
    ```

Witness output (32-bit pass; `cargo nextest run --cargo-profile release --no-fail-fast -E 'test(/_checked_guest$/)'` in `crates/before/wasm32-pins`, against the guest built for `wasm32-unknown-unknown` and run under wasmtime; results.md `## 32-bit pass`, clock-17):

    ```text
            PASS [   0.140s] ( 2/11) wasm32-pins-harness::zz_witness clock_forks_len_past_usize_checked_guest
            PASS [   0.141s] ( 4/11) wasm32-pins-harness::zz_witness clock_forks_size_hint_past_usize_checked_guest
            PASS [   0.141s] ( 5/11) wasm32-pins-harness::zz_witness clock_forks_len_at_usize_checked_guest
    ```

    `clock_forks_len_past_usize_checked_guest` asserts `call1("pin_clock_forks_len", 1u64 << 32) == Outcome::Trapped(Trap::UnreachableCodeReached)`; `clock_forks_size_hint_past_usize_checked_guest` asserts the `(_, None)` hint (the export's `-2`); `clock_forks_len_at_usize_checked_guest` asserts `Outcome::Value(4_294_967_295)` at `k = 2^32 - 1`. The trap's attribution to the default `len`'s `assert_eq!(upper, Some(lower))` is by reading, the only assertion on the path: the guest has no stderr.

Resolution: Owner's call among (a) dropping `ExactSizeIterator` from `Forks`/`party::Forks`/`Split` and keeping the accurate `size_hint` (`len()` disappears; iter.rs:16's doc example uses it); (b) taking the count as `usize`; (c) documenting under `# Panics` that `len()` panics past `usize` on narrow targets, or clamping the reserved count at construction and documenting the saturation beside the existing `u64::MAX` one. Whichever is chosen, add a red-first pin to `wasm32-pins` calling `.len()` and `size_hint()` on `forks(1u64 << 32)`. Acceptance: a committed wasm32 pin exercises the boundary under the chosen contract; the rustdoc of `Clock::forks`, `Forks`, `Party::forks`, and `party::Forks` states what happens past `usize`.
Construction: on any 32-bit target (the pinned wasm32 guest): `let mut c = before::Clock::seed(); let it = c.forks(1u64 << 32); let _ = it.len();`. `usize::try_from(4294967297).ok()` is `None`, so `size_hint` is `(usize::MAX, None)` and the default `len` panics on `assert_eq!(None, Some(usize::MAX))`. On 64-bit the same call returns `4294967296`.

Synthesis note (32-bit pass): the Construction's `usize::try_from(4294967297)` names the wrong value. `Forks::new` (party/forks.rs:113-118) builds the split with `k + 1` shares and draws the residual immediately, so `remaining` is `k = 2^32` when `len()` runs; `2^32` also exceeds a 32-bit `usize`, and the adjacency result confirms `2^32` as the minimal panicking `k`. The conclusion is unchanged.

Synthesis note: party-13 below records the same defect from the `party::Forks`/`Split` side; the two partitions' recommendations differ (drop the impl versus saturate `len()`), and the open questions carry one recommendation for both. Related, in other classes: the public "exactly `k` children" contract is false at `k == u64::MAX` (clock-22, party-14, tests-other-19).

### party-13: `Forks`/`Split` implement `ExactSizeIterator`, whose default `len()` panics for `k >= usize::MAX` shares on 32-bit targets
- Where: crates/before/src/party/forks.rs:65-76 (related: crates/before/src/party/forks.rs:132, crates/before/src/clock/forks.rs:57, crates/before/tests/forks_max.rs:26, crates/before/tests/forks_max.rs:49, crates/before/src/party.rs:239-240)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read `size_hint`; read std's default `ExactSizeIterator::len` in the pinned toolchain's `core/src/iter/traits/exact_size.rs:116-124`: `assert_eq!(upper, Some(lower))`; `grep -rn 'forks\|Forks' crates/before/wasm32-pins/` is empty; tests/forks_max.rs:26 and :49 carry `.expect("64-bit test host")`); executed: no (no 32-bit build permitted in this review)
- Seen by: structure; refutation: confirmed; history: no rationale found (cdad46060 widened `k` to `u64` and addressed only the `Iterator` face; the 32-bit `len()` was not examined)
- Owner-gated: yes: a public trait impl's contract
- Witness: demonstrated (32-bit pass, run in the `wasm32-pins` executor; the main pass was inconclusive, agent 1: "No test constructed: the panic needs usize::MAX < 2^32 + 1, i.e. a 32-bit target", and confirmed from the toolchain's `core` source that the default `len` asserts `upper == Some(lower)`). `Party::seed().forks(1u64 << 32).len()` traps in the wasm32 guest, `size_hint()` at that `k` has upper bound `None`, and `forks((1u64 << 32) - 1).len()` returns 4294967295 with no panic, so `k = 2^32` is the minimal panicking input and the panic is O(1) from caller input.

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

Witness output (agent 1):

    ```text
       116:    fn len(&self) -> usize {
       122:        assert_eq!(upper, Some(lower));
    ```

Witness output (32-bit pass; the same guest and harness run as clock-17's; results.md `## 32-bit pass`, party-13):

    ```text
            PASS [   0.142s] ( 6/11) wasm32-pins-harness::zz_witness party_forks_len_past_usize_checked_guest
            PASS [   0.143s] ( 7/11) wasm32-pins-harness::zz_witness party_forks_size_hint_past_usize_checked_guest
            PASS [   0.144s] ( 9/11) wasm32-pins-harness::zz_witness party_forks_len_at_usize_checked_guest
    ```

    `party_forks_len_past_usize_checked_guest` asserts `call1("pin_party_forks_len", 1u64 << 32) == Outcome::Trapped(Trap::UnreachableCodeReached)`; `party_forks_size_hint_past_usize_checked_guest` asserts the `(_, None)` hint (the export's `-2`); `party_forks_len_at_usize_checked_guest` asserts `Outcome::Value(4_294_967_295)` at `k = 2^32 - 1`. The checked/unchecked guest profile is irrelevant here (the panic is an assert, not an overflow check); the run used the checked guest.

Resolution: Owner's choice among: document a `# Panics` on `Forks`/`clock::Forks` for `len()` past `usize::MAX` shares on 32-bit targets; override `len()` to saturate at `usize::MAX` (documented as the one place the count is inexact); or stop implementing `ExactSizeIterator` (an API removal, least attractive). Whichever is chosen, add a wasm32 pin. Nit alongside: `size_hint` converts `remaining` twice; compute `usize::try_from(self.remaining)` once. Acceptance: a committed wasm32-pins test exercises `len()` on `forks(u64::from(u32::MAX) + 1)` under the chosen contract; tests/forks_max.rs loses its `64-bit test host` caveat or states why it remains.
Construction: on any 32-bit target: `let mut p = Party::seed(); let it = p.forks(u64::from(u32::MAX) + 1); let _ = it.len();`. After the residual is drawn, `remaining` is `2^32 + 1`, `size_hint` is `(usize::MAX, None)`, and the default `len()`'s `assert_eq!(upper, Some(lower))` fails.

Synthesis note (32-bit pass): after the residual is drawn `remaining` is `2^32`, not the Construction's `2^32 + 1` (`Split::new(whole, k.saturating_add(1))` then one `next()` at party/forks.rs:114-117); the mechanism and conclusion stand, and the adjacency result confirms `2^32` as the minimal panicking `k`.

Synthesis note: the same defect as clock-17, reached through `Clock::forks` there and `Party::forks` here; one fix at `Split` and the two wrapping impls resolves both.

### api-audit-2: Decode::Io interpolates the io::Error into Display and returns None from Error::source()
- Where: crates/before/src/error.rs:89-91 (related: party.rs:625, clock.rs:789, version.rs:1112, version/rank.rs:463, version/ranked.rs:273)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read error.rs in full: no `#[source]`, `#[from]`, or field named `source`; read `thiserror-impl-2.0.18/src/prop.rs:97-105`, the derive's source-field rule; `Cargo.lock` resolves before's thiserror to 2.0.18); executed: no
- Verification: confirmed; severity lowered from the sweep's medium: the variant's field is public and pattern-matchable, so structured access exists by `match`; only the `dyn Error` chain (anyhow/eyre reports, `ErrorKind` recovery through `source()`) is cut; history: no-rationale-found
- Owner-gated: no: the variant shape is unchanged; only `Display` text and `source()` change

thiserror implements `source()` as the field annotated `#[source]`/`#[from]` or named `source`, else `None`. `Io(io::Error)` has none of these, so a reporter walking the chain sees one flat string and loses the `io::Error`. Interpolating `{0}` into `Display` while also exposing it as the source would duplicate the text in chained reporters, which is why the idiomatic spelling separates them.

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

Synthesis note: the crate-root partition filed the same defect as crate-root-15 (idiom class) and the fresh-eyes sweep observed `source() is_some = false` at runtime; this entry is the correctness face of one fix.

## The skyline coding

The coding kernels (`validate`, `admit`, `build`, `emit`, `text`, `literal`, `shape`, `walk`) and the watermark carry no correctness finding: the skyline-coding partition's two surviving defects are cost arguments (claim class), and the watermark partition traced every arm against suanpan's `Accumulator` contract. The entries here are in the fused tick's frame ledger, the masked comparison, and the min-ticks epoch ledger.

### Fill and grow

### inventory-1: Frame-ledger link index caps at u32 and panics on a valid, large decoded input
- Where: crates/before/src/version/skyline/fill/memo.rs:133-139 (related: crates/before/src/version/skyline/fill/prescan.rs:43-45 and 691-693, crates/before/src/version/skyline/fill/prescan.rs:344 and 392, crates/before/src/version.rs:161-182, crates/before/src/party/ops/index.rs:53-57 and 73-75, crates/before/src/version/skyline/query/web.rs:370-372)
- Class / severity / confidence: correctness / medium / high
- Provenance: assessed (read); executed: no
- Verification: confirmed; history: no-rationale-found (prescan.rs:43-45 names the cap; prescan.rs:691-692 defers to it; neither states why 2^32 nonzero links in one scan is unreachable, and the before-prefixed notes do not mention the cap)
- Owner-gated: no (the index width is internal; which resolution to take is the owner's call, see open questions)
- Witness: inconclusive (agent 3: "not constructed: the trigger needs a ~2 GiB right-nested id and 2^32 distinct-minima sites"; the three cited sites were re-read and quoted)

`Memo::set_link` converts the running count of nonzero ledger links to `u32` with an `expect` whose message states the capacity contract, not a proof. Every link is cleared per fresh scan (`begin_scan`, lines 127-128), so the trigger is one pre-scan spanning 2^32 or more left-full sites with pairwise distinct sibling minima. Both decode doors accept inputs of any allocatable size, so on a 64-bit host this is a panic reachable from caller input, and neither `Version::tick`/`ticks` nor the `Party`/`Clock` doors carry a `# Panics` section (a grep for `# Panics` over version.rs, party.rs, and clock.rs returns nothing). The crate's own choice at the same scale is graceful fallback, not a panic (`IdIndex::build`, index.rs:73-75).

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

Witness output (agent 3, read-only):

    ```text
    memo.rs:133-139: "pub(super) fn set_link(&mut self, slot: usize, link: Accumulator) {\n    // Push first: the store's length is then provably a valid, nonzero\n    // 1-based index (the `expect` is the u32 capacity contract alone).\n    self.links.push(link);\n    let index = u32::try_from(self.links.len()).expect(\"site count fits u32\");\n    self.queue[slot] = NonZeroU32::new(index);\n}"
    prescan.rs:344: "                self.memo.set_link(slot, head);"
    prescan.rs:43-45: "//! containers. The ledger's capacity contract is different in kind:\n//! stored-link indices fail loudly at their `u32` cap\n//! ([`Memo::set_link`])."
    ```

Resolution: widen the link index so the ledger is bounded only by memory like every other structure on the walk (`Option<NonZeroUsize>` or `NonZeroU64` in `queue`; the const assert at memo.rs:97 moves with it), or keep the cap and add a `# Panics` section to `Version::tick`/`ticks`, `Party::tick`/`ticks`, and `Clock::tick`/`ticks` stating the bound. Acceptance: either the `expect` is gone and the contract is memory alone, or the public docs state the cap. The same u32 shape recurs at web.rs:371 (`expect("freeze count fits u32")`); the sweep estimates its input requirement at roughly 128 GiB, which I did not verify.

Construction: build a Party whose id is a right-nested chain of N left-full nodes (each level: tag `11`, left child the terminal `00`, right child the next level; 4 bits per level, about 2 GiB for N = 2^32), decoded through `Party::decode`. Build a Version with an internal node at every chain level whose right-sibling subtree minima are pairwise distinct (unit-step deltas), so every site's ledger link is nonzero and the covering root site launches one fresh pre-scan over the whole chain. `version.tick(&party)` on a 64-bit host with enough memory reaches `Memo::set_link` once per site (prescan.rs:344), and the 2^32-th push fails the `u32::try_from` at memo.rs:137 with `site count fits u32`.

Synthesis note: skyline-fill-grow-23 and recursion-5 record this same defect at low severity, and both mark it owner-gated because the 4-byte queue cell is a committed memory pin (memo.rs:96-97). Three reviewers agree on the mechanism and disagree only on how much a multi-GiB trigger discounts a contract breach; the crate's own "for all input sizes" sentence (lib.rs:351-353) and the doctrine's rule that only infeasible *work* excuses a corner favor this entry's medium. One decision resolves all three, and the epoch cap at web.rs:371 (skyline-query-24) is the same shape one module over.

### skyline-fill-grow-23: `Memo::set_link`'s `u32` link cap is a production panic reachable by input scale, undisclosed at the public tick
- Where: crates/before/src/version/skyline/fill/memo.rs:133-139 (related: crates/before/src/version/skyline/fill/prescan.rs:43-45; crates/before/src/version/skyline/fill.rs:202-208, 241-246; crates/before/src/version/skyline/fill/memo.rs:96-97; crates/before/src/version.rs:161-182; crates/before/src/meter.rs:993-1009)
- Class / severity / confidence: correctness / low / medium
- Provenance: assessed (read `set_link`; read `memo_chain(k, distinct)` and `memo_chain_id(k)`, canonical for every `k`, yielding `k` distinct nonzero links under one covering site; read `Version::tick`'s rustdoc, which has no `# Panics` section); executed: no
- Seen by: correctness (32), claims (42); refutation: confirmed (not constructible here: hundreds of GB of link store); history: deliberate-and-holds for the mechanism (055f2e48 pinned the compactness trade and the const assert; 3b883dd3 states the policy that a width-capped counter fails loudly at its cap; 05d87e1b scoped itself to `usize`-derived caps); the public-door disclosure is the unaddressed residue
- Owner-gated: yes (a documented design decision: compactness vs. cap)

The frame ledger indexes its nonzero links behind a `NonZeroU32` and panics via `expect` when one fresh pre-scan records more than `2^32 − 1` nonzero links. The trigger is a canonical stream and a canonical party (about 13 GB of packed input on the `MemoChain(distinct)` layout), not programmer error, and the `expect` message is an assumption about input size rather than a one-line proof. The cap is disclosed only in prescan.rs's private module doc; `fill::tick`'s `# Panics` names only non-canonical bytes and the empty id, and the public `Version::tick` has no `# Panics` at all. The doctrine's tolerable-unreachability exemption covers `2^64`-iteration corners, not `2^32`-site inputs; the memory-pricing argument (roughly 400 GB of link store before the cap) holds but is the kind of argument the crate's own `u64` denomination work declined to rest on.

Evidence:

       133	    pub(super) fn set_link(&mut self, slot: usize, link: Accumulator) {
       134	        // Push first: the store's length is then provably a valid, nonzero
       135	        // 1-based index (the `expect` is the u32 capacity contract alone).
       136	        self.links.push(link);
       137	        let index = u32::try_from(self.links.len()).expect("site count fits u32");
       138	        self.queue[slot] = NonZeroU32::new(index);
    (prescan.rs)
        43	//! containers. The ledger's capacity contract is different in kind:
        44	//! stored-link indices fail loudly at their `u32` cap
        45	//! ([`Memo::set_link`]). The fill walk's near-synonymous `depth` (the

Resolution: Owner's call between (a) widening the index to `Option<NonZeroUsize>` (8 bytes per queue cell on 64-bit, still niche-packed; update the const assert at memo.rs:97 and the "one index-sized cell per site" claim; re-pin the memo rows' heap columns with attribution) so the only cap is allocatable memory, and (b) keeping the cap and stating it as a capacity contract in `fill::tick`/`ticks`'s `# Panics`, with `Version::tick` and `Party::tick` pointing at it. Either way, the `expect` message should name the capped quantity: "nonzero link count fits u32". Acceptance: no `u32::try_from(...).expect` on the ledger path, or the public tick doors' docs name the cap.

Construction: Build `Shape::MemoChain.packed_flagged(k, true)` × `Shape::MemoChainId.packed1(k)` at `k = 2^32` (each interior site has the distinct minimum `j`, so every sibling link is nonzero and every site lands in one fresh scan under the root covering site), decode both through the public doors, and call `Version::tick`; the `2^32`-th `set_link` panics with "site count fits u32". Needs roughly 13 GB of packed input and hundreds of GB of heap; stated to fix reachability, not as a test to commit.

Synthesis note: same defect as inventory-1; this entry adds the owner history (055f2e48, 3b883dd3) that makes the 4-byte cell a deliberate pin, which is why it is marked owner-gated where inventory-1 is not.

### recursion-5: Memo ledger's u32 link index panics on a feasible input where IdIndex degrades gracefully
- Where: crates/before/src/version/skyline/fill/memo.rs:132-139 (related: crates/before/src/version/skyline/fill/memo.rs:69-97, crates/before/src/version/skyline/fill/prescan.rs:686-694, crates/before/src/party/ops/index.rs:53-57, crates/before/src/party/ops/index.rs:72-75, crates/before/src/party/ops/index.rs:173-178, crates/suanpan/src/accumulator.rs:90-155)
- Class / severity / confidence: correctness / low / medium
- Provenance: assessed (memo.rs read in full; prescan.rs, index.rs, and the Accumulator struct read at the cited sites; no upstream cap on a scan span's site count found, but prescan.rs was not read in full); executed: no
- Verification: reframed: the panic's reachability is set by the link store, not the packed id: 2^32 links need roughly 2^32 times `size_of::<Accumulator>()` bytes (the struct holds an `Option<i128>`, a `Vec<i64>`, two `usize`, and a `BTreeMap`) in addition to the ~2 GiB id and a version with one leaf per site, so on hosts with less heap the `Vec` growth aborts first (the crate's uniform exhaustion mode) and the `expect` fires only on hosts that can hold the store; history: no-rationale-found (memo.rs:134-135 calls the expect "the u32 capacity contract alone"; prescan.rs:691-692 cites a "link-storage contract" that is not derived anywhere)
- Owner-gated: yes (the u32 cell is a committed memory pin, memo.rs:96-97; widening it moves the affected heap envelopes)

A `tick` over a party whose one pre-scan span holds more than `u32::MAX - 1` nonzero-link left-full sites reaches `set_link`'s `expect`. The input is large but constructible, and the doctrine's tolerated corner is one costing infeasible work, not a large allocation. The crate's own `IdIndex` shows the sanctioned shape for a u32-sized table: past the width it falls back to the unindexed walk rather than panicking.

Evidence:

       133	    pub(super) fn set_link(&mut self, slot: usize, link: Accumulator) {
       134	        // Push first: the store's length is then provably a valid, nonzero
       135	        // 1-based index (the `expect` is the u32 capacity contract alone).
       136	        self.links.push(link);
       137	        let index = u32::try_from(self.links.len()).expect("site count fits u32");
       138	        self.queue[slot] = NonZeroU32::new(index);
       139	    }

    prescan.rs:
       691	        // Ledger slots are queue indices, capped far below `u32::MAX` by the
       692	        // ledger's own link-storage contract.
       693	        self.slots.pop(&mut self.values) as usize

    index.rs:
        73	        if bits.len() > u64::from(u32::MAX) {
        74	            return IdIndex { bits, rights: None };
        75	        }

Resolution: Either widen the queue cell to `Option<NonZeroUsize>` (re-pin the affected heap envelopes and the const assert at memo.rs:97), or make the pre-scan launch a fresh scan when the link store would exceed u32 (a scan-span cap mirroring `IdIndex`'s unindexed arm), or rule the corner tolerable and document it at `set_link` with its true reachability (a multi-GiB operand pair and a link store of 2^32 accumulators), replacing the undefined "link-storage contract" at prescan.rs:691-692 with that ruling. Acceptance: either no u32 capacity panic remains on the tick path for any decodable input, or the corner is documented at `set_link` with its reachability and an owner ruling, and prescan.rs cites that ruling.

Construction: Build a `Party` whose packed id chains 2^32 left-full sites under one covering site (each site is a both-present tag `11` followed by the full terminal `00`, then the right child: 4 bits per site, about 2 GiB of id bits), plus a `Version` with one leaf per site so every site has a nonzero link; call `Clock::tick`. On a host that can hold 2^32 `Accumulator`s, expect the panic "site count fits u32" from memo.rs:137; on a smaller host, expect an allocation abort from `links.push` first. Not run: the input is multi-GiB and the link store is hundreds of GiB.

Synthesis note: same defect as inventory-1 and skyline-fill-grow-23; this entry contributes the third resolution (a scan-span cap that launches a fresh scan) and the reachability refinement that the link store, not the id, sets the trigger.

### Comparison kernels

### skyline-sweep-place-masked-3: The masked height debug-asserts panic in debug builds on input the Panics contract says sweeps silently
- Where: crates/before/src/version/skyline/masked.rs:244-250 (related: crates/before/src/version/skyline/masked.rs:252-260, 96-103; crates/before/src/version/skyline/masked/tests.rs:15-72; crates/before/src/version/skyline/sweep.rs:87-93)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (hand trace of the walk state against the code, not run); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed (independent trace); history: no rationale found (the asserts are 6c88c2ad's; 8b1db79d transplanted the sweep's Panics sentence two weeks later and pinned only the collapsible-pair half; neither commit reconciles them)
- Owner-gated: no

The Panics section promises that a delta driving the running height negative "sweep[s] silently"; under `debug_assertions` the two single-owner arms assert the height sign is not `Less` and panic on exactly that input whenever a single-owner interval reads the integrator. The same sentence in sweep.rs is true (the pair sweep has no height assert); masked's copy is false in debug and test builds. Statement faithfulness is breached, and the guard names no failure the committed suites miss: on canonical input a negative height needs a fold-orientation bug, which misreads the interval's sign and separates from the projection laws.

Evidence:

       244	                    let height_sign = self
       245	                        .height_a
       246	                        .as_mut()
       247	                        .expect("a masked `b` maintains h_a")
       248	                        .sign();
       249	                    debug_assert_ne!(height_sign, Ordering::Less, "heights are nonnegative");
       250	                    height_sign

       100	/// operands canonical packed ids. The violations the walk structurally notices
       101	/// (truncation, malformation) panic; the rest (a collapsible sibling pair, a
       102	/// delta driving the running height negative) sweep silently, and the verdict
       103	/// is then unspecified.

Resolution: delete the two `debug_assert_ne!` lines (249 and 258) and keep the contract as written. If the owner prefers the guard, amend the Panics section to say a negative running height trips a debug assertion, and either way extend `collapsible_sibling_pair_sweeps_without_panicking` (or add a sibling) with the negative-height witness so the sentence's second case is pinned. Acceptance: a debug-profile test feeding the witness below through `masked::causal_cmp` and `masked::eq`, in both operand positions, returns without panicking.

Construction: `a` = bits `0` (internal root), `1` + gamma(5) (left leaf, absolute 5), `1` + gamma(11) (right leaf; `unzigzag(11)` is odd, so `(Negative, 11/2 + 1 = 6)`, height -1 on [1/2, 1)); `b` = `1` + gamma(0) (single leaf 0); `b_mask` = packed id `10 00` (left child present and terminal, right absent: owned on [0, 1/2) only). `validate_bits` rejects `a` (skyline.rs:74-75, 251-252), so the witness sits outside the precondition exactly as the collapsible pair does. Trace: `Walk::open` seeds `diff = 5`, `height_a = Some(5)`; round 1 reads (owned, owned) and `Greater`; `advance_set` under priority `[B_MASK, B, A_MASK, A]` steps `B_MASK` (depth 1, flip 1, right child absent so `owned = false`) and ties `A` (depth 1 >= 1), decoding -6 into `diff` (-1) and `height_a` (-1); round 2 is the `(true, false)` arm, `height_a.sign() == Less`, and line 249 fires. In release the call returns `None`.

### Query

### skyline-query-24: `EpochLedger::epoch` narrows the freeze count to `u32` behind an expect that asserts rather than argues, and its doc miscounts
- Where: crates/before/src/version/skyline/query/web.rs:369-372 (related: query/web.rs:177-186, 384-396; query.rs:456-458, 467; codec/bits.rs:117-119)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read `epoch`, `freeze`, the `Reign` struct, and the per-leaf call at query.rs:467; the freeze-cost arithmetic re-derived by hand from the trigger at query.rs:456); executed: no
- Seen by: correctness, claims (the narrowing), prose (the doc); refutation: confirmed; history: no-rationale-found (the bare expect dates to 56a08f90; two later width audits, 6323d667 and 7ea3df58, passed over it; d3a029d4 reworded `freeze`'s doc to name the discard arm and left `epoch`'s doc untouched)
- Owner-gated: no

`epoch()` runs once per leaf from `min_ticks` and converts `drifts.len() - 1` with `expect("freeze count fits u32")`. The message states the conclusion, not the premise, and the bound is reachable from a canonical stream on a 64-bit target: a `+2^288` delta followed by a `+1` trips the trigger (live at 10 digits against 1 funded plus 8 allowed) with nonzero drift, so one freeze costs about 580 code bits and 2^32 freezes need a stream of about 2^41 bits (roughly 311 GB), which the codec admits (bits.rs:117-119). Principle 1: panics only for programmer error, every expect message a one-line proof, and input size carries no weight; the one tolerated corner is infeasible work, which 2^41 bits is not. Separately, the doc says "the freezes so far" but `freeze` discards a redundantly spelled zero without pushing, so the epoch counts parked drifts, and the next method's own doc ("or discard a redundantly spelled zero, keeping the epoch") contradicts this one.

Evidence:

    369      /// The current epoch: the freezes so far.
    370      pub(super) fn epoch(&self) -> u32 {
    371          u32::try_from(self.drifts.len() - 1).expect("freeze count fits u32")
    372      }

    (web.rs:388-393)
    388          if drift != UBig::ZERO {
    389              self.drifts.push((
    390                  Sign::from_is_negative(sign == Ordering::Less),
    391                  Base::from(drift),
    392              ));
    393              self.refs.push(0);

Resolution: Make `Reign::epoch` and `epoch()` `usize` (or `u64`), drop the `try_from`/`expect`, and index `refs` with it directly (`Reign` is a heap-owned per-boundary payload, so the width change costs at most a few bytes of alignment per stacked boundary). If the owner instead rules a 2^41-bit operand infeasible, rewrite the message as the argument ("a freeze costs at least 2^9 stream bits, so 2^32 freezes need a 2^41-bit stream"). Either way, reword the doc: "The current epoch: the drifts parked so far (a freeze that finds no drift keeps the epoch)." Acceptance: no `u32` conversion remains on the epoch path, or the expect reads as a proof from a stated bound; the two method docs agree; min_ticks value pins unchanged.
Construction: A right spine of 2^32 + 1 two-leaf blocks whose heights step by `+2^288` then `+1`: after the `+2^288` fold `live.digit_count() = 10` and `int_digits = 10` (no trigger); after the `+1` fold `10 > 1 + 8` fires `EpochLedger::freeze` with nonzero drift, pushing one epoch per block. At the 2^32-th block's leaf, `ledger.epoch()` at query.rs:467 fails the `u32::try_from`. Stream size about 2^32 · 580 bits; construct through `Version::decode` on a 64-bit machine with enough memory rather than running it at test scale.

Synthesis note: the inventory sweep estimated this trigger at roughly 128 GiB without verifying it; the partition's hand derivation (about 311 GB) is the figure of record. Same shape as the ledger cap (inventory-1); the resolution is the same widening.

## The codec

### Bits

### codec-bits-23: The wide-gamma width guard sits at usize::MAX, above dashu's capacity cap: 32-bit decode can panic inside dashu
- Where: crates/before/src/codec/gamma.rs:243-253 (related: crates/before/src/codec/dsi.rs:248-267; crates/before/src/borsh_impls.rs:119-125; crates/before/wasm32-pins/harness/tests/pins.rs:130-146; dashu-int-0.5.0 src/buffer.rs:48, 57-60, 121-129, 205-206, 230-231; src/bits.rs:428-434, 551-561; src/arch/mod.rs:64-66; src/arch/generic_32_bit/word.rs:2)
- Class / severity / confidence: correctness / medium / medium
- Provenance: assessed (traced `dashu-int` 0.5.0 by reading: `Word = u32` under `target_pointer_width = "32"`; `Buffer::MAX_CAPACITY = usize::MAX / WORD_BITS_USIZE`; `default_capacity` clamps with `.min(Self::MAX_CAPACITY)` behind a `debug_assert`; `UBig::ZERO.set_bit(k)` for `k >= 64` takes `with_bit_dword_spilled`, which allocates `idx + 1` words and pushes `idx + 1` words; `push` asserts `self.len < self.capacity` in release); executed: no
- Seen by: correctness [25]; refutation: confirmed, severity raised low to medium; history: already-known in part (wasm32-pins/harness/tests/pins.rs:138-141 records the backend capacity as unreachable "through the doors", pricing `Version::decode`'s working set only; the borsh `ReaderCursor` path is not priced there, and 05d87e1b's stated intent was to reject at the same width with `NotCanonical` on both paths)
- Owner-gated: no (the fix is crate-private; the pins.rs record wants amending alongside)
- Witness: demonstrated (32-bit pass, run in the `wasm32-pins` executor; the main pass was inconclusive, agent 3: "not constructed: requires a wasm32 or other 32-bit target", and re-read the dashu-int 0.5.0 chain: `Repr::set_bit` on a `Small` value with `n >= DWORD_BITS` calls `with_bit_dword_spilled` (bits.rs:428-439), `Buffer::allocate(idx + 1)` clamps through `default_capacity(...).min(MAX_CAPACITY)` (buffer.rs:48, 57-60, 121-123), and `push` asserts `self.len < self.capacity` (buffer.rs:205-206)). Borsh-deserializing a `Version` in the wasm32 guest from a stream of the leaf flag, `2^32 - 32` zero bits, and the mantissa's leading `1` traps with no mantissa byte read; the lower adjacency at `2^32 - 33` zeros allocates the identical clamped buffer, sets the bit, reads on to the stream's end, and returns a typed error, so the trap is the guard's and not memory exhaustion. Each run took about 29.7 s (the `2^32` per-bit reads at borsh_impls.rs:95-103).

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

Witness output (32-bit pass; `cargo nextest run --cargo-profile release --no-fail-fast -E 'test(/_checked_guest$/)'` in `crates/before/wasm32-pins` against the guest built for `wasm32-unknown-unknown`; results.md `## 32-bit pass`, codec-bits-23):

    ```text
            PASS [  29.695s] (10/11) wasm32-pins-harness::zz_witness version_borsh_wide_gamma_below_loose_guard_checked_guest
            PASS [  29.744s] (11/11) wasm32-pins-harness::zz_witness version_borsh_wide_gamma_at_loose_guard_checked_guest
         Summary [  29.745s] 11 tests run: 11 passed, 51 skipped
    ```

    `version_borsh_wide_gamma_at_loose_guard_checked_guest` asserts `call1("pin_version_borsh_wide_gamma", (1u64 << 32) - 32) == Outcome::Trapped(Trap::UnreachableCodeReached)`; `version_borsh_wide_gamma_below_loose_guard_checked_guest` asserts `Outcome::Value(-1)` (the export's typed-error code) at `(1u64 << 32) - 33`. The guest's reader synthesizes the stream one byte per `read` call and ends it after the byte holding the mantissa's lead, so nothing is buffered on the test's side. Assessed by reading, not observed: the trap's exact site, `Buffer::push`'s `assert!(self.len < self.capacity)` after `push_zeros(idx - 2)` fills the clamped buffer; the guest has no stderr.

Synthesis note (32-bit pass): the pins workspace's `Cargo.lock` resolves dashu-int 0.5.1 while the root lock pins 0.5.0 (the entry quotes 0.5.0); the cited lines are the same in both, and the run exercised 0.5.1. The lower adjacency resolves the entry's working-set caveat (the `ReaderCursor`'s ~512 MiB byte buffer plus the 512 MiB backend buffer fit the 4 GiB guest). Not exercised: the word-parallel twin at dsi.rs:255-267, which needs a `2k + 1`-live-bit slice; the committed pin `version_decode_memory_terminal_traps` records that the byte door's working set exhausts memory first at that size.

Resolution: Replace both `usize::try_from(k)` guards with one shared predicate at the backend's cap: with `W = usize::BITS as u64` (dashu's `Word` is `usize`-wide on 32- and 64-bit targets), a `k`-bit mantissa needs `k / W + 1` words and the backend holds at most `usize::MAX / W`, so reject when `k / W >= usize::MAX as u64 / W` (which subsumes the `try_from` failure); state the derivation inline and the dependence on dashu's `Word` width beside the existing "bumping that dependency is a breaking change" rule; amend pins.rs:138-141 to name the borsh path. Pin it under `cfg(target_pointer_width = "32")` (or in the wasm32-pins guest) with a test-only `BitCursor` that yields `2^32 - 32` zero bits then a `1` without materializing them, asserting `Err(Decode::NotCanonical)`. Acceptance: on wasm32 the pin returns `Err(Decode::NotCanonical)` where today it panics with dashu's `assertion failed: self.len < self.capacity`; both arms route through the one predicate (`grep` shows a single `usize::MAX` division site); the 64-bit differential suites are unchanged.
Construction: target wasm32 (`usize` 32 bits, `Word = u32`, `MAX_CAPACITY = 2^27 - 1`). Borsh-deserialize a `Version` from a reader yielding the skyline stream `1` then `k = 2^32 - 32` zero bits then a `1` (about 512 MiB of `0x80, 0x00 ...`): `ReaderCursor::read_int` (borsh_impls.rs:125) falls to `decode_int_from`; `k < 64` is false; `usize::try_from(2^32 - 32)` is `Ok`; `m.set_bit(k)`: `idx = 2^27 - 1`, `Buffer::allocate(2^27)` clamps capacity to `2^27 - 1`, `push(lo)`, `push(hi)`, `push_zeros(2^27 - 3)` passes its `assert!(n <= capacity - len)` exactly, then `push(1 << (k % 32))` fails `assert!(self.len < self.capacity)`. Any `k` in `[2^32 - 32, 2^32 - 1]` works; `k = 2^32 - 33` and below allocate and succeed. The word-parallel path reaches dsi.rs:267 identically given a stream of `2k + 1` live bits.

Synthesis note: the rank partition's wasm32 pins hold the numerator's two-arm dispatch to the production backend on the side the claim rests on (rank-23) and record the `NotCanonical` genre for unrepresentable widths (rank-10, claim class); this guard is the codec-side twin those pins do not reach. The codec-bits open question about whether representability on the target belongs in `Decode::NotCanonical`'s doc rides along.

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

### clippy-pedantic-2: `PackedBuilder::patch_bit` truncates the offset to `u32` before the assert that implements its documented panic
- Where: crates/before/src/codec/build.rs:155-159 (related: crates/before/src/codec/build.rs:141-143, crates/before/src/codec/build.rs:321-322, crates/before/src/codec/build.rs:287, crates/before/src/party/ops/build.rs:130-131)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read build.rs:60-175 and 284-326; grepped every caller of `patch_bit`, `read_bits`, `bit_at`, `reserve`; `git log -L` on `patch_bit`); executed: no
- Verification: reframed: the sweep names `read_bits` as sharing the pattern, but its `debug_assert!` at line 287 checks the untruncated `pos + u64::from(n) <= self.len()` at entry, so only `bit_at` (321-322) shares the truncated-check shape; history: no-rationale-found (the cast-before-assert order dates to 525e7324, when `at` was `usize` and the cast was exact, and survived the `u64` widening at 83e61b4d unchanged)
- Owner-gated: no (`pub(crate)`)

The `# Panics` section promises a panic for any `at` at or past the output length, but the staged-region arm computes `(at - committed) as u32` first and asserts on the truncated value, so an `at` with `at - committed >= 2^32` whose low 32 bits fall below `staged_len` passes the assert and patches a staged bit instead of panicking. Today's only callers (party/ops/build.rs:130-131) pass positions returned by `reserve`, so the trigger is programmer error only; the finding is that the check proves less than the sentence above it says (panic doctrine: a documented panic is a contract clause and its check is its proof).

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

Resolution: assert on the untruncated position before the cast: `assert!(at < self.len(), "patch position {at} is past the output");` and then `let offset = (at - committed) as u32;` is exact because `at - committed < staged_len`. Apply the same reorder to `bit_at`'s `debug_assert!` (321-322). Acceptance: the construction below panics with the documented message.
Construction: in a codec unit test, `let mut b = PackedBuilder::with_capacity(0); b.push_bit(false); b.patch_bit(1u64 << 32, true);`. Today `committed == 0`, `offset == 0 < staged_len == 1`, no panic, and `finish()` carries a set bit the caller never wrote; after the fix the call panics as documented.

Synthesis note: inventory-7 records the same defect at nit; the two agree on mechanism and fix, and differ on whether a programmer-error-only trigger rates low or nit.

### inventory-7: Cast to u32 precedes the range assert it is meant to be guarded by
- Where: crates/before/src/codec/build.rs:154-159 (related: crates/before/src/codec/build.rs:141-143, 302, 321-322)
- Class / severity / confidence: correctness / nit / high
- Provenance: assessed (read); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`patch_bit` truncates `at - committed` to `u32` before asserting it is below `staged_len` (at most 64), so a programmer-error position of the form `committed + k·2^32 + small` passes the assert and patches a staged bit, while the function's `# Panics` promises a panic for any `at` at or past the output.

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

Resolution: compare at u64 width first (`assert!(at - committed < u64::from(self.staged_len), ...)`) and cast after; `bit_at` (321-322) and `read_bits` (302) can take the same shape. Acceptance: the assert's operand is never narrowed before the comparison.

Synthesis note: same defect as clippy-pedantic-2, whose verification pass established that `read_bits` already checks the untruncated range at entry (line 287), so only `bit_at` needs the companion reorder.

### Base, text, and tree

### codec-base-text-tree-20: The clock text door trims Unicode whitespace; every other door and the cursor's own contract are ASCII-only
- Where: crates/before/src/codec/text.rs:185-191 (related: crates/before/src/codec/text.rs:5-9, crates/before/src/codec/text.rs:23-27, crates/before/src/party.rs:784-789, crates/before/src/version.rs:1454-1459, crates/before/src/codec/tests.rs:1766, crates/before/src/codec/tests.rs:1803)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read `parse_clock_str`, `Cur::skip_ws`, and the `Party`/`Version` `FromStr` impls, which go straight to `Cur`-based parsers with no trim; std documents `str::trim` as `char::is_whitespace`, Unicode White_Space, which includes U+000B and U+00A0, and `u8::is_ascii_whitespace` as excluding U+000B); executed: no
- Seen by: correctness, claims; refutation: confirmed (with the owner gate added); history: no rationale found (`s.trim()` is Phase 7's original line, introduced beside `Cur`'s ASCII predicate with no recorded whitespace-class decision)
- Owner-gated: yes: narrows what a public `FromStr` accepts

`parse_clock_str` strips the stamp's outer whitespace with `str::trim`, while `Cur::skip_ws` (and hence the `Party` and `Version` doors, and the id and event halves inside the same stamp) skip `u8::is_ascii_whitespace` only. The same byte sequence is accepted at the stamp's edges and rejected everywhere else, contradicting the module's own contract that the grammar is pure ASCII and that the skyline kernel "must make byte-identical grammar decisions to this module's". The exhaustive alphabet (`b"()01, "`) and the whitespace injector (`b" \t\n\r"`) are ASCII-only, so no committed instrument sees the divergence. Correct at all scales, for all inputs: three doors spell one notation.

Evidence:

       185	pub(crate) fn parse_clock_str(s: &str) -> Result<(BitsBuf, &str), Parse> {
       186	    let t = s.trim();
       187	    let bytes = t.as_bytes();
       188	    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
       189	        return Err(Parse::Syntax);
       190	    }
       191	    let inner = &t[1..t.len() - 1];

         5	/// A whitespace-skipping byte cursor over the input string. The grammar is pure
         6	/// ASCII (`(`, `)`, `,`, digits, `0`/`1`), so byte-level scanning is exact.

        24	        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {

Resolution: Trim with the cursor's own predicate, `s.trim_matches(|c: char| c.is_ascii_whitespace())`, or let the cursor-based split of finding codec-base-text-tree-19 do the skipping through `Cur` so the stamp door has no whitespace rule of its own. Add a point pin beside `id_text_parser_error_precedence_pins` asserting the three `FromStr` doors agree on a leading U+000B and U+00A0 (all `Err(Parse::Syntax)`). Acceptance: `"\u{0B}(1, 0)".parse::<Clock>()` and `"\u{A0}(1, 0)".parse::<Clock>()` return `Err(Parse::Syntax)`, matching `"\u{0B}1".parse::<Party>()`; the pin and `clock_text_deep_nesting_never_panics` stay green.

Construction: `"\u{0B}(1, 0)".parse::<Clock>()` is `Ok` today (vertical tab is Unicode White_Space, stripped by `trim`), while `"(1,\u{0B}0)".parse::<Clock>()` is `Err(Parse::Syntax)` (the event text `\u{0B}0` reaches `Cur`, which does not skip U+000B) and `"\u{0B}1".parse::<Party>()` is `Err(Parse::Syntax)`. The same with U+00A0 in place of U+000B.

Synthesis note: the sibling question of which `Parse` precedence the notation owns (the id door's per-node `NotCanonical` versus the version door's whole-pass syntax-first rule) is codec-base-text-tree-18 (api-surprise class); the two doors' whitespace class and their error precedence are one "three doors spell one notation" decision.

## Cross-cutting: fold, shape, recurse, serde and borsh

`fold.rs`, `shape.rs`, and `recurse.rs` carry no correctness finding; the borsh door is the crate-root partition's model wire door. The one entry is the serde door's data-model type.

### crate-root-34: serde impls serialize as `bytes` but deserialize by requesting a `seq`
- Where: crates/before/src/serde_impls.rs:20-31 (related: crates/before/src/serde_impls.rs:33-57, crates/before/src/serde_impls.rs:64-75, crates/before/src/serde_impls.rs:80-94, crates/before/src/serde_impls.rs:98-114, crates/before/src/serde_impls/tests.rs:23-105)
- Class / severity / confidence: correctness / medium / medium
- Provenance: verified (serde_core-1.0.229 src/de/impls.rs: `VecVisitor` at 1143 implements only `visit_seq` (1157) and `Vec<T>::deserialize` calls `deserializer.deserialize_seq(visitor)` (1175); ciborium-0.2.2 src/de/mod.rs:425-435 answers `deserialize_seq` on `Header::Bytes` by synthesizing a `BytesAccess` and calling `visit_seq`; serde_bytes-0.11.19 src/de.rs:132-174 accepts `visit_borrowed_bytes`, `visit_bytes`, `visit_byte_buf`, and `visit_seq`; `grep -c serde_bytes Cargo.lock` = 0. Which third-party formats fail is assessed, not run); executed: no
- Seen by: structure, correctness; refutation: confirmed, with one correction to the correctness lens (serde_bytes is not in the workspace's dependency graph, so it would be a new optional dependency); history: no-rationale-found (the `<Vec<u8>>::deserialize` shape is from a519aa88e with only the wire-form intent recorded; ada0db0e extended it and tested three formats' leniency without reconsidering the visitor)
- Owner-gated: no (a new optional dependency is a design choice; see the open question)
- Witness: demonstrated (agent 1, under the `serde` feature: a twenty-line Deserializer holding one typed byte string, which answers every request through `deserialize_any` with `visit_bytes`, the data-model behavior `serde_test` exhibits for `Token::Bytes`; `Party`, `Version`, and `Clock` deserialization all fail)

Every `Serialize` here calls `serialize_bytes`; every `Deserialize` goes through `<Vec<u8>>::deserialize`, whose visitor implements only `visit_seq`. In serde's data model `bytes` and `seq` are distinct types, and a Deserializer that answers `deserialize_seq` on a typed byte string by calling `visit_bytes` (which the model permits, and which serde's own `serde_test` does) rejects what `before` itself serialized with "invalid type: byte array, expected a sequence". The suite cannot see this: serde_json has no typed bytes, postcard frames `serialize_bytes` and `deserialize_seq` identically, and ciborium explicitly bridges a CBOR byte string into a seq. The module doc's contract ("the serialized form is exactly the wire form") should hold for every conforming format, not the three the tests drive. Prefer a dependency over hand-rolling: `serde_bytes::ByteBuf`'s visitor is strictly more accepting and makes both directions `bytes`.

Evidence:

    20  impl Serialize for Party {
    21      fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    22          s.serialize_bytes(&self.encode())
    23      }
    24  }
    25  
    26  impl<'de> Deserialize<'de> for Party {
    27      fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    28          let bytes = <Vec<u8>>::deserialize(d)?;
    29          Party::decode(&bytes[..]).map_err(D::Error::custom)
    30      }
    31  }

Witness output (agent 1, `cargo nextest run -p before -p suanpan --all-features --no-capture -E 'test(/^w_/) | ...'`):

    ```text
    MEASURED crate-root-34: Party: invalid type: byte array, expected a sequence; Version: invalid type: byte array, expected a sequence; Clock: invalid type: byte array, expected a sequence
            PASS [   0.004s] (16/25) before::zz_witness_1 w_crate_root_34_serde_deserialize_rejects_typed_bytes
    ```

The witness's own caveat: "The serialize leg was verified by reading, not by a recording Serializer."

Resolution: Deserialize through `serde_bytes::ByteBuf::deserialize(d)?` (add `serde_bytes` as an optional dependency enabled by the `serde` feature) or a local ~20-line `Visitor` implementing `visit_bytes`, `visit_byte_buf`, and `visit_seq`, driven by `deserialize_bytes`. While touching all twelve impls, fold the six identical pairs into one macro taking a per-type doc attribute (`version.rs`'s `causal_cmp_impls!` is precedent) so the door has one body; the `Rank`/`Ranked`/`Span` impl docs carrying contract survive as doc arguments. Pairs naturally with crate-root-35 (the owned Vec the visitor yields can become the storage). Acceptance: a committed test drives a strict-typed deserializer, `serde_test::assert_tokens(&value, &[Token::Bytes(&value.encode())])` for each of the six types (dev-dependency `serde_test`), or a minimal local `Deserializer` whose `deserialize_seq` forwards a bytes payload to `visit_bytes`; the existing json/postcard/ciborium legs and the canonical-bytes pin stay green.
Construction: Add `serde_test` as a dev-dependency and write `serde_test::assert_tokens(&Party::seed(), &[serde_test::Token::Bytes(Party::seed().as_bytes())])`: the serialize leg passes (`serialize_bytes` emits `Token::Bytes`); the deserialize leg fails, because `serde_test`'s `deserialize_seq` forwards a non-seq token to `deserialize_any`, which calls `visit_bytes`, which `VecVisitor` does not implement.

## suanpan

### suanpan-24: Digit-position arithmetic wraps on 32-bit targets before any allocation can fail: a silent wrong value in release
- Where: crates/suanpan/src/accumulator.rs:1477-1487 (related: 1470; 561-573 (`offset + digit_shift`); 1314-1330 (`position += 1`); 1371 (`resize(pos + 1, 0)`); 1423 (`to + 1`); 317-323; Cargo.toml:174-192; crates/suanpan/src/limbs.rs:14-19; crates/suanpan/src/lib.rs:240-243)
- Class / severity / confidence: correctness / high / high
- Provenance: verified (read and trace; grep: the root Cargo.toml declares `[profile.dev]`, two `dev.package` overrides, and `[profile.bench]` only, so release builds carry Rust's default `overflow-checks = false`; crates/before/wasm32-pins contains no suanpan reference, so no first-party 32-bit run of suanpan exists); executed: no
- Seen by: correctness (26), claims (39); refutation: confirmed both and added the boundary sibling at 1371; history: no rationale found (the unchecked sums date from 092b149f; the `# Panics` text (7ab518ce) and the assertion audit (9f68c475) considered only `shift / 32`)
- Owner-gated: no
- Witness: demonstrated (32-bit pass, run in the `wasm32-pins` executor in two guest builds; the main pass was inconclusive, agent 4: "32-bit release semantics cannot be exercised on this 64-bit host", and re-read the cited lines and the profile table, quoted below). In the unchecked guest (release with rustc's default `overflow-checks = false`, the profile a consumer ships) construction (1) returns `Value(5)` with `sign == Greater`, `limbs.len() == 1`, and `digit_count() == 1`, and construction (2) returns `Value(1)`: the predicted wrong values, with no trap. In the checked guest (the pins workspace's `[profile.release] overflow-checks = true`) both trap before any allocation, so the position arithmetic at accumulator.rs:1479 and :566 is what wraps. Construction (3), the `usize::MAX` boundary, traps in both builds.

Only `shift / 32` is checked. The landing positions `2 * limb_index + digit_shift`, `2 * limb_index + 1 + digit_shift`, `offset + digit_shift` (566), and `position += 1` (1329) are plain `usize` additions after the guard, and zero contributions are skipped before any `resize` (1477, 1483, 563, 1317), so when a shift's low neighbours carry nothing the wrap happens before any allocation could fail loudly. On a 32-bit target in release the operand lands at digit 0 or 1 and the value reads back wrong with no panic. Principle 1 (never incorrect behavior; input likelihood carries no weight) and the `# Panics` clause, which promises a panic or an allocation failure. The crate names 32-bit targets as supported (limbs.rs:14-19, lib.rs:240-243). Both lens authors rated this medium (32-bit and release only); under the rubric (a bug that contradicts a stated contract) it is high, and the fix is one helper.

Evidence:

      1470	        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");
      ...
      1477	            if low != 0 {
      1478	                self.add_at(
      1479	                    2 * limb_index + digit_shift,
      1480	                    if negative { -low } else { low },
      1481	                );
      1482	            }
      1483	            if high != 0 {
      1484	                self.add_at(
      1485	                    2 * limb_index + 1 + digit_shift,
      1486	                    if negative { -high } else { high },
      1487	                );

       319	    /// Panics if the shifted digit position `shift / 32` overflows
       320	    /// `usize` — possible only on targets narrower than 64 bits (from
       321	    /// `shift = 2^37` on a 32-bit one). On 64-bit targets every `u64`
       322	    /// shift fits, and an enormous one fails at allocation instead, like
       323	    /// any collection asked to grow to `shift / 32` entries.

Witness output (agent 4, read-only re-check of the cited lines and the profile table):

    ```text
      1470	        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");
      1471	        for (limb_index, limb) in limbs.enumerate() {
      1477	            if low != 0 {
      1478	                self.add_at(
      1479	                    2 * limb_index + digit_shift,
      1483	            if high != 0 {
      1484	                self.add_at(
      1485	                    2 * limb_index + 1 + digit_shift,
       565	                self.add_at(
       566	                    offset + digit_shift,
      1329	            position += 1;
      1370	            if pos >= self.digits.len() {
      1371	                self.digits.resize(pos + 1, 0);
    174:[profile.dev]
    183:[profile.dev.package.before]
    186:[profile.dev.package.suanpan]
    191:[profile.bench]
    ```

Witness output (32-bit pass; two guest builds of `wasm32-pins-guest` for `wasm32-unknown-unknown`, the second with `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false`, each run through the wasmtime harness with `cargo nextest run --cargo-profile release --no-fail-fast`; results.md `## 32-bit pass`, suanpan-24):

    ```text
    # run 2 (unchecked guest; run2-unchecked.log):
        Starting 3 tests across 4 binaries (59 tests skipped)
            PASS [   0.131s] (1/3) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_wrap_unchecked_guest
            PASS [   0.133s] (2/3) wasm32-pins-harness::zz_witness suanpan_shl_wrap_unchecked_guest
            PASS [   0.135s] (3/3) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_boundary_unchecked_guest
         Summary [   0.135s] 3 tests run: 3 passed, 59 skipped
    # run 1 (checked guest; run1-checked.log), the suanpan lines:
            PASS [   0.139s] ( 1/11) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_boundary_checked_guest
            PASS [   0.140s] ( 3/11) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_wrap_checked_guest
            PASS [   0.144s] ( 8/11) wasm32-pins-harness::zz_witness suanpan_shl_wrap_checked_guest
    # profile verification from the -v build logs (grep of the suanpan / wasm32_pins_guest rustc lines):
    # checked:   crate-name suanpan	-C overflow-checks=on	--target wasm32-unknown-unknown
    # unchecked: crate-name suanpan	-C opt-level=3	--target wasm32-unknown-unknown   (no overflow-checks flag anywhere in the log: `grep -c overflow-checks` = 0)
    ```

    `suanpan_add_limbs_shl_wrap_unchecked_guest` asserts `call0("pin_suanpan_add_limbs_shl_wrap") == Outcome::Value(5)`, where the export returns the limb only when `sign == Greater`, `limbs.len() == 1`, and `digit_count() == 1`; `suanpan_shl_wrap_unchecked_guest` asserts `Outcome::Value(1)`; the `_checked_guest` twins assert `Outcome::Trapped(Trap::UnreachableCodeReached)`; `suanpan_add_limbs_shl_boundary_*` asserts the trap in both builds. Assessed by reading, not observed: construction (3)'s panic message and site (overflow at `resize(pos + 1)`, line 1371, checked; `resize(0)` then an index out of bounds at 1374, unchecked), since the guest has no stderr, so the entry's "not the documented message" clause for (3) is not observable in this executor.

Synthesis note (32-bit pass): the tree's own 32-bit executor could never have observed this defect as a wrong value, because its guest profile turns wraps into traps by design; the review's second build with overflow checks off is what showed it. No committed pin reaches suanpan's shift entry points at all. The pass's guest exports and harness tests (`witness/wasm32/witness.rs`, `zz_witness.rs`) are the constructions in the pins workspace's idiom and assert the defect as it stands, so they serve as the red-first pin the Resolution asks for once their expected outcomes are flipped to the documented panic.

Resolution: compute every landing position through one checked helper wired to the documented panic (the natural home is suanpan-12's `split_shift`): `digit_shift.checked_add(offset).expect("digit positions fit a usize")`, with `2 * limb_index` via `checked_mul` and `position.checked_add(1)` in `deposit_value`; or compute positions in `u64` and `usize::try_from` once per `add_at`. Restate the `# Panics` text in terms of the landing position (`shift / 32` plus the operand digit's offset). Acceptance: a red-first pin on a 32-bit target (extending crates/before/wasm32-pins' guest, which names suanpan nowhere today, or a new guest) observes the documented panic message for the constructions below; the 64-bit differential and ledger suites unchanged. Construction (32-bit target, release profile): (1) `let mut a = Accumulator::new(); a.add_limbs_shl([0u64, 0, 5], 32 * (u64::from(u32::MAX) - 3));`: `digit_shift = 2^32 - 4` passes `try_from`; limbs 0 and 1 are skipped (`low == high == 0`); limb 2's `low = 5` lands at `2 * 2 + (2^32 - 4) = 2^32`, which wraps to 0, so `a.sign_limbs()` returns `(Greater, vec![5])` and `a.digit_count()` is 1. (2) `a.add_u64(1); a.shl(64); a.shl(32 * (u64::from(u32::MAX) - 1));`: the held digits `[0, 0, 1]` skip offsets 0 and 1 in `fold_accum` and offset 2 wraps, so the value reads as 1. (3) The boundary sibling: at `shift = 32 * (2^32 - 1)`, `resize(pos + 1, 0)` at 1371 computes `usize::MAX + 1`, wraps to `resize(0)`, and `self.digits[pos]` at 1374 panics with an index-out-of-bounds: still the panic class, but not the documented message. In debug builds every case panics with "attempt to add with overflow".

Synthesis note: suanpan-26 (verification-gap class) records that `WORDS_PER_LIMB = 2` has no first-party 32-bit coverage; the same wasm32 guest leg would give both this pin and that coverage a home.

## The instruments

The oracle and laws, the meter core, the registry, and the tier-2 sizer carry no correctness finding: their surviving defects are prose, verification gaps, and scaffolding, filed in those classes. The board, the surface roster, the test harness, the envelope suite, the other integration suites, the benches and examples, the fuzz and pin workspaces, the fuzz-fit harness, and the fuelscape pipeline each have entries below.

### The board

### board-ops-render-12: `mechanism()` tags every liveness-floor trip as `constant+floor`, and its justifying comment cites the excised red-triage buffer
- Where: crates/before/src/meter/board/render.rs:48-62 (related: render.rs:98-104; judge.rs:56-73, 280-309, 335, 373-388; tests.rs:362, 480)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (a Python snippet reproduced the three substring tests at render.rs:50-60 over the verbatim label strings at judge.rs:58-73 and 283-335: every `*_FLOOR_TRIP` yields `constant+floor` because each contains "counter"; `heap capacity-model floor (stale model)` yields `floor`; `segments count` yields `constant`. `git show -s 920bfabb2`: "board: excise the expected-reds triage buffer; any red of record fails outright"; grep for `red-buffer`, `BOARD_EXPECTED_REDS`, `ExpectedRed` over src, tests, examples, benches, AGENTS.md, and the justfile finds only render.rs:99; `mech[` has no consumer outside render.rs); executed: yes (the Python evaluation over the exact labels)
- Seen by: scaffolding [0], structure-prose [27], instrument-correctness [48]; refutation: confirmed (severity of the mis-tag lowered because the `<- reasons` list beside the tag is correct); history: deliberate-but-expired (the tag was the class-binding seal's substrate (a4cc1cf35), whose `ExpectedRed` carried only `exponent` and `constant` booleans, so a floor mis-tag never reached a consumer; the seal was dissolved at 0a5bdaebd, which re-aimed the comment at the triage buffer; 920bfabb2 excised the buffer without touching render.rs; the `"count"`/`"counter"` collision is present in a4cc1cf35's own board.rs)
- Owner-gated: no

The red row's `mech[...]` tag is derived by substring search over the judge's human-readable labels. `"count"` targets the segments constant label (`"segments count"`) and also matches "counter" in every floor-trip message, so a cell red on a floor alone renders `mech[constant+floor]`: a liveness vacuity presented as also a constant regression, contradicting the function's own doc. The comment that justifies the tag names a mechanism the tree no longer has (root AGENTS.md hard rule: nothing refers to code that no longer exists), and nothing consumes the tag since the seal's dissolution. A stringly-typed classifier over another module's message text is the fragile mechanism; the labels are fixed `&'static str` constants, so the kind can be carried as data.

Evidence:

        53	    if red.iter().any(|label| {
        54	        label.contains("constant") || label.contains("count") || label.contains("ceiling")
        55	    }) {
        56	        kinds.push("constant");
        57	    }
        58	    if red.iter().any(|label| label.contains("floor")) {
        59	        kinds.push("floor");
        60	    }

        98	    // A red cell's mechanism tag: which judgment kinds put it on the red list,
        99	    // mirroring the tags a red-buffer triage entry commits.

    judge.rs:
        58	pub(super) const HEAP_FLOOR_TRIP: &str =
        59	    "heap floor: counter reads below floor: the meter is not watching this work";

Resolution: Delete the "mirroring the tags a red-buffer triage entry commits" clause. Then decide the tag's fate on present-tense grounds: either dissolve `mechanism()` and the `mech[...]` column (the `<- {reasons}` list already names every red leg), or keep it as a reader aid and have judge.rs carry the kind as data (a `RedKind { Exponent, Constant, Floor }` beside each label in `CellResult.red`, with `Display` producing today's text) so render classifies by type. The tests that match labels by string (`vec!["limb exponent"]`, `SCAN_FLOOR_TRIP`) compare variants or `to_string()` afterwards. Acceptance: `git grep -n -i 'red-buffer' -- crates` is empty; either `mechanism` is gone or a unit test asserts `mechanism(&[SCAN_FLOOR_TRIP]) == "floor"` and `mechanism(&["segments count"]) == "constant"`, with no `contains(` in the classifier.

### board-families-floors-judge-27: `version_output_bytes` and the sibling ops.rs sites omit the marker bit and under-report the packed size by one byte on byte-aligned streams
- Where: crates/before/src/meter/board/operand.rs:292-297 (related: ops.rs:1313-1326, 1491-1492, 1504-1511, 1794-1795, 1807-1814; version.rs:1129-1131, 1174-1180; party.rs:576-578; clock.rs:852-858; codec/bits.rs:70-81)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (the size law at version.rs:1129-1131 and party.rs:576-578; the marker-then-zero-pad rule at bits.rs:74-77; `Clock::encoded_bits` at clock.rs:856-857 already rounds the party component to whole bytes, so a clock is at most one byte short; ops.rs:1316 floor-divides); executed: no
- Seen by: instrument-correctness; refutation: reframed (a clock's `div_ceil` form is at most one byte short, not two); history: deliberate-but-expired (correct until d800957e's marker bit on 2026-08-06 changed the size law without touching operand.rs or ops.rs)
- Owner-gated: no

The function documents itself as the packed byte size of a measured version but computes `encoded_bits().div_ceil(8)`; the crate's own contract is `encode().len() == (encoded_bits() + 1).div_ceil(8)`, so the result is one byte short whenever `encoded_bits() % 8 == 0`. The same stale law sits at ops.rs:1491, 1510, 1794, 1813 (the party and clock parse rows' I/O denominators and heap floors), and ops.rs:1316 floor-divides (`/ 8`): on a child half with fewer than 8 live bits `child_bytes` is 0 and the fork row's heap floor silently becomes NA_HEAP_IN_PLACE (1319-1320), and on every other child the floor is up to seven bits lenient. Correct for all inputs: the function's doc is false on one stream in eight; the direction is conservative on the constants and lenient on the heap floors, so it masks nothing today, but it is a plain harness bug with a one-token fix that disagrees with every input-side `encode().len()` on the same board.

Evidence:

       292	/// The packed byte size of a version produced by a measured body.
       293	pub(super) fn version_output_bytes(v: &Version) -> usize {
       294	    // The measured value's stored buffer is allocated on this host, so its
       295	    // byte count fits `usize`.
       296	    usize::try_from(v.encoded_bits().div_ceil(8)).expect("an allocated buffer's byte count")
       297	}

    version.rs:
      1129	    /// The exact length in bits of [`encode`](Self::encode) before its
      1130	    /// padding — the marker bit and zero-pad to the byte boundary, so
      1131	    /// `encode().len()` is `(encoded_bits() + 1).div_ceil(8)`.

    ops.rs:
      1316	                    probe.fork().encoded_bits() / 8

Resolution: `v.as_bytes().len()` (O(1), public) and delete the `try_from` dance; at the ops.rs sites use `as_bytes().len()` / `encode().len()` for party and clock (1316 included, which also removes the NA collapse on tiny children). Acceptance: a committed test sweeping `study_family_versions(DEFAULT_SCALE)` plus one version with `encoded_bits() % 8 == 0` asserts `version_output_bytes(&v) == v.encode().len()` and passes.
Construction: Tick a fresh `Version` with `Party::seed()` until `v.encoded_bits() % 8 == 0` (or pick one such stream from the family corpus), then `assert_eq!(version_output_bytes(&v), v.encode().len())`: the left side is one less today.

Synthesis note: board-frame-15 records the same `version_output_bytes` defect from the `IoSpec::output_bytes` contract side; this entry adds the four ops.rs sites and the fork row's NA collapse.

### board-frame-15: The version output reader undercounts a flush stream by its marker byte
- Where: crates/before/src/meter/board/cell.rs:173-174 (related: crates/before/src/meter/board/operand.rs:292-297, crates/before/src/version.rs:1151-1152, crates/before/src/version.rs:1174-1180, crates/before/src/codec/bits.rs:160-166, crates/before/src/codec/buf.rs:383-386)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (bits.rs:160-166 `len()` returns the live bit count recovered from the marker; version.rs:1151-1152 `encoded_bits` returns it; `as_bytes` at :1174-1180 returns the raw slice including the marker; buf.rs:383-386 appends the marker after the live bits, so 8k live bits store as k+1 bytes while `encoded_bits().div_ceil(8)` gives k); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-but-expired (exact under the unmarked coding of 6814d77b9; d800957e8 changed the size law to `encode().len() == (encoded_bits() + 1).div_ceil(8)` and re-derived the board's other adapters but not operand.rs)
- Owner-gated: no

`IoSpec::output_bytes` is documented as reading the actual output's byte size, and board.rs:158-159 insists the output side is read back from the result, never assumed; the version reader behind it derives the size from a bit count with a rounding that disagrees with the stored bytes one time in eight, so `n_io` is one byte short on flush outputs while the input side uses the wire `bytes.len()`. Negligible for any verdict, but the reader's contract and its arithmetic disagree, and `as_bytes().len()` is exact and O(1) (Principle 1: correct for all inputs).

Evidence:

       173	    /// Read the actual output's byte size from the boxed result.
       174	    pub(super) output_bytes: fn(&dyn Any) -> usize,

    [operand.rs:293-296]
       293	pub(super) fn version_output_bytes(v: &Version) -> usize {
       294	    // The measured value's stored buffer is allocated on this host, so its
       295	    // byte count fits `usize`.
       296	    usize::try_from(v.encoded_bits().div_ceil(8)).expect("an allocated buffer's byte count")

    [bits.rs:165]
       165	                self.bytes.len() as u64 * 8 - 1 - u64::from(last.trailing_zeros())

Resolution: `v.as_bytes().len()` in `version_output_bytes`. Acceptance: for a version whose `encode()` ends in `0x80`, `version_output_bytes(&v) == v.encode().len()`; a unit test beside the reader pins it.
Construction: Any version with 8k live bits (`encoded_bits() == 8k`): `div_ceil(8) == k` while `encode().len() == k + 1`; `Version::new().encoded_bits()` is 2, so tick a seeded version until `encoded_bits() % 8 == 0` and compare.

Synthesis note: same defect as board-families-floors-judge-27, which carries the wider site list.

### board-frame-21: `truncated_bytes` argues its two-byte cut from a decoder verdict the decoder no longer produces, and the one-byte cut is both correct and more deferred
- Where: crates/before/src/meter/board/defect.rs:30-51 (related: crates/before/src/codec/bits.rs:467-470, crates/before/src/codec/bits.rs:489-491, crates/before/src/error.rs:72-74, crates/before/src/version.rs:1116-1118, crates/before/src/meter/board/ops.rs:1849-1854, crates/before/src/codec/bits.rs:449)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (bits.rs:489-491 `match remainder { 0 => Err(Decode::Truncated), ...}` and :467-470 document the empty remainder as `Truncated`; error.rs:72-74 says the same; version.rs:1116-1118 validates the whole buffer then calls `require_marker_padding`, so a one-byte cut on a flush stream parses the complete tree and fails at the padding judge with `Truncated`; ops.rs:1851-1853 asserts `Decode::Truncated`; `git show 61d00223a --stat` touches bits.rs and error.rs only); executed: no
- Seen by: instrument-correctness (scaffolding's marker-literal item folds in); refutation: confirmed; history: deliberate-but-expired (d800957e8 wrote the rationale when the empty remainder was `TrailingBits`; 61d00223a reclassified it as `Truncated` at every decode door and did not touch defect.rs)
- Owner-gated: no

The doc justifies cutting two bytes on a flush stream because dropping only the marker byte "would leave a complete tree missing its padding — a `TrailingBits` defect, not a cut". The padding judge returns `Decode::Truncated` for an empty remainder, so the one-byte cut is already the defect the row asserts and is more deferred (the whole tree parses before the judge fires), while the two-byte cut removes eight live bits so the parse fails early. The rejection rows exist to price the defect maximally deferred (board.rs:194-196); a rationale that contradicts the decoder's documented taxonomy is a ghost. The one-byte cut also removes the bare `0b1000_0000` marker literal (spelled `[0x80]` at bits.rs:449, with no named constant).

Evidence:

        35	/// parsing to the cut. The final byte is pure padding exactly when it is
        36	/// the whole-byte marker `1000_0000` (the live bits end flush against
        37	/// the byte boundary); dropping only that byte would leave a *complete*
        38	/// tree missing its padding — a `TrailingBits` defect, not a cut — so
        39	/// the cut then takes the last live byte with it.
        40	pub(super) fn truncated_bytes(bytes: &[u8]) -> Vec<u8> {
        41	    let cut = if bytes.last() == Some(&0b1000_0000) {
        42	        2
        43	    } else {
        44	        1
        45	    };

    [bits.rs:489-491]
       489	    let remainder = total - pos;
       490	    match remainder {
       491	        0 => Err(Decode::Truncated),

    [bits.rs:467-469]
       467	/// - An empty remainder is [`Decode::Truncated`]: the input ends where the
       468	///   padding should begin — a flush stream cut before its whole marker byte —
       469	///   so required data is missing, exactly what a byte-starved reader reports

Resolution: Always cut one byte and re-state the doc: on a non-flush stream the cut removes the last live bits and the marker, and the tree walk runs out of input; on a flush stream it removes the marker byte alone, the whole tree parses, and the padding judge reports `Truncated` at the end, the most deferred placement byte granularity allows. Relax the guard to `bytes.len() > 1`. Acceptance: a unit test beside the builders: for a version whose `encode()` ends in `0x80`, `truncated_bytes(&bytes).len() == bytes.len() - 1` and `Version::decode(&truncated_bytes(&bytes)[..])` is `Err(Decode::Truncated)`; the truncation rows' `matches!(err, Decode::Truncated)` assertions stay green; scan readings on flush-stream families rise, never fall.
Construction: Take any version whose live bits are a multiple of 8 (its `encode()` ends in `0x80`); drop only the final byte; `Version::decode` walks the complete tree, reaches `pos == total`, and `require_marker_padding` returns `Decode::Truncated` (remainder 0), not `TrailingBits`.

### board-frame-23: `party_noncanonical_text` never places its defect at the text's end on any board operand
- Where: crates/before/src/meter/board/defect.rs:168-176 (related: crates/before/src/meter/board/ops.rs:2078-2097, crates/before/src/meter/board/family.rs:1109-1126, crates/before/src/meter/board/family.rs:1036-1039, crates/before/src/codec/display.rs:41-52, crates/before/src/codec/text.rs:141, 165-169, crates/before/src/meter.rs:561-574, crates/before/src/meter/board/floors.rs:426-439, crates/before/src/meter/board/defect.rs:10-12)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (display.rs:41-42 renders a terminal as `1` and :52 renders an absent child as `0`; text.rs:141 treats `0` as absence and :165-169 rejects `(0, 0)` and `(1, 1)` identically at the node's `)`; family.rs:1114-1115 pushes `left, !left` so the mount adapter's `a` is `(shape, 0)`; `party_pair` (:1036-1038) returns `a` first and the row feeds it (ops.rs:2082-2083); `id_spine` (meter.rs:564-572) is a left spine rendering `((((1, 0), 0), 0), 0)`; text-rejection floors are all NA (floors.rs:431-439)); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed, severity medium to low (the row's floors are NA, the id parser does no metered work, and a linear prefix leaves the time exponent unchanged, so the shortfall is a constant fraction of one row's readings; the false doc claim is the concrete defect); history: no-rationale-found (false at birth: 3c43a8154 wrote "at the text's end" against an operand 3eadcb107 had already made `(shape, 0)`)
- Owner-gated: no

The placer re-spells the last `1` token, but the id notation spells absent children as `0`, so on every board operand at least `, 0)` follows the last `1`, and on the left-leaning spine (the id side of `id-pair` and `staircase`) the pair lands about `d+2` bytes into a roughly `5d` byte text and the parser rejects with most of the text unparsed. This contradicts the doc's "at the text's end" and the module's criterion that every defect is maximally deferred (defect.rs:10-12; Principle 6: an early-exit measurement is the cheapest artifact that passes). The packed-side sibling is correct because absent children occupy no bits; the text side needs its own construction.

Evidence:

       168	/// `text` with its last `1` token re-spelled `(1, 1)`: the collapsible pair,
       169	/// judged non-normal at the node's close, at the text's end
       170	/// ([`Parse::NotCanonical`](crate::error::Parse)).
       171	pub(super) fn party_noncanonical_text(text: &str) -> String {
       172	    let at = text
       173	        .rfind('1')
       174	        .expect("a party's text spells at least one owned leaf");
       175	    format!("{}(1, 1){}", &text[..at], &text[at + 1..])
       176	}

    [display.rs:52; text.rs:166-169]
        52	            f.write_str("0")?; // an absent child renders `0`
       166	                        (IdKind::Empty, IdKind::Empty) => return Err(Parse::NotCanonical), // (0, 0)
       167	                        (IdKind::Terminal, IdKind::Terminal) => {
       168	                            return Err(Parse::NotCanonical); // (1, 1)
       169	                        }

    [family.rs:1114-1115; ops.rs:2082-2083]
      1114	        bits.push(left);
      1115	        bits.push(!left);
      2082	                let (a, _, _) = f.party_pair()?;
      2083	                let fed = party_noncanonical_text(&a.to_string());

Resolution: Target the last leaf token whichever it is, `rfind(|c: char| c == '0' || c == '1')`, and re-spell `t` as `(t, t)`; the parser rejects `(0, 0)` and `(1, 1)` identically at the `)`, so the row's `Parse::NotCanonical` assertion holds and only closing parens follow the defect. Re-word the doc ("its last leaf token `t` re-spelled `(t, t)`, the non-normal pair judged at the node's close, the text's last token"). Acceptance: a unit test beside the builder: for the mounted `id-pair` operand, the produced text's `(t, t)` closes at the last non-paren byte and `parse::<Party>()` returns `Parse::NotCanonical`; the row's heap readings on the left-mounted families do not fall.
Construction: `Party` text `(((((1, 0), 0), 0), 0), 0)` (the `id_spine(4, false)` shape mounted left, 26 bytes) becomes `((((((1, 1), 0), 0), 0), 0), 0)`; `parse_id_tree` returns `NotCanonical` after consuming the 12 bytes `((((((1, 1)`, leaving 19 unparsed. A test asserting `d.len() - d.find("(1, 1)").unwrap() <= 8` fails on the current placer.

### board-ops-render-31: A zero denominator makes the exponent fit `NaN`, and the exponent leg then reads green on unbounded growth
- Where: crates/before/src/meter/board/judge.rs:41-44 (related: judge.rs:120-124, 363; cell.rs:204-308; measure.rs:78-123; defect.rs:40-51)
- Class / severity / confidence: correctness / nit / high
- Provenance: verified (arithmetic from judge.rs:37-54: `(0f64).ln()` is `-inf`, so `mean_x` is `-inf`, the first centered term is `NaN`, `sxx` is `NaN`, `sxx <= f64::EPSILON` is false, and the slope is `NaN`; at 363 `e > ceiling` is false for `NaN`; `spans` passes trivially; cell.rs:204-308 and measure.rs carry no positivity assert on `input_bytes`; `truncated_bytes` asserts `bytes.len() > cut`, every encoding is at least one marker byte, and text rows use `s.len()`, so no committed row can produce it); executed: no
- Seen by: instrument-correctness [54]; refutation: confirmed (a programmer-error class; the constant leg still fires on the construction); history: no-rationale-found
- Owner-gated: no

The finalizer noted the anchors live in judge.rs and cell.rs, outside the ops-render partition's five files, and asked for deduplication against the judge partition; that partition filed no entry on this mechanism, so this is the only record.

`trend` takes `ln` of the denominator; a point with `n = 0` yields a `NaN` slope, and the exponent check `e > ceiling` is false for `NaN`, so the leg is judged and never red. No committed family produces a zero denominator, but `Cell::new`, `Cell::io`, and `Cell::text` accept it and nothing asserts otherwise. Correct for all inputs: the judge should fail loudly on a denominator it cannot fit rather than fold `NaN` into a green; the invariant belongs at construction.

Evidence:

    judge.rs:
        41	    let xy: Vec<(f64, f64)> = points
        42	        .iter()
        43	        .map(|&(n, m)| ((n as f64).ln(), (m.max(1) as f64).ln()))
        44	        .collect();

       363	        if s.exp_judged && s.exp.is_some_and(|e| e > *ceilings.get(c)) {

Resolution: `assert!(input_bytes > 0, "a board cell charges against at least one byte")` in the three `Cell` constructors (or in `measure` before the fit); optionally `debug_assert!(!slope.is_nan())` at the end of `trend`. Acceptance: a unit test constructing a `Sample` pair with `exp_denom_bytes: 0` through `evaluate` panics at the guard rather than returning a green cell.

Construction: `trend(&[(0, 1), (100, 1_000_000)])` returns `NaN`; two `Sample`s with `denom_bytes`/`exp_denom_bytes` 0 and 100 and limb readings 1 and 1_000_000 through `evaluate` yield `red` without "limb exponent" (the constant leg still fires).

### The surface roster

### surface-roster-28: The shared extractor silently drops `pub const fn` and any `pub fn` at an unexpected indent, contradicting its stated never-under-report contract
- Where: crates/surface-scan/src/lib.rs:111-129 (related: crates/surface-scan/src/lib.rs:19-25, crates/surface-scan/src/lib.rs:69-72, crates/surface-scan/src/lib.rs:104-110, crates/before/src/testing/surface_coverage.rs:267-270, crates/suanpan/src/claims/tests.rs:172-195, crates/surface-scan/src/tests.rs:56-75)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read lib.rs:73-133: exactly two positive shapes, `    pub fn ` and `pub fn `, and no negative catch-all; the nested-impl panic at 104-110 fires only inside a `pub mod`; `grep -nE '^\s*pub (const|async|unsafe|extern) fn|^(  |      |        +)pub fn '` over the seventeen `SURFACE_SOURCES` files returns nothing, so the hole is latent for before; suanpan's only totality check is `claims_are_total_over_the_public_surface` over this extractor and it has no rustdoc-JSON leg); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed; history: no-rationale-found (the qualifier and nested-indent cases were never considered when the contract was written)
- Owner-gated: no (a catch-all panic; replacing the line scan with `syn` parsing would be a design decision)
- Witness: demonstrated (agent 1, against the shipped `surface_scan::extract_public_fns` over two fixture files)

The module doc's one invariant, "A `pub fn` at an unexpected position panics rather than silently vanishing — the extractor must never under-report the surface it exists to pin", is false for constructible shapes: `    pub const fn`, `    pub async fn`, `    pub unsafe fn`, and any `pub fn` at an indent other than 0 or 4 (an inherent impl inside a private `mod` block, still public API on a public type) match neither positive arm and produce no row and no panic. Making a constructor `const` is a routine change. before is rescued by surfacecheck; suanpan's claims roster has no second jaw. Correct for all inputs: an instrument that pins the public surface must not have silent-drop shapes, and its doc must not claim a discipline it lacks.

Evidence:

        23	//! methods and module-block functions at one indent. A `pub fn` at an
        24	//! unexpected position panics rather than silently vanishing — the
        25	//! extractor must never under-report the surface it exists to pin.
    ...
       111	            if let Some(rest) = line.strip_prefix("    pub fn ") {
    ...
       123	            if let Some(rest) = line.strip_prefix("pub fn ") {

Witness output (agent 1, Run A2):

    ```text
    MEASURED surface-28: const_fn fixture rows={"Thing::one"}; nested fixture rows={}
            PASS [   0.016s] ( 6/10) before::zz_witness_1 w_surface_28_extractor_drops_pub_const_fn_silently
    ```

The fixture held `pub const fn zero`, `pub async fn later`, `pub unsafe fn raw`, and `pub fn one` in one inherent impl, and a second file held a `pub fn hidden` at an 8-space indent inside `mod inner { impl Thing { ... } }`; the extractor returned exactly `{Thing::one}` for the first and nothing for the second, with no panic. The suanpan consequence (a `pub const fn` on `Accumulator` leaving `claims_are_total_over_the_public_surface` green) was not run; it follows from the extractor result.

Resolution: after the two positive arms, a negative catch-all: any line whose trimmed form starts with `pub` and contains ` fn ` that was not classified panics naming file and line ("beyond the line discipline"). Accept `pub const fn` positively at both indents (strip an optional `const ` after `pub `), since it is public surface with the same naming. Fixtures in surface-scan/src/tests.rs: `pub const fn` in an inherent impl is named; an 8-indent `pub fn` inside `mod x { impl T { .. } }` panics. Acceptance: the new fixtures are red on the current extractor and green after; before's and suanpan's totality tests stay green on the tree.
Construction: fixture `impl Thing {\n    pub const fn zero() -> u8 {\n        0\n    }\n}` with `spec(None)`: `extract_public_fns` returns an empty set and does not panic. In suanpan, adding `pub const fn probe() -> u8 { 0 }` inside `impl Accumulator` leaves `claims_are_total_over_the_public_surface` green with no `Accumulator::probe` claim row.

Synthesis note: surface-roster-9 (scaffolding class) proposes retiring before's line scan in favor of surfacecheck; that retirement leaves suanpan's roster on this extractor alone, so the catch-all here is owed regardless of that decision. The api-audit sweep's open question 2 (should `Version::new`/`Party::seed`/`Clock::seed` be `const fn`) is exactly the routine change that would trip this hole in before were surfacecheck not the second jaw.

### surface-roster-29: `parse_impl_self_type` treats the `>` of a `->` return arrow as a closing angle bracket; a header in the scanned tree already trips it
- Where: crates/surface-scan/src/lib.rs:139-153 (related: crates/surface-scan/src/lib.rs:135-137, crates/before/src/version/rank.rs:693-703)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (`grep -nE '^impl.*->'` over the `SURFACE_SOURCES` files hits exactly rank.rs:693 `impl<F: FnMut() -> Result<u8, Decode>> BitSource<F> {`; hand trace of 139-153: `<` sets depth 1, the `>` of `->` sets depth 0 and breaks, and the remainder ` Result<u8, Decode>> BitSource<F> {` yields `Result`, so `current_type` is `Result`; the block holds only the private `fn bit`, so nothing is misnamed today); executed: no
- Seen by: instrument-correctness; refutation: confirmed by trace; history: no-rationale-found (the parser predates the header that trips it)
- Owner-gated: no

The function documents "skip a balanced generics list, then read the first identifier" and mis-skips a header the tree already contains. A `pub fn` added to that block would be extracted as `Result::..`, failing the roster loudly but under a phantom type name. Correct for all inputs: the failure is loud, which keeps the severity low, but the mismatch message would misdirect the maintainer.

Evidence:

       139	    if chars.peek() == Some(&'<') {
       140	        let mut depth = 0usize;
       141	        for c in chars.by_ref() {
       142	            match c {
       143	                '<' => depth += 1,
       144	                '>' => {
       145	                    depth -= 1;
       146	                    if depth == 0 {
       147	                        break;
       148	                    }
       149	                }

Resolution: track the previous character and do not count a `>` preceded by `-` as a close (or skip `->` as a unit); add a fixture `impl<F: FnMut() -> u8> Thing<F> {\n    pub fn poke(&self) {}\n}` extracting as `Thing::poke`. Acceptance: the fixture is red on the current parser (it yields `u8::poke`) and green after.
Construction: the fixture above with `spec(None)`: `extract_public_fns` returns `{"u8::poke"}`.

### surface-roster-21: The two totality jaws contradict each other on a `#[doc(hidden)]` inherent `pub fn` in a scanned file
- Where: crates/before/surfacecheck/src/extract.rs:31-32 (related: crates/surface-scan/src/lib.rs:111-121, crates/before/surfacecheck/src/check.rs:296-300, crates/before/tests/doc_hidden.rs:16-21)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (the line scan at lib.rs:111 strips `    pub fn ` with no attribute check, so a hidden inherent method in a `SURFACE_SOURCES` file is extracted and `roster_is_total_over_the_public_fn_surface` demands a row; extract.rs:31-32 states hidden items are absent from the JSON, and check.rs:296-300 reports every rostered op absent from the extraction as orphaned, with no exception path for orphans; tests/doc_hidden.rs:21 pins today's only hidden items as the sealed `PartyLiteral` trait and its method, which the line scan never sees); executed: no
- Seen by: none (raised by the refutation pass); refutation: new; history: not examined
- Owner-gated: no

A `#[doc(hidden)] pub fn` added to an inherent impl in a listed file would put the tree in a state no edit can make green: the attribute-blind line scan demands a `METHOD_SURFACE` row, and surfacecheck then reports that row orphaned because hidden items never reach rustdoc JSON. Today the interaction is moot, but nothing documents the consequence (hidden inherent methods are forbidden by construction, which may be desirable), and a maintainer meeting it sees two contradictory red diffs. If the line scan is retired (surface-roster-9) the contradiction dissolves; if it is kept, the consequence belongs in prose at tests/doc_hidden.rs or surface_coverage.rs.

Evidence:

        31	//! `#[doc(hidden)]` items never appear in the JSON at all, so the ground
        32	//! truth here is the documented public surface.

    surface-scan/src/lib.rs:
       111	            if let Some(rest) = line.strip_prefix("    pub fn ") {

    check.rs:
       296	        orphaned: rostered
       297	            .iter()
       298	            .filter(|op| !extracted.contains(**op))

Resolution: retire the line scan (surface-roster-9), or state at tests/doc_hidden.rs (beside the roster) that a hidden inherent `pub fn` in a `SURFACE_SOURCES` file cannot satisfy both totality checks and is therefore not a shape the crate admits. Acceptance: the prose exists, or only one extractor remains.
Construction: add `#[doc(hidden)] pub fn probe(&self) {}` inside `impl Party` in src/party.rs. `roster_is_total_over_the_public_fn_surface` fails naming `Party::probe` as unrostered; add the row, and `just surface-totality` fails naming `Party::probe` as orphaned.

### surface-roster-1: The CI instruments job installs a floating nightly while the recipe it runs invokes the dated pin, and its comment describes the coupling the pin exists to remove
- Where: .github/workflows/ci.yml:142-145 (related: .github/workflows/ci.yml:128-133, justfile:25-40, justfile:935-945, .github/workflows/ci.yml:64-67, .github/workflows/ci.yml:206-210)
- Class / severity / confidence: correctness / low / medium
- Provenance: verified (read ci.yml:118-190, justfile:20-42 and 935-945; `grep 2026-06-30 .github/workflows/ci.yml` returns nothing); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-but-expired (the comment was accurate under a floating `nightly_toolchain`; e7a4b7b0 dated the pin and touched the justfile, AGENTS.md, rust-toolchain.toml, and the bands, not ci.yml)
- Owner-gated: no

The `instruments` job installs `toolchain: nightly` and runs `just surface-totality`, whose `surface-json` prerequisite invokes `cargo +{{ nightly_toolchain }}` = `+nightly-2026-06-30`. The job comment says the job "tracks nightly, so a format bump upstream can turn the leg red on an untouched tree", which is the failure the justfile's dated pin (justfile:25-38) exists to prevent and which the recipe as written cannot exhibit. Whether the job is green depends on the runner's rustup auto-installing the dated toolchain on first use; either way the two committed descriptions of what CI runs disagree (Principle 4: the goal stands beside the mechanism, and the two must agree; measurements bind to their run).

Evidence:

       128	  # Toolchain coupling: surface-totality parses nightly rustdoc JSON
       129	  # through a `rustdoc-types` pin matched to the installed nightly's
       130	  # format_version. This job tracks nightly, so a format bump upstream can
       131	  # turn the leg red on an untouched tree — the checker refuses loudly,
    ...
       142	      - name: Install nightly toolchain (surface-totality rustdoc JSON)
       143	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
       144	        with:
       145	          toolchain: nightly

    justfile:
        40	nightly_toolchain := "nightly-2026-06-30"
    ...
       937	    cargo +{{ nightly_toolchain }} rustdoc -p before --lib --all-features --target-dir target/surface-json -- -Z unstable-options --output-format json

Resolution: install the dated toolchain in CI from the justfile's single pin (a step that writes `just --evaluate nightly_toolchain` to `GITHUB_OUTPUT`, consumed by the `toolchain:` input), and rewrite ci.yml:128-133 to say the job runs the pinned nightly. The `ci` and `coverage` jobs' `toolchain: nightly` steps (64-67, 206-210) feed `doctest`, `fuzz-build`, and the branch-coverage leg, all spelled `+{{ nightly_toolchain }}` too; same fix, outside this partition. Acceptance: ci.yml derives `nightly-2026-06-30` wherever a `+{{ nightly_toolchain }}` recipe runs; the comment matches the mechanism; bumping one side alone fails the job with surfacecheck's format_version message, not a rustup error.
Construction: on a machine with rustup auto-install disabled (`RUSTUP_AUTO_INSTALL=0`, rustup >= 1.28.1) and only the floating `nightly` installed, `just surface-totality` fails at `surface-json` with a toolchain-not-installed error before surfacecheck runs.

Synthesis note: the deps sweep settled the operational edge (deps-6, documentation class): run 33560347645 on main shows the coverage job green including the branch leg, so rustup does provision the dated nightly on demand today; the disagreement between the two committed descriptions stands. gate-legs-2 below is the same genre for cargo-mutants.

### The test harness

### testing-oracles-4: The oracle-to-impl doors state normal form as a fact but check nothing; a non-normal or empty oracle tree lowers to a wrong `Party`
- Where: crates/before/src/testing/bridge.rs:70-76 (related: crates/before/src/testing/bridge.rs:10, crates/before/src/testing/bridge.rs:23-24, crates/before/src/testing/bridge.rs:36-43, crates/before/src/testing/bridge.rs:83-89, crates/before/src/party.rs:712-722, crates/before/src/version.rs:1192-1201, crates/before/src/testing/grow_brute_force.rs:43-47, crates/before/src/testing/exhaustive/tests.rs:12-14)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (party.rs:716 and version.rs:1196 state caller guarantees and `from_bits` only freezes; `emit_id` on `Node(Leaf(false), Leaf(false))` pushes `false, false` at 39-40, the `Leaf(true)` terminal tag at 33-34, so the empty region lowers to the full one; `from_oracle_party(&Leaf(false))` emits no bits and freezes an anonymous `Party`; `all_inflations` returns raw trees by contract at grow_brute_force.rs:45-46; `oracle::Party` is a `pub enum` with `Node` public); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found ("only ever emits normalized" entered at 32a65543 as a statement about callers; the oracle envelope assigns input bounding to harnesses and says nothing about a normal-form check at the door)
- Owner-gated: no

`Party::from_bits` and `Version::from_bits` delegate the normal-form and nonempty obligation to their callers; the bridge passes it on silently to its own callers while its module doc asserts the outcome as a fact ("Both forms are normalized"). A non-normal oracle tree is constructible from committed test API: `all_inflations` returns raw trees, and `oracle::Party::Node` is public. Lowering `Node(Leaf(false), Leaf(false))` yields a `Party` byte-equal to the seed's full id; lowering `Leaf(false)` yields the anonymous `Party` that exhaustive/tests.rs:12-14 says never exists standalone; lowering a raw version yields a skyline with equal adjacent plateaus whose byte-`Eq` disagrees with every canonical value, a red that would be blamed on production. All present callers comply (the oracle's constructors normalize; the exhaustive suite normalizes candidates at tests.rs:326), so nothing is masked today; the doctrine's bar for a guard is met because the failure is constructible from test API, no committed test exercises it, and the check is O(n) on bounded test trees.

Evidence:

        10	//! Both forms are normalized, so structural `==` ⇔ semantic equality.
        23	/// Whether an oracle id subtree is the empty `0` region. In normal form that is
        24	/// exactly the `Leaf(false)`; the bridge only ever emits normalized oracle trees.
        72	pub(crate) fn from_oracle_party(t: &oracle::Party) -> Party {
        73	    let mut bits = BitsBuf::new();
        74	    emit_id(&mut bits, t);
        75	    Party::from_bits(bits)
        76	}

    party.rs:
       716	    /// Callers guarantee normal *tree* form (a nonempty, normalized id);

Resolution: Add `debug_assert!(t.is_normal(), …)` at `from_oracle_version` and `from_oracle_party`, plus `debug_assert!(!t.is_empty(), …)` on the party door (both oracle types expose `is_normal`; `oracle::Party::is_empty` exists at oracle/party.rs:30), and reword lines 10 and 23-24 to state the precondition as the caller's. Acceptance: a test lowering `oracle::Party::Node(Arc::new(Leaf(false)), Arc::new(Leaf(false)))` panics at the door instead of yielding a `Party` equal to the seed's.
Construction: In any unit test with `pub(crate)` access: `from_oracle_party(&oracle::Party::Node(Arc::new(oracle::Party::Leaf(false)), Arc::new(oracle::Party::Leaf(false))))` returns bits `00`, equal to `from_oracle_party(&oracle::Party::Leaf(true))`. For versions: take the left-descent candidate of `all_inflations(&oracle::Party::Leaf(true), &oracle::Version::node(0u64, oracle::Version::leaf(1u64), oracle::Version::leaf(2u64)))` (a non-normal `Node(0, Leaf 2, Leaf 2)`), lower it with `from_oracle_version`, and observe it is not equal to `from_oracle_version(&that.normalized_for_test())`.

Synthesis note: testing-oracles-3 (idiom class) covers the bridge's uneven `descend!` guarding; both are edits to the same four doors.

### The envelopes

### gate-legs-1: The coverage leg is red at HEAD on a byte-identical before tree; the heap meter is not deterministic under instrumentation
- Where: crates/before/tests/meter.rs:6860-6860 (related: crates/before/tests/meter.rs:13-21, crates/before/tests/meter.rs:6907-6910, crates/before/tests/meter.rs:6929-6933, justfile:1031-1035, .github/workflows/ci.yml:226-227)
- Class / severity / confidence: correctness / high / high
- Provenance: verified (`gh run view 33567211421` and `--log-failed`; `gh run view 33560347645` with its coverage job log; `git diff --stat 3327a92b 9e5784fb -- crates/before crates/suanpan` empty; Cargo.lock package diff walked against before's and suanpan's dependency closures); executed: yes: `cargo nextest run -p before -p suanpan --all-features -E 'test(masked_cmp_hole_envelope)' --success-output immediate`, exit 0, `peak_heap=384`
- Verification: confirmed, and sharpened: the last green coverage run (3327a92b) and the red one (9e5784fb) have byte-identical crates/before and crates/suanpan trees, and the Cargo.lock movement between them touches only rumors' hash crates, none of which any before or suanpan dependency reaches; history: no-rationale-found
- Owner-gated: no

GitHub run 33567211421 fails the `coverage` job at `just coverage-kernel` with `masked_cmp_hole_envelope` reading peak heap 1156 B against the 480 B pin, while the previous main run's coverage job passed on an identical before tree and test-binary closure, and the same test reads 384 B uninstrumented (the pin is exactly 384 x 1.25). The peak-heap column the suite documents as one of three deterministic meters is therefore not a function of the tree under the coverage leg, and the gate never runs that leg, so no local check tells a committer that main is red.

Evidence:

        13	//! Three deterministic meters, asserted together per scenario:
      6860	    pub const MASKED_CMP_HOLE: QueryEnvelope = query_envelope(480, 0, 0, 7_535, 18, 0, 10); // the block skip consumes the spine's unowned continuation whole: [...]
      6907	    HEAP.reset_peak_usage();
      6908	    let baseline = HEAP.current_usage();
      6909	    let r = f();
      6910	    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
      1034	    {{ justfile_directory() }}/tools/memwatch cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/workspace.lcov

    CI job 100053091628 (coverage, run 33567211421):
        MEASURED masked_cmp_hole: input_bytes=755 peak_heap=1156 segments=0 limb_ops=0 touches=14 scan_bits=6028
        masked_cmp_hole: peak heap 1156 B exceeds the pinned envelope 480 B (input 755 B): note: the meters are process-global and meaningful only one scenario per process: run under cargo nextest, not a shared-process cargo test

    Local, uninstrumented (this review):
        MEASURED masked_cmp_hole: input_bytes=755 peak_heap=384 segments=0 limb_ops=0 touches=14 scan_bits=6028

Resolution: Settle the source before touching the pin. Run `just coverage-kernel` twice at 9e5784fb and compare the `MEASURED masked_cmp_hole` lines with the uninstrumented 384. If the instrumented reading moves between identical runs, find what allocates inside the scenario body only under `-C instrument-coverage` (or only sometimes) and either isolate the meter from it or exclude the heap column under coverage builds with the reason stated at the exclusion. If the instrumented reading is stable but differs from 384, the envelope suite and the coverage leg judge different profiles, and the coverage recipes should filter the meter suite out (they exist to measure kernel coverage, not envelopes). Never widen 480 to accommodate. Acceptance: two consecutive green `coverage` jobs on main with no change to any envelope constant, and a comment at the chosen site naming the mechanism.
Construction: The comparison above already demonstrates the claim: identical trees, one green and one red coverage run, and a local reading of 384 B. The residual question is which of the two remedies applies, which only the coverage leg itself can answer.

Synthesis note: the deps sweep observed the same failed run independently (its open question 4 names run 33567211421, job `coverage`, the same MEASURED line, and the captured log at `scratchpad/before/final-sweep-deps/ci-failed.log`) and routed it to the metering partition; this entry is the disposition. Neither envelope partition filed it (both reviewed the suite's text, not CI runs). The masked-hole row's re-pin history is envelopes-b-16 (simplification class).

### envelopes-b-28: Unchecked counter subtraction in `span_shares_the_crossing_folds`
- Where: crates/before/tests/meter.rs:10320-10326
- Class / severity / confidence: correctness / nit / high
- Provenance: verified (read; the suite runs under the dev/test profile with overflow checks, which .cargo/mutants.toml:33-35 relies on); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no rationale found (1bb610d19a introduced the form)
- Owner-gated: no

`fused - cmp` on `u64` readings rests on the premise that the span ladder runs the classifying comparison first, so `fused >= cmp`. If a ladder change dropped that comparison, the test would die with "attempt to subtract with overflow" instead of this assertion's message. The relation `fused - cmp < met + joined` is `fused < met + joined + cmp` with no subtraction. A harness assertion should fail with its own message under every input it can meet.

Evidence:

     10320	        assert!(
     10321	            fused - cmp < met + joined,
     10322	            "the fused hull's own folds must undercut the composed \
     10323	             emissions' two accumulators ({} vs {} composed touches)",
     10324	            fused - cmp,
     10325	            met + joined
     10326	        );

Resolution: rewrite as `fused < met + joined + cmp` and print all four readings (or assert `fused >= cmp` first with its own message). Acceptance: no bare `-` between counter readings in the range.
Construction: any hypothetical `span` fast path that skips the classifying `partial_cmp` yields `fused` below `cmp`'s early-exiting prefix; the current line panics on overflow before reaching the assertion.

### The gate and CI workflow

### gate-legs-2: cargo-mutants is version-pinned by the count roster but installed unpinned in CI, and the install comment misclassifies it
- Where: .github/workflows/ci.yml:76-86 (related: tools/mutantcheck-expected.json:2, tools/mutantcheck:145-151, justfile:308-311, justfile:326)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read ci.yml:76-86, mutantcheck:137-151, mutantcheck-expected.json:2; `cargo mutants --version` = 27.1.0 locally; `git log -S'"tool":'` shows the pin landing in bec1ceaf on 2026-08-13 and the CI install landing in e4d92ae4 on 2026-08-17 without it); executed: no
- Verification: confirmed; history: no-rationale-found (e4d92ae4's message says only that the runner must carry the binary; the ci.yml comment's rationale for not pinning is false for this tool)
- Owner-gated: no
- Witness: inconclusive (agent 4: "installing a different cargo-mutants release is outside the permitted actions"; the mechanical checks confirmed every premise, quoted below)

tools/mutantcheck refuses any `cargo mutants --version` other than the string pinned in tools/mutantcheck-expected.json, so `just ci`'s `mutants-list` leg turns red on an untouched tree the day taiki-e/install-action's manifest resolves the next cargo-mutants release. The step comment justifies pinning only cargo-rdme on the grounds that every other tool merely reports findings; cargo-mutants' version string and operator set are committed expectations compared exactly.

Evidence:

        76	      # cargo-rdme carries a version because it is the only tool here whose
        77	      # output is a committed artifact compared byte for byte: readme-check
        80	      # in the repo, turning the sweep red on a tree nobody touched. The others
        81	      # report findings rather than generate bytes, so they ride the pinned
        82	      # action's own tool manifest, moving when Dependabot bumps the pin.
        86	          tool: just,cargo-nextest,cargo-rdme@2.1.0,cargo-fuzz,cargo-mutants,wasm-pack

    tools/mutantcheck-expected.json
         2	  "tool": "cargo-mutants 27.1.0",

    tools/mutantcheck
       146	    if tool_version != pinned_tool:
       147	        problems.append(
       148	            f"tool version {tool_version!r} does not match the pinned "
       149	            f"{pinned_tool!r}: operator sets move between releases, so "
       150	            "re-pin the counts in the same diff as the tool bump"

Witness output (agent 4):

    ```text
    cargo-mutants 27.1.0
    2:  "tool": "cargo-mutants 27.1.0",
        83	      - name: Install just, cargo-nextest, cargo-rdme, cargo-fuzz, cargo-mutants, and wasm-pack
        84	        uses: taiki-e/install-action@37f7c5781271959fb65b6b35224e28652ff2b63d # v2.87.0
        85	        with:
        86	          tool: just,cargo-nextest,cargo-rdme@2.1.0,cargo-fuzz,cargo-mutants,wasm-pack
       145	    pinned_tool = expected.get("tool")
       146	    if tool_version != pinned_tool:
       147	        problems.append(
       148	            f"tool version {tool_version!r} does not match the pinned "
       149	            f"{pinned_tool!r}: operator sets move between releases, so "
       150	            "re-pin the counts in the same diff as the tool bump"
    ```

Resolution: Pin `cargo-mutants@27.1.0` beside `cargo-rdme@2.1.0` on ci.yml:86 and reword lines 76-82: the pinned tools are those whose version is a committed expectation (cargo-rdme's emitted bytes; cargo-mutants' version string and operator inventory), and a bump touches the expectation file and the install line in one diff. Acceptance: the workflow names the same cargo-mutants version as tools/mutantcheck-expected.json, and tools/workflowlint (or a one-line grep in `mutants-list`) fails when the two disagree.
Construction: Install any other cargo-mutants release and run `just mutants-list`: the checker exits 1 with `tool version ... does not match the pinned 'cargo-mutants 27.1.0'` before comparing a single count.

### Workspace tools (tools/)

### tools-22: Two hand-rolled Rust-file walkers read build output and a foreign worktree into the gate
- Where: tools/doclint:370-373 (related: tools/testdoc:19, 66-78; justfile:181, 186; tests/seed_liveness.rs:35; .github/workflows/ci.yml:103-105)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified; executed: yes (reproducing doclint's traversal, `Path(r).rglob("*.rs")` over benches, crates, examples, src, tests, lists 547 files, 2 of them under crates/before/fuzz/target/{aarch64-apple-darwin/release,debug}/build/thiserror-*/out/private.rs; reproducing testdoc's walk over `.` lists 1088 files, 543 under .claude/, which git excludes via .git/info/exclude:7; CACHEDIR.TAG is present in target/, crates/before/fuzz/target, crates/before/surfacecheck/target, and crates/before/wasm32-pins/target)
- Seen by: scaffolding [8], adequacy [24], structure-prose [34]; refutation: confirmed; history: no-rationale-found (the walkers were written independently in 432ac34d and 358c6b1a with no shared policy; tests/seed_liveness.rs:35 carries a third copy with `.claude` added, so the divergence was noticed once and fixed locally; the testdoc half is recorded as verification-infra-9 in the rumors review, whose proposed fix, explicit roots for testdoc, would leave doclint's descent in place)
- Owner-gated: no
- Cross-references: gate-legs-10 (simplification) records the doclint half with the two `.cargo/config.toml` redirects that route around it; this entry adds testdoc's hidden-directory walk, the third copy in `tests/seed_liveness.rs`, and the shared discovery routine.

doclint takes hand-named roots and walks them with no exclusions, so `crates` includes thiserror's generated code under the fuzz workspace's target, present only on machines that have built fuzz; testdoc excludes `target` by basename but not hidden directories, so `just testdoc .` reads another agent's uncommitted worktree. A gate verdict must be a function of the committed tree (Principle 6). Two sibling lints with two discovery routines, and a third copy in a test, is the duplicated-capability pattern that produces the divergence; `git ls-files` already knows the tracked set. ci.yml:103-105 restores the fuzz workspace's target from cache, so the runner is not necessarily exempt (an inference from the workflow, not verified).

Evidence:

   370	def rust_files(root):
   371	    if root.is_file():
   372	        return [root] if root.suffix == ".rs" else []
   373	    return sorted(root.rglob("*.rs"))

    (tools/testdoc)
    19	IGNORED_DIRECTORIES = {".git", "node_modules", "target"}

    (tests/seed_liveness.rs)
    35	const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", ".claude"];

Resolution: one discovery routine shared by doclint and testdoc (a small helper module in tools/, offered to seed_liveness.rs as well): `git ls-files -z --cached --others --exclude-standard -- '*.rs'` restricted to the given roots (tracked plus untracked-not-ignored, so pre-add work is still linted while target/, .claude/worktrees, and other excluded trees are invisible), or, if git is not wanted in the lint tier, prune directories carrying CACHEDIR.TAG and hidden directories. Keep each tool's missing-root guard; pin the routine in both self-tests with a fixture tree containing `target/x.rs` beside a CACHEDIR.TAG and `.hidden/y.rs`. Acceptance: doclint and testdoc report the same file set on the same tree; with the two generated files and the `.claude/worktrees` files present, neither tool visits them.
Construction: write a `///` paragraph of 300 characters into a scratch `.rs` under crates/before/fuzz/target/ and run `./tools/doclint crates`: it fails; on a fresh checkout the same commit passes. Write an undocumented `#[test]` into `.claude/worktrees/w/src/t.rs` and run `./tools/testdoc .`: it reports it.

### tools-17: covcheck returns early on an out-of-scope entry, hiding every other finding, against its own stated policy two lines later
- Where: tools/covcheck:212-218
- Class / severity / confidence: correctness / low / high
- Provenance: verified; executed: yes (an expectation carrying `"other/x.rs": []` beside a report with two new uncovered lines returned only `["other/x.rs: expected entry outside the pinned scope 'crates/k/src/'"]`)
- Seen by: none of the four lenses; raised as new by the refutation pass; history: not examined
- Owner-gated: no

The scope check `return`s with a single problem, so a mis-scoped entry hides every hole and every anchor finding, while the comment immediately below says resolution problems must report side by side with the holes "rather than hiding one class behind the other".

Evidence:

   212	    for relpath in entries:
   213	        if not relpath.startswith(scope):
   214	            return [f"{relpath}: expected entry outside the pinned scope {scope!r}"]
   215	    # Resolution problems do not stop the judgment: the comparison still
   216	    # runs over whatever resolved, so a drifted pin reports the anchor
   217	    # findings AND the holes side by side rather than hiding one class
   218	    # behind the other.

Resolution: accumulate the scope problems into `problems`, drop the offending keys from `entries`, and continue to `resolve` and `check`; add a self-test case pairing an out-of-scope entry with a new hole and expecting both messages. Acceptance: the case passes; the comment at 215-218 is true of the scope check too.

### tools-25: memwatch's per-process kill for test and bench binaries keys on a `target/` path component that cargo's build.build-dir removes
- Where: tools/memwatch:82-87 (related: memwatch:11-13; justfile:101-103)
- Class / severity / confidence: correctness / low / high
- Provenance: verified; executed: yes (the `is_build_proc` case statement, run in isolation, returns 1 for `/Volumes/forge/build/70/874132ca3784ca/debug/deps/before-1234 --exact` and 0 for the same path under `target/`; `~/.cargo/config.toml` sets `build-dir = "/Volumes/forge/build/{workspace-path-hash}"`; the repository's `.cargo/` holds only mutants.toml; the only `target/*/deps` directories in the tree belong to the detached fuzz workspace, and the root `target/debug` has no `deps/`)
- Seen by: instrument-correctness [52]; refutation: confirmed (resolved the build directory for this workspace via read-only `cargo metadata`); history: deliberate-but-expired (the glob was written under cargo's default layout in 90df4227; the build-dir setting lives outside version control and cannot be dated)
- Owner-gated: no

Under `build.build-dir`, set on the development machine these limits are sized against, test and bench executables live at `<build-dir>/<profile>/deps/<bin>` with no `target` component, so the branch never fires for them and only the swap backstop remains; justfile:101-103 promises "a runaway test fails the build with the offender named". rustc and clippy-driver still match by name, so the monomorphization case the tool was built for is unaffected. Principle 1: "test binaries under target/" is a path-shape premise the environment falsifies.

Evidence:

    82	is_build_proc() {
    83	    case "$1" in
    84	        *rustc*|*clippy-driver*|*build-script-build*|*/target/*/deps/*) return 0 ;;
    85	        *) return 1 ;;
    86	    esac
    87	}
    11	#   1. Per-process: any build-related process (rustc, clippy-driver, build
    12	#      scripts, test binaries under target/) whose resident size exceeds

Resolution: match on `*/deps/*` regardless of the directory above it (test and bench binaries are only ever emitted into a `deps/` directory), or resolve `build_directory` once via `cargo metadata --no-deps` and match on that prefix; state the premise at the case arm and at line 12. Acceptance: on a machine with build.build-dir set, a test binary that allocates past PROC_LIMIT_GB is killed and logged with its pid and command.

### Other suites

### tests-other-26: Two seed comments describe a "concurrent" sibling version that `Clock::sync` has already made equal
- Where: crates/before/tests/support/fuzz_seed_set.rs:290-294 (related: crates/before/tests/support/fuzz_seed_set.rs:273-275, crates/before/tests/support/fuzz_seed_set.rs:335-336, crates/before/tests/support/fuzz_seed_set.rs:346, crates/before/src/clock.rs:319-320, crates/before/src/clock.rs:333-334)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read clock.rs:333-334: `self.version |= &other.version; other.version = self.version.clone();`, so after line 275 `sibling.version() == clock.version()`; the doc example at 319-320 asserts the same); executed: no
- Seen by: adequacy; refutation: confirmed; history: no rationale found (the sync and the "concurrent" comment were born together in bb6ea8b7; 23e46a7c repeated the claim for `laws_family`)
- Owner-gated: no
- Witness: demonstrated (agent 2, replicating fuzz_seed_set.rs:269-275 exactly and asserting the relation the comment claims)

After `clock.sync(&mut sibling)` both clocks hold one version, so the `clock_then_msg` payload is the clock's own version, not a message "concurrent to the clock's own history" (the fuzz target's flavour-1 path compares Equal and its `recv` is a no-op), and `laws_family`'s first two versions are one value, not "the synced clock's nested version, the sibling's concurrent version" (the family seeds `{v, v, empty}`). The suite exists so a seed cannot "quietly stop representing the values it was written for"; these two never represented the relation their comments claim.

Evidence:

       273	    clock
       274	        .sync(&mut sibling)
       275	        .expect("forked clocks are disjoint");
       ...
       290	    // Flavour 1: compare against, then receive, a canonical message (the
       291	    // sibling's version, concurrent to the clock's own history).
       292	    let mut msg = vec![1u8, len];
       293	    msg.extend_from_slice(&clock_bytes);
       294	    msg.extend_from_slice(&sibling.version().encode());

    (fuzz_seed_set.rs:335-336)
       335	    // A live family: the synced clock's nested version, the sibling's
       336	    // concurrent version, the empty version, and the two disjoint sibling

    (clock.rs:333-334)
       333	        self.version |= &other.version;
       334	        other.version = self.version.clone();

Witness output (agent 2, execution 1):

    ```text
    MEASURED tests_other_26: clock=((1, 0), 1) sibling_version=1 equal=true
    thread 'tests_other_26_synced_sibling_version_is_concurrent' panicked at crates/before/tests/zz_witness_1.rs:178:5
    ```

After `sync` both clocks hold the single version `1` (the clock prints `((1, 0), 1)`, the sibling's version prints `1`, and the two compare equal), so `assert!(clock.version().concurrent(sibling.version()))` fails.

Resolution: Capture `let concurrent = sibling.version().clone();` before the sync (or build the message from a sibling that has not synced) and use it for the flavour-1 payload and the laws family's second version; assert the relation at the derivation (`assert!(clock.version().concurrent(&concurrent))`) and in the ops contract test of tests-other-18; regenerate with `cargo run -p before --example fuzz_seeds` and commit the changed seed files. Acceptance: the relation assertion fails against the current derivation and passes after the reorder; `committed_seeds_match_the_live_derivation` is red until regeneration and green after; the two comments read true of the bytes.
Construction: After fuzz_seed_set.rs:275 add `assert!(clock.version().concurrent(sibling.version()));`. It panics: `sync` joins both histories into both clocks, as the `Clock::sync` doc example asserts with `assert_eq!(a.version(), b.version())`.

### tests-other-27: `weave_pair()` equals `scatter_pair()` by value, so the Weave family contributes nothing to the matrix pool
- Where: crates/before/tests/verdict_matrix.rs:124-137 (related: crates/before/tests/verdict_matrix.rs:45-46, crates/before/tests/verdict_matrix.rs:116-122, crates/before/tests/verdict_matrix.rs:438-453, crates/before/tests/verdict_matrix.rs:1295-1302, crates/before/src/meter/board/family.rs:378, crates/before/src/meter/board/family.rs:897-904, crates/before/src/version/skyline.rs:38, crates/before/src/lib.rs:262-263, crates/before/src/version.rs:1483-1484)
- Class / severity / confidence: correctness / medium / high
- Provenance: assessed (ITC reasoning against the crate's stated canonical normal form; not executed); executed: no
- Seen by: refutation pass (new, as a reframe of the scaffolding lens's duplication finding); history: the duplication itself is forced by a deliberate design (a44502fe privatized the shape constructors; registry.rs:1031-1033 records that the organic families' construction lives in the board's private family module), which is why the matrix carries its own smallest instances; nothing records the collision
- Owner-gated: no: the one-line fix is local; a smallest-instance door in the family module (meter-feature API) is the optional durable form
- Witness: demonstrated (agent 3, with verbatim copies of `weave_pair` and `scatter_pair`; the two render identically)

`b = a.fork()` splits the seed into halves; `c = a.fork()` and `d = b.fork()` split each half into quarters; `a.tick()` and `c.tick()` produce height-1 events over sibling quarters of one half, whose join `(0, (0, 1, 1), 0)` collapses under ITC normal form (the crate rejects collapsible `(n, m, m)` nodes and every value is canonical) to `(0, 1, 0)`, the value `scatter_pair`'s `a.tick()` yields over the same half; likewise on the right. `intern` (438-453) dedups on `Eq`/`Hash`, and `every_family_answers_the_matrix_coverage_question` checks only that the answer is nonempty, so the module doc's "no hand-enumerated subset can silently exclude one" is false for Weave, and `weave_pair`'s doc ("interleave across the shared upper skeleton") describes a structure the pair lacks: the operands occupy disjoint halves and share only the root. The board's own weave deals leaves round-robin (`i % WEAVE_GROUPS`), which for four leaves and two groups pairs `a` with `b` and `c` with `d`.

Evidence:

       124	/// The weave population's smallest organic instance: four fork-tree
       125	/// parties of one universe, one tick each, joined round-robin so both
       126	/// operands interleave across the shared upper skeleton.
       127	fn weave_pair() -> (Version, Version) {
       128	    let mut a = Clock::seed();
       129	    let mut b = a.fork();
       130	    let mut c = a.fork();
       131	    let mut d = b.fork();
       132	    a.tick();
       133	    b.tick();
       134	    c.tick();
       135	    d.tick();
       136	    (a.version().join(c.version()), b.version().join(d.version()))
       137	}

    (verdict_matrix.rs:1298-1302)
      1298	        assert!(
      1299	            !answer.versions.is_empty() || !answer.masks.is_empty(),
      1300	            "{family:?} contributes no matrix operands: an empty answer silently \
      1301	             excludes the family from every axis"
      1302	        );

Witness output (agent 3, `cargo nextest run -p before -p suanpan --all-features --success-output immediate -E 'test(/witness_/)' ...`):

    ```text
    MEASURED tests_other_27: weave=((0, 1, 0), (0, 0, 1)) scatter=((0, 1, 0), (0, 0, 1))

    thread 'witness_tests_other_27_weave_pair_differs_from_scatter_pair' (241406466) panicked at crates/before/tests/zz_witness_3.rs:357:5:
    assertion `left != right` failed: the weave pair collapses to the scatter pair under ITC normal form
      left: ((0, 1, 0), (0, 0, 1))
    ```

Resolution: Join the round-robin groups the doc describes: `(a.version().join(b.version()), c.version().join(d.version()))`, which yields `(0, (0,1,0), (0,1,0))` and `(0, (0,0,1), (0,0,1))`, both present at the root's two children. Add a pool-membership floor to `every_family_answers_the_matrix_coverage_question`: each family's answer interns at least one version not already in the pool, or the collision is declared at the arm. Optionally (owner-gated) expose a smallest-instance door from the board's family module under the `meter` feature so the three organic pairs derive from the same code as `scatter`/`weave`/`benign`. Acceptance: `assert_ne!(weave_pair(), scatter_pair())` holds; the pool grows by two versions against the parent commit; the new floor reads red on HEAD's `weave_pair` and green after the swap.
Construction: In `every_family_answers_the_matrix_coverage_question` add `assert_ne!(weave_pair(), scatter_pair(), "the weave pair collapses to the scatter pair");` and run `cargo nextest run -p before --all-features --test verdict_matrix`: red at HEAD by the normal-form argument above; green after the join swap.

Synthesis note: the finalizer recorded the provenance as assessed; the witness run settles it as demonstrated, and the recorded line stands as the finalizer wrote it.

### Benches and examples

### benches-examples-8: `scale_from_env` says "positive number" but accepts zero, negatives, NaN, infinity, and saturating magnitudes; the tripwire has no downstream guard
- Where: crates/before/benches/common/sidecar.rs:110-119 (related: crates/before/benches/tripwire.rs:56-59; crates/before/benches/common/sidecar.rs:160; crates/before/examples/amp_board.rs:193-198; crates/before/src/meter/board/export.rs:133-137; crates/before/src/meter/board/shard.rs:139-146)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (the parse arm only calls `raw.parse::<f64>()`; `bench_cells` asserts `scale > 0.0 && scale.is_finite()` at export.rs:134-137, which rescues the board target; tripwire.rs:57 computes `((TRIPWIRE_BASE as f64) * scale).round() as usize`, a saturating cast; `{scale:?}` at sidecar.rs:160 renders `NaN` and `inf`, neither of which is JSON; amp_board.rs:195-197 has the same shape and only the child's `assert_scale` (shard.rs:146) fires, so the parent reports "shard child failed"); executed: no
- Seen by: scaffolding [13], adequacy [23], structure-prose [35 part], instrument-correctness [56]; refutation: confirmed, adding the huge-scale saturation; history: no rationale (b4942461 applied its NaN/non-positive discipline to the judge's medians, not the parameter; the export.rs assert arrived with sharding)
- Owner-gated: no

Principle 1: a panic message must be true of the check beside it. For the tripwire, `nan` or `-1` yields `n = 0`, a zero-work probe, a `denominator_bytes: 0` cell, and a stamp line `"scale": NaN` that is not JSON, refused by the judge two stages later with a less direct message; a huge value saturates `n` to `usize::MAX` and the `n²` loop never finishes.

Evidence:

       114	        Ok(raw) => raw.parse().unwrap_or_else(|_| {
       115	            panic!("{SCALE_ENV} must be a positive number or `acceptance`, got {raw:?}")
       116	        }),

    tripwire.rs:
        57	    let n = ((TRIPWIRE_BASE as f64) * scale).round() as usize;

Resolution: after parsing, require `scale > 0.0 && scale.is_finite()` inside `scale_from_env` (one site, both bench targets) and at amp_board.rs:195-197, keeping the existing messages. Acceptance: `BOARD_BENCH_SCALE=nan cargo bench -p before --bench tripwire` panics at the parameter naming `BOARD_BENCH_SCALE`; likewise for `0`, `-1`, `inf`; `cargo run --example amp_board ... -- -1` panics at the parse site.
Construction: `BOARD_BENCH_SCALE=nan BOARD_BENCH_DENOMS=<scratch>/d.json cargo bench -p before --bench tripwire -- --sample-size 10 --measurement-time 1` completes, and d.json carries `"scale": NaN` and `"denominator_bytes": 0`.

### benches-examples-24: space_consumption accepts `--data-iters 0` and `--runs 0` under a "non-negative integer" message, then panics or writes NaN rows
- Where: crates/before/examples/space_consumption.rs:199-201 (related: crates/before/examples/space_consumption.rs:343-348, 352-367, 421-427)
- Class / severity / confidence: correctness / nit / high
- Provenance: verified by tracing the code: `parse_u64` accepts 0; `checkpoints(0)` computes `k_max` from `(0f64).log10()` = -inf, which saturates to `i64::MIN`, so the loop is empty and `points.push(max)` yields `[0]`; `simulate` iterates `1..=0` (empty) so `sizes` is empty; the aggregation indexes `run[ci]` at ci = 0 and panics; `--runs 0` makes `per_run` empty and `mean_std(&[])` divides by zero into NaN rows; executed: no
- Seen by: instrument-correctness [55]; refutation: confirmed; history: fe3d5f44-original
- Owner-gated: no

Principle 1 at harness weight: the parser owns the error message and should reject what the program cannot run, instead of an index panic downstream or a silently NaN CSV.

Evidence:

       200	                let bits: Vec<f64> = per_run.iter().map(|run| run[ci].0).collect();
       424	            "{flag} expects a non-negative integer, got {value:?}"

Resolution: require `runs >= 1`, `data_iters >= 1`, and `process_iters >= 1` in `parse_args`; change the message to "positive integer". Acceptance: `cargo run --example space_consumption -- --data-iters 0` exits 2 with the flag error; `--runs 0` likewise.
Construction: `cargo run --release --example space_consumption -- --runs 1 --data-iters 0 --process-iters 10 --entities 4` panics with an index out of bounds at line 200.

### Fuzz and pins

### fuzz-guests-pins-15: The cap asserts on absolute peak heap, not the "peak transient heap" its doc names
- Where: crates/before/fuzz/src/lib.rs:28-40 (related: crates/before/fuzz/Cargo.toml:23-28)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read peak_alloc 0.3.0 `src/lib.rs:107-110`: `reset_peak_usage` stores `CURRENT` into `PEAK`, and `peak_usage` returns the absolute `PEAK`); executed: no
- Seen by: scaffolding [1]; refutation: confirmed; history: no-rationale-found (the doc and 48965ff3 both say "transient"; nothing acknowledges the baseline)
- Owner-gated: no

After the body, `peak_usage()` is the larger of the heap live at reset and the in-body peak: libFuzzer's resident corpus and every process-lifetime allocation sit inside the asserted number, so the effective headroom shifts over a long run, and under a proportional cap (fuzz-guests-pins-14) the baseline would dominate small inputs outright. The doc names the quantity "one input's peak transient heap". Distinguish what is measured from what is claimed.

Evidence:

    28	/// Hard ceiling on one input's peak transient heap: 1 GiB.
    29	pub const PEAK_HEAP_CAP_BYTES: usize = 1 << 30;
    ...
    38	    HEAP.reset_peak_usage();
    39	    let r = body();
    40	    let peak = HEAP.peak_usage();

Resolution: Read `let base = HEAP.current_usage();` before the reset and assert on `HEAP.peak_usage().saturating_sub(base)`, the transient quantity the doc names. Acceptance: the asserted quantity is zero for an empty body regardless of process baseline; the doc sentence and the arithmetic agree.
Construction: Hold 900 MiB before the fuzz loop (or grow the live corpus to that size), then run an input whose body allocates 200 MiB: the cap trips with no amplification present.

Synthesis note: the envelope suite's `metered` helper already takes exactly this baseline (`HEAP.current_usage()` after the reset, tests/meter.rs:6907-6910, quoted under gate-legs-1); the fuzz cap is the one heap reader in the tree that does not. fuzz-guests-pins-14 (verification-gap class) carries the proportional-cap question this correction is a precondition for.

### fuzz-guests-pins-27: `synth_version` does 32-bit `usize` arithmetic that wraps seven bytes above the terminal pin, and synthesis failures share the trap channel
- Where: crates/before/wasm32-pins/guest/src/lib.rs:47-58 (related: crates/before/wasm32-pins/guest/src/lib.rs:13-15, crates/before/wasm32-pins/guest/src/lib.rs:133-175, crates/before/wasm32-pins/guest/src/lib.rs:213-216, crates/before/wasm32-pins/guest/src/lib.rs:326-332, crates/before/wasm32-pins/harness/tests/pins.rs:150)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (arithmetic from the quoted line: `4 * n` for `n = 2^30 = 1_073_741_824` is 2^32, which does not fit a 32-bit `usize`; the pinned terminal is `1_073_741_817`, seven bytes under; every other synthesizer computes positions in `u64` and converts at indexing, and `synth_ranked:327` computes `4 * n as u64 - 5` in `u64` before calling `synth_version(n)`); executed: no
- Seen by: structure-prose [39]; instrument-correctness [62]; refutation: confirmed (seven bytes, not six); history: no-rationale-found (written in `usize` when the largest pinned size was 512 MiB; c75d5022 pushed the terminal to within 28 bits of the wrap while writing its own new synthesizers in `u64`)
- Owner-gated: no

With overflow checks on, `pin_version_decode(n)` for `n >= 2^30` traps inside the synthesizer, before `Version::decode` runs, and that trap is byte-identical to the one `version_decode_memory_terminal_traps` pins for a decode-side terminal. The guest's contract (lines 13-15) promises a negative code for the first failed observation; synthesis is exempt (its `expect("the stream is addressable")` and `vec!` allocation failures also trap), so the one outcome class the suite pins red is the one it cannot attribute. Correct at all scales applies to the instrument too, and the wrap is the very genre it audits, occurring in the instrument.

Evidence:

    47	fn synth_version(n: usize) -> Vec<u8> {
    48	    assert!(
    49	        n >= 18,
    50	        "the single-wide-leaf layout needs k = 4n - 5 >= 64"
    51	    );
    52	    let k = 4 * n - 5;
    53	    let mut bytes = vec![0u8; n];
    54	    bytes[0] |= 0x80; // the leaf flag
    55	    bytes[(k + 1) / 8] |= 0x80 >> ((k + 1) % 8); // the mantissa's leading 1
    56	    bytes[n - 1] |= 0x80; // the padding marker

Resolution: Take `n_bytes: u64`, compute `k` in `u64`, place the bits with the existing `set_bit` helper (lines 54-56 and `synth_rank:172` hand-roll the `0x80 >> (pos % 8)` it names; move `set_bit` and `fill_ones` above their first use), and convert to `usize` only for the allocation via `usize::try_from`. Make synthesis fallible in-band: `Vec::try_reserve_exact` and `checked_mul`/`checked_add` returning distinct negative codes, so a trap is a `before` trap by construction and the pins' "the probe backtrace attributes..." sentences become unnecessary. Acceptance: `call1("pin_version_decode", 1 << 30)` returns a negative synthesis code, never a synthesizer trap; `grep -c '0x80 >>' guest/src/lib.rs` is 1.
Construction: `call1("pin_version_decode", 1_073_741_824)`: `4 * n` overflows at line 52 under `overflow-checks = true`; the harness reports `Trapped(UnreachableCodeReached)`, indistinguishable from the pinned terminal.

Synthesis note: fuzz-guests-pins-35 (verification-gap class) asks for the trap-origin discriminator the three memory-terminal pins lack; the in-band synthesis codes here are one half of that discriminator.

### fuzz-guests-pins-32: The harness's fallback guest path resolves one directory above the repository
- Where: crates/before/wasm32-pins/harness/src/lib.rs:45-47 (related: crates/before/wasm32-pins/harness/src/lib.rs:38-40, crates/before/fuzzfit/harness/src/wasm.rs:99-108, justfile:60-67, justfile:77-78, justfile:644)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (`os.path.normpath` of the manifest dir joined with five `..` yields `/Users/oxide/src/target/wasm32-pins/...`, which does not exist; four `..` yields `/Users/oxide/src/rumors/target/wasm32-pins/...`, which the recipe builds into; the fuzzfit twin at the same depth uses four); executed: no
- Seen by: scaffolding [6]; adequacy [30]; structure-prose [36]; instrument-correctness [64]; refutation: confirmed, severity low (the recipe always sets `WASM32_PINS_GUEST_WASM`, so no gate or CI path reaches the fallback); history: no-rationale-found (written in eb6ba627 together with the recipe's target dir; never correct)
- Owner-gated: no

The doc at 38-40 says the fallback is "the workspace-relative target dir the recipe builds into"; it is not. Anyone running `cargo nextest run` in the workspace by hand gets a "not loadable" panic naming a path nothing writes, the exact failure the justfile's own comment at 60-67 warns about. The fuzzfit twin also honors `CARGO_TARGET_DIR` (wasm.rs:103-105), an arm this harness lacks. Correct for all inputs: the unset-variable path is an input.

Evidence:

    45	    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
    46	        "../../../../../target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm",
    47	    )

Resolution: Either drop the fallback and require `WASM32_PINS_GUEST_WASM`, panicking with a message that names the variable and `just wasm32-pins-build`; or fix it to four `..` and mirror the fuzzfit precedence (`CARGO_TARGET_DIR` first), with fuzz-guests-pins-25's `.cargo/config.toml` as the one definition it mirrors. Acceptance: with the variable unset after `just wasm32-pins-build`, `cargo nextest run --cargo-profile release` in `crates/before/wasm32-pins` loads the guest (or fails naming the variable).
Construction: Unset `WASM32_PINS_GUEST_WASM`, build the guest, run the harness tests: every pin panics with `wasm32-pins guest not loadable from /Users/oxide/src/target/...`.

### Fuzz-fit

### fuzzfit-bands-7: `fit()`'s floor fallback contradicts `FIT_FLOOR_BITS`'s doc, and "classifies constant" is not guaranteed
- Where: crates/before/fuzzfit/harness/src/fit.rs:71-73 (related: fit.rs:104-107, fit.rs:114-118, fit.rs:149, crates/before/fuzzfit/harness/tests/enforce.rs:367-395)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (Python over the rule: 8..127 bits spans 1.201 decades and four half-decade buckets, so the constant test at fit.rs:149 is false for an all-sub-floor set; with one floored sample the corner is the same); executed: no
- Seen by: instrument-correctness; refutation: confirmed, with the fit.rs:106-107 over-claim added; history: deliberate-and-holds for the two-sample fallback (b1403c59 restated `fit()`'s doc to the actual guard); the stale half is `FIT_FLOOR_BITS`'s doc (b1c7d31f), which b1403c59 did not touch
- Owner-gated: no

`FIT_FLOOR_BITS`'s doc promises `Fit::min_denom` never sits below the floor when floored samples exist, but `fit` uses the floored subset only when it has at least two members; with exactly one floored sample the whole set is fitted and `min_denom` lands below 128. `fit()`'s own doc says a kernel sampled only below the floor "classifies constant", but sub-floor samples spanning 8..127 bits cover 1.2 decades and four buckets, so such a set fits a size-law slope through the constant-overhead regime the floor exists to exclude. Principle 1 (a contract clause breached is a finding regardless of the input's likelihood) and Principle 5 (prose states what is). No committed band exhibits the corner today; the refit leg runs `fit` on 256-program prefixes where a thinly sampled key could.

Evidence:

        71	/// The fit floor: samples below this denominator are excluded from both
        72	/// the fit and the committed judgment range (`Fit::min_denom` never sits
        73	/// below it when floored samples exist).

       104	/// Samples below [`FIT_FLOOR_BITS`] are dropped first when at least two
       105	/// samples remain above the floor (a slope needs two points); otherwise
       106	/// the full sample set is fitted — a kernel sampled only below the floor
       107	/// fits over what it has and classifies constant.

       114	    let samples: &[(u64, u64)] = if floored.len() >= 2 {
       115	        &floored
       116	    } else {
       117	        samples
       118	    };

Resolution: Either state the actual rule at both docs ("when at least two floored samples exist"; drop "and classifies constant") or make the fallback classify constant by rule (the sub-floor law of record, as `fit_constant` does), so a size-law slope is never fitted through sub-floor points. Acceptance: a unit test in `fit/tests.rs` with sub-floor samples over 8..127 bits plus one at 128 either yields `constant == true` or the docs name the two-sample condition; the test's doc comment states which.
Construction: `samples = [(8,f),(12,f),(16,f),(24,f),(32,f),(48,f),(64,f),(96,f),(100,f),(110,f),(120,f),(127,f),(128,f)]` with `f` constant: `floored.len() == 1`, so all thirteen are fitted; `decades = log10(128/8) = 1.204 >= 1.0`; populated buckets {1, 2, 3, 4} >= 3; `constant` is false and `min_denom` is 8.

### fuzzfit-bands-16: `calibrate` accepts any `programs` count without asserting it covers the refit prefix
- Where: crates/before/fuzzfit/harness/src/bin/calibrate.rs:62-65 (related: calibrate.rs:86, crates/before/fuzzfit/harness/src/bands.rs:205, crates/before/fuzzfit/harness/tests/enforce.rs:358)
- Class / severity / confidence: correctness / nit / high
- Provenance: verified (read: `REFIT_COVERAGE` and the refit evidence derive from `case < REFIT_PREFIX_PROGRAMS` over whatever `programs` was given; nothing asserts `programs >= REFIT_PREFIX_PROGRAMS`); executed: no
- Seen by: raised as new by the refutation pass; history: not examined
- Owner-gated: no

A run such as `calibrate 100` pins a coverage list and refit evidence from a 100-program "prefix" while `enforce.rs` refits 256 programs, so the two legs would silently compare different streams. The module doc names 4096 as the corpus of record but nothing enforces the relation to the prefix. Principle 1: handle every input; a one-line assertion closes it.

Evidence:

        62	    let programs: usize = std::env::args()
        63	        .nth(1)
        64	        .map(|s| s.parse().expect("programs must be a number"))
        65	        .unwrap_or(4096);

        86	            if case < REFIT_PREFIX_PROGRAMS {

Resolution: `assert!(programs >= REFIT_PREFIX_PROGRAMS, "corpus must cover the {REFIT_PREFIX_PROGRAMS}-program refit prefix")` after parsing; name the default as `CORPUS_OF_RECORD` (see fuzzfit-bands-26). Acceptance: `calibrate 100` fails by name before sampling.
Construction: run `cargo run --bin calibrate -- 100`: `REFIT_COVERAGE` is regenerated from 100 programs and the "refit evidence: prefix (256 programs)" line reports over a prefix that does not exist.

### Fuelscape

### fuelscape-pipeline-23: Self-pair overlay points on rows whose kernel opens with canonical_eq measure the equality rung, not the family
- Where: crates/before-fuelscape/src/families.rs:158-165 (related: crates/before-fuelscape/src/families.rs:91-92, 342-349; crates/before/src/version.rs:391, 432, 865, 890, 913, 935, 970; crates/before/src/version/ranked.rs:342; crates/before/src/causally/conjunction.rs:41, 45, 82, 98; crates/before/src/party/ops/compare.rs:16-21, 59-99; crates/before/src/version/skyline/sweep.rs:100; crates/before-fuelscape/src/ops.rs:530-547; crates/before-fuelscape/src/render.rs:501-509; crates/before-fuelscape/src/compact.rs:146-148)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified (read the routing: the guest's `ff_version_join` computes `va | vb` and `ff_version_meet` `va & vb`, whose views open with `codec::canonical_eq` at version.rs:865 and 913 (`join_refs`/`meet_refs` at 890 and 935 the same); `ff_version_span` calls `va.span(vb)`, which routes to `span_refs` and its rung at 970; `distance` and `lag` at 391 and 432; `ranked_cmp`'s `total_cmp` at ranked.rs:342; the conjunction's floor join and ceiling meet route to `join_refs`/`meet_refs` at conjunction.rs:41, 45, 82, 98; `IdReader::is_disjoint` is `lockstep_holds` with `a_settles = Empty`, and any `(Full, Full)` pairing returns `false` at compare.rs:98, so a self-pair refutes at its first owned leaf; `causal_cmp` has only a `ptr_eq` rung (sweep.rs:100) and `causally` contains no `canonical_eq`, so `version_cmp`, `version_concurrent`, and the contains rows are unaffected); executed: no
- Seen by: instrument-correctness [42]; refutation: confirmed end to end, adding the party `is_disjoint` early exit; history: the self-pairs were added as a labeled fallback for signatures without a committed pair generator (69a22b390), and `join_view`/`meet_view` already opened with `canonical_eq` that day, so the join and meet points have read the equality rung from the start; the distance, lag, and ranked rungs arrived two days later (1bb610d19), so that subset is deliberate-but-expired
- Owner-gated: no
- Witness: inconclusive (agent 3: "not constructed: measuring an overlay point requires the fuelscape wasm guest in the detached before-fuelscape pipeline"; the routing was re-read and quoted; the fuel comparison against the `version_eq` row was not measured)

The module doc (families.rs:5-9) says the overlay exists to show "where the adversarial frontier sits relative to the population" because engineered corners are measure-zero in the cloud. On version_join, version_meet, version_span, version_distance, version_lag, ranked_cmp, query_conjoin_floors, and query_conjoin_ceilings the "dense × self" and "hugeleaf × self" points feed byte-identical operands into a kernel whose first rung is a byte compare and a refcount clone, so the marked point is the operation's cheapest path under its most adversarial-sounding label; the party self-pairs on party_is_disjoint exit at the first owned leaf the same way. The roster already reasons about short-circuits correctly elsewhere: the `version_eq` row (ops.rs:534-538) measures the equal pair because there memcmp is the worst case, and the distinct-party row gets committed crosses (families.rs:321-322). The self-pairs are the inverse mistake. The SVG gallery draws these points (render.rs:501-509) and the committed compact datasets carry them (compact.rs:146-148 defers drawing to a later widget revision), so the mislabel is in the audit artifacts now and will reach the doc islands when a revision draws overlays. The sentence at families.rs:91-92 ("unless a committed pair generator exists") also misdescribes the `[Version, Version]` arm, which carries the three pair generators and the two self-pairs together.

Evidence:

       158	        out.extend(ramp("dense × self", max_bytes, |t| {
       159	            let v = version_bytes(&Shape::Dense.packed1(t));
       160	            Some(vec![v.clone(), v])
       161	        }));
       162	        out.extend(ramp("hugeleaf × self", max_bytes, |t| {
       163	            let v = version_bytes(&Shape::Hugeleaf.packed1(8 * t));
       164	            Some(vec![v.clone(), v])
       165	        }));

    (crates/before/src/version.rs)
       890	        if codec::canonical_eq(&a.0, &b.0) {
       891	            return a.clone(); // a ∨ a = a

    (families.rs)
        91	/// Binary rows pair a family with itself (declared by the label) unless a
        92	/// committed pair generator exists — `jump_pair`, `tooth_tail`, and

Witness output (agent 3, read-only):

    ```text
    families.rs:158-165: "out.extend(ramp(\"dense × self\", max_bytes, |t| {\n    let v = version_bytes(&Shape::Dense.packed1(t));\n    Some(vec![v.clone(), v])\n}));\nout.extend(ramp(\"hugeleaf × self\", max_bytes, |t| {\n    let v = version_bytes(&Shape::Hugeleaf.packed1(8 * t));\n    Some(vec![v.clone(), v])\n}));"
    families.rs:91-93: "/// Binary rows pair a family with itself (declared by the label) unless a\n/// committed pair generator exists — `jump_pair`, `tooth_tail`, and\n/// `concurrent_pair` are the pair-shaped families"
    version.rs:890-891: "if codec::canonical_eq(&a.0, &b.0) {\n    return a.clone(); // a ∨ a = a"
    ```

Resolution: On the `[Version, Version]` arm replace the two self-pairs with perturbed twins that defeat the equality rung while keeping the shape (the family and the same family after one tick on its first leaf, or dense(t) crossed with dense at the next ramp point), or drop them and rely on the three committed pair generators already present; do the same for the `[Party, Party]` self-pairs on rows with an early-exit predicate. State at `overlay_inputs` which rows own an equality short-circuit and that self-pairs are reserved for rows without one (version_cmp, version_concurrent, party_covers, the contains rows). Rewrite families.rs:91-92 to describe what the arm does. Acceptance: a committed test in a families.rs sibling `tests.rs`: for every `[Version, Version]` and `[Party, Party]` roster row, every `overlay_inputs` point has `inputs[0] != inputs[1]` unless the row is in an explicit allowlist of rows without an equality rung in the measured kernel.
Construction: Build `Plan { base_seed: 0x5eed, samples_per_column: 1, max_bytes: 64 }`, call `overlay_inputs` on the version_join row at 64, take the largest "dense × self" point, and run `(op.measure)(&mut Guest::new(), &fam.inputs, 2)`; compare its fuel with the `version_eq` row measured on `inputs[0]` at the same size. The two agree up to the clone's constant and both sit far below the "jump_pair" point at the same total size, which is the join sweep's cost.

Synthesis note: the overlay's contract as a whole (a curated hand list presented as "the committed adversarial families") is fuelscape-pipeline-1 (claim class); this entry is the subset of that list that is wrong rather than incomplete. The same genre in other instruments: benches-examples-17 (the bench's `partial_cmp` equal row times the `ptr_eq` rung) and meter-adequacy-7 (the board's `version_eq` row).

### fuelscape-pipeline-19: The samplers recurse on the drawn tree's depth
- Where: crates/before-fuelscape/src/sample.rs:307-315 (related: crates/before-fuelscape/src/sample.rs:463, 495-502; crates/before/AGENTS.md:26-37)
- Class / severity / confidence: correctness / nit / medium
- Provenance: assessed (read: `VersionSampler::subtree` recurses at 307, 312, 315 and `PartySampler::subtree` at 463, 495, 501-502, with no explicit stack and no stated depth bound); executed: no
- Seen by: instrument-correctness [51]; refutation: confirmed (before's rule binds the library, not this detached dev crate); history: no-rationale-found
- Owner-gated: no

`before`'s hard rule ("No library traversal recurses on tree depth") is scoped to the library, and the depth here is drawn from a uniform measure rather than chosen by an adversary, so this is a suggestion: a left-spine version costs 3 bits per level and a unary party chain 2, so a 4096-byte span (the spanbands default) admits depths near 11,000 and 16,000 frames, each carrying several `BigUint` temporaries, against rayon's 2 MiB worker stack. The probability under the uniform draw is negligible; the bound is unstated where the recursion lives.

Evidence:

       307	                return self.subtree(b, Mode::NoZeroLeaf, sink, walk, rng);
       ...
       312	                if !self.subtree(a, Mode::NoLeaf, sink, walk, rng) {
       313	                    return false;
       314	                }
       315	                return self.subtree(b, Mode::Free, sink, walk, rng);

Resolution: Either state the depth bound (bits/3 and bits/2) and the span it implies at the two recursive sites, or iterate with an explicit stack of pending `(bits, mode)` frames as the library's walks do. Acceptance: a comment or an explicit stack at both recursive sites; if iterative, the uniformity and round-trip pins stay green.
Construction: A demonstration, not a test: in a debug build measure one `subtree` frame's size, multiply by the spine depth at `--max-bytes 4096` (32,768 bits / 3), and compare with the 2 MiB rayon worker stack.

Synthesis note: the recursion sweep scanned `crates/before/src` and `crates/suanpan/src` only, so this detached-crate recursion is outside its inventory by construction; it is recorded here so the inventory the sweep's recursion-4 asks for can name it.

### fuelscape-render-27: The typesetting pass rewrites code spans on every workspace crate's rustdoc pages, while the justfile says the script activates only on `.fuelscape` elements
- Where: crates/before/docs/fuelscape.js:1504-1511 (related: crates/before/docs/fuelscape.js:1468, :711, :1544-1545, :1575-1579; justfile:249-258; crates/before/Cargo.toml:9-14; src/link.rs:190)
- Class / severity / confidence: correctness / medium / high
- Provenance: verified by reading and grep (hydrate calls `typesetDocMath(scope)` unconditionally and runs at load on every page; `MATH_SHAPED` matches a lone lowercase letter; the header is injected via workspace-wide `RUSTDOCFLAGS`; a census of doc lines carrying a lone-lowercase-letter code span: 79 in rumors `src/`, 15 in `crates/suanpan/src`, 5 in `crates/before-viz/src`; rustdoc emits `data-current-crate` in target/doc/rumors/index.html and target/doc/before/index.html); executed: no (not rendered in a browser)
- Seen by: instrument-correctness [52]; refutation: confirmed; history: deliberate-but-expired (61f05692 wrote the justfile claim when the script was inert off before's pages; 2efff149 added the pass to reach before's own doc prose and no record considers other crates' pages)
- Owner-gated: no
- Witness: demonstrated (agent 2, under node with a stub DOM holding three `.docblock` code spans and no `.fuelscape` element)

Distinguish what the instrument does from what its documentation says it does: `typesetDocMath` replaces every `.docblock code` element whose whole text has a math variable's form (a lone lowercase letter included) with an `<i>` inside `<span class="fs-math">`, on every page the header reaches, so rumors', suanpan's, before-viz's, and rumors-tracing's rendered docs have code identifiers such as `f`, `k`, or `b` re-set as italic math variables with no visible cause, while the justfile tells a reader the script is inert there. docs.rs builds per crate and is unaffected.

Evidence:

      1504	function typesetDocMath(scope) {
      1505	  (scope || document).querySelectorAll(".docblock code").forEach(code => {
      1506	    if (code.closest("pre") || !MATH_SHAPED.test(code.textContent)) return;
      1507	    const span = document.createElement("span");
      1508	    span.className = "fs-math";
      1509	    typesetInto(span, code.textContent);
      1510	    code.replaceWith(span);
      1511	  });
      1512	}

    justfile:
       252	# per-crate rustdocflags — so non-before pages carry ~40 KB of inert
       253	# head weight; the script activates only on .fuelscape elements.

Witness output (agent 2, `node fuelscape_stub.js crates/before/docs/fuelscape.js`):

    ```text
    before: <#document><div class="docblock"><code>k</code></div><div class="docblock"><code>Vec<u8></code></div><div class="docblock"><code>W</code></div></#document>
    after:  <#document><div class="docblock"><span class="fs-math"><i>k</i></span></div><div class="docblock"><code>Vec<u8></code></div><div class="docblock"><code>W</code></div></#document>
    fuelscape elements present: 0
    lone `k` <code> replaced: true -> <span class="fs-math"><i>k</i></span>
    `Vec<u8>` <code> replaced: false
    `W` <code> replaced: false
    ```

With zero `.fuelscape` elements on the page, loading the script calls `Fuelscape.hydrate()` at load (fuelscape.js:1575-1578), which calls `typesetDocMath(scope)` unconditionally (line 1545); the lone lowercase `<code>k</code>` is replaced, while `Vec<u8>` and the uppercase `W` are left alone.

Resolution: gate the pass on before's own pages: the script already reads `meta[name="rustdoc-vars"]` at line 711, and rustdoc stamps `data-current-crate`, so `hydrate` can run `typesetDocMath` only when `vars.dataset.currentCrate === "before"`. If the owner wants the typesetting workspace-wide instead, correct justfile:253 and the Cargo.toml comment at 9-12 to say so and confirm the other crates' authors accept the face change. Acceptance: after `just docs`, rumors' `Link` doc ("Hand the completed half to `f`.", src/link.rs:190) renders `<code>f</code>` intact while before's `# Complexity` sections are still typeset; the justfile comment matches the behavior either way.
Construction: after `just docs`, open the rendered page for `src/link.rs`'s completed-half method: the `f` code span is an `<i>f</i>` inside `<span class="fs-math">`. Or in node with a stub DOM containing `<div class="docblock"><code>k</code></div>`, call `Fuelscape.hydrate()`: the `<code>` is gone.

### fuelscape-render-3: `aggregate`'s "programmer error" panic is reachable from `--max-bytes`
- Where: crates/before-fuelscape/src/render.rs:211-222 (related: crates/before-fuelscape/src/plan.rs:51-59, :376-387; crates/before-fuelscape/src/bin/fuelscape.rs:95-97, :253-262; crates/before-fuelscape/src/ops.rs:118-128, :1718-1725; crates/before-fuelscape/src/render/tests.rs:16-17)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read plan.rs, ops.rs, the runner loop); executed: no
- Seen by: structure-prose [38], instrument-correctness [53]; refutation: confirmed; history: no rationale found
- Owner-gated: no

Panics are for programmer error only, and every `# Panics` premise must be a one-line proof. `Plan::columns(min_bytes)` returns an empty list whenever `max_bytes < min_bytes` (plan.rs:53-57 loops `while n <= self.max_bytes` from `min_bytes`), `run_op_with_progress` then yields zero samples, and the panel loop calls `render_op` unconditionally (bin/fuelscape.rs:261-262), so a CLI value trips the assert whose doc says every roster row has at least one column; nothing validates `--max-bytes` against `Inputs::min_bytes` (4 for the four-operand rows such as `own_span_contains`, ops.rs:1718-1725).

Evidence:

       211	/// # Panics
       212	///
       213	/// Panics if the atlas has no samples: every roster row has at least
       214	/// one column, and the dump loader rejects empty sample lists, so an
       215	/// empty atlas here is a programmer error.
       216	pub fn aggregate(data: &AtlasData) -> HeatGrid {
       ...
       222	    assert!(!sizes.is_empty(), "an atlas without samples cannot render");

Resolution: after `select` in `main`, reject a plan whose `max_bytes` is below the largest `min_bytes` among the selected rows with a message naming the row and its minimum (or make `Plan` construction fallible), so the `# Panics` premise becomes true by construction. Acceptance: `just fuelscape --max-bytes 3 own_span_contains` exits nonzero naming the 4-byte minimum instead of panicking in `aggregate`.
Construction: `own_span_contains` is `Inputs::Packed` with four operands, so `min_bytes()` is 4; run `FUZZFIT_GUEST_WASM=... cargo run --release --bin fuelscape -- --max-bytes 3 own_span_contains` in crates/before-fuelscape and observe the panic "an atlas without samples cannot render". `--max-bytes 0` reproduces for every row.

### fuelscape-render-6: Positivity of log-scaled quantities is enforced three ways for fuel and not at all for the size axis
- Where: crates/before-fuelscape/src/render.rs:309-319 (related: crates/before-fuelscape/src/compact.rs:179-188, :226, :387-395, :404-410; crates/before-fuelscape/src/dump.rs:234-243; crates/before/build.rs:297-306; crates/before/docs/fuelscape.js:381; crates/before-fuelscape/src/render/tests.rs:39-43; crates/before-fuelscape/src/plan.rs:53)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read all three readers and the widget); executed: no (a read-only scan confirmed the committed dump and dataset hold no zero fuel or zero size, so every case is latent)
- Seen by: scaffolding [11], instrument-correctness [60], adequacy [23] (overlay half), refutation (new item: zero size axis); refutation: confirmed; history: each treatment individually deliberate, no record chooses a gate
- Owner-gated: no

Correct for all inputs means one policy per condition. A zero fuel reading is a measurement bug (`compact` says so and refuses it), yet `dump::read` accepts it and `lg` floors it to `log2(1)` so the SVG plots the bug at fuel 1 with no signal; `compact::validate` rejects a zero size or fuel on overlay points only, never on the `sizes` axis, and `build.rs` likewise requires the axis to be ascending but not positive, so a document with `sizes: [0, ...]` loads through both readers and the widget computes `Math.log2(0)` for its X domain. Only `Plan::columns` (`min_bytes.max(1)`) keeps measured sizes positive.

Evidence:

       309	/// `log2` with a floor of 1 so a degenerate zero reading cannot produce
       310	/// an infinite coordinate.
       ...
       317	fn lg(v: u64) -> f64 {
       318	    libm::log2(v.max(1) as f64)
       319	}

    compact.rs:
       387	    if op.sizes.is_empty() {
       388	        return reject("the size axis is empty");
       389	    }
       390	    if !op.sizes.windows(2).all(|w| w[0] < w[1]) {
       391	        return reject("the size axis must be strictly ascending");
       392	    }

Resolution: reject zero fuel (samples and overlay points) and zero size once at the format's strict gate, `dump::read` (and `DumpWriter::append`), and add a positivity check on `sizes[0]` to `compact::validate` and `build.rs`'s validator; then either drop the floor in `lg` or document it as unreachable given the gate, and extend the smoke assertion at render/tests.rs:39-43 to overlay points. Acceptance: a dump tamper case setting a sample's fuel to 0 is refused by `read` naming the check; a compact tamper case setting `sizes[0] = 0` is refused; `compact`'s own zero check is then a second line and says so, or goes.
Construction: build an `AtlasData` with one `fuel: 0` sample, `DumpWriter::append` it, `dump::read` it back (accepted), `render_op` it (renders, the point at `log2(1) = 0`), then `compact` it (refused). Separately, in `compact/tests.rs`'s tamper closure set `doc["op"]["sizes"][0] = 0`: `read` accepts today.

### fuelscape-render-8: `compact_dump` never runs `validate` on what it writes, and the Strictness paragraph names the wrong consumer of `read`
- Where: crates/before-fuelscape/src/compact.rs:45-53 (related: crates/before-fuelscape/src/compact.rs:226, :243-279, :371, :404-410; crates/before/build.rs:280-282; crates/before-fuelscape/src/bin/fuelscape.rs:131)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (grep for `compact::read` or a `read` import outside compact/tests.rs finds nothing; bin/fuelscape.rs:131 calls only `compact_dump`); executed: no
- Seen by: adequacy [23]; refutation: confirmed; history: the doc-build attribution was a known imprecision (design note §2:104 says the reader is "used by its own round-trip tests"; §2:114-117 says build.rs re-validates); the writer/reader asymmetry has no record
- Owner-gated: no

The cheapest passing artifact must be the intended one: `compact()` copies `data.overlay` through unchecked (226), `compact_dump` calls `dump::read`, `compact`, and `write` and never `validate` (243-279), while `validate` rejects a zero-size or zero-fuel overlay point on read (404-410), so the compactor can write a dataset its own reader refuses and the production path (`fuelscape-verify` diffs bytes only) would not notice. The paragraph's rationale for `read`'s strictness (environmental input to `before`'s doc build) is false as stated: the doc build's reader is `build.rs`, which panics by design; `read`'s consumers are the round-trip and tamper tests.

Evidence:

        45	//! # Strictness
        46	//!
        47	//! A compact dataset is environmental input to `before`'s doc build, so
        48	//! [`read`] rejects malformed data as errors, never panics: unknown
        ...
       226	        overlay: data.overlay.clone(),
        ...
       277	    write(out, &params, &ops)?;

Resolution: call `validate(&path, &op)` on every `WidgetOp` inside `write` (or in `compact_dump` before writing), so the writer's output is by construction what the reader accepts; reword the Strictness paragraph to name `read`'s actual consumers and state that `build.rs` re-checks independently because the detached workspace cannot share the reader. Acceptance: a `compact_dump` test whose dump carries a zero-fuel overlay point fails at compaction naming the overlay check; the module doc no longer attributes `read` to the doc build.
Construction: write a dump via `DumpWriter::append` with `overlay: vec![OverlayData { fuel: 0, size: 2, family: "f".into() }]`; `dump::read` accepts; `compact_dump` succeeds; `compact::read` on the output errors at line 405.

### fuelscape-render-22: `smoothSeries` indexes past the array for one- or two-column datasets, producing a NaN probe path
- Where: crates/before/docs/fuelscape.js:325-331 (related: crates/before/docs/fuelscape.js:472-476, :1344-1360; crates/before-fuelscape/src/compact.rs:387-395; crates/before-fuelscape/src/plan.rs:51-59)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (ran the function body copied from lines 325-342 under node); executed: yes (`smoothSeries([5],2,0.9)` gives `[NaN]`, `smoothSeries([1,2],2,0.9)` gives `[NaN,NaN]`, a three-element input is finite; python over the committed datasets shows 11 to 13 columns per document, so the case is latent today)
- Seen by: adequacy [25], instrument-correctness [57]; refutation: confirmed (same node result); history: no rationale (the mirror extension arrived with the widget port and has no bounds comment)
- Owner-gated: no

Correct at all scales, for all inputs: with radius 2 the mirror reads `vals[-j]` and `vals[2*(n-1)-j]`, which fall outside the array whenever `n <= radius`; `undefined` propagates as NaN through the weighted sum, `monotonePath` emits NaN coordinates, and the probe trace silently fails to draw. `compact::validate` admits a single-column dataset (`sizes` need only be nonempty and ascending), and `Plan::columns` yields one column whenever `min_bytes <= max_bytes < 2*min_bytes`, so the reader and the consumer disagree on the admissible input.

Evidence:

       325	function smoothSeries(vals, radius, sigma) {
       326	  const n = vals.length;
       327	  const at = j => {
       328	    if (j >= 0 && j < n) return vals[j];
       329	    if (j < 0) return 2 * vals[0] - vals[-j];
       330	    return 2 * vals[n - 1] - vals[2 * (n - 1) - j];
       331	  };

Resolution: clamp the mirror index (`vals[Math.min(-j, n - 1)]` and `vals[Math.max(2 * (n - 1) - j, 0)]`), or return `vals.slice()` when `n <= radius`, matching the raw fallback `traceValues` already takes at the extreme quantiles. Acceptance: a dataset with `sizes: [8]` and one column hydrates with a finite probe path (`d` contains no `NaN`); a node check of the extracted function gives finite output for arrays of length 1 and 2.

### fuelscape-render-23: The probe trace draws a five-column smooth of the quantile while its label, handle, readout, and guide anchor use the raw value
- Where: crates/before/docs/fuelscape.js:472-476 (related: crates/before/docs/fuelscape.js:322-324, :491-500, :1165-1173, :1356, :1379)
- Class / severity / confidence: correctness / low / medium
- Provenance: assessed (read `traceValues`, the trace path from `tv.sm`, the guide crossing from `tv.raw[aI]`, the handle from `tv.raw[li]`); executed: no
- Seen by: instrument-correctness [56]; refutation: confirmed (the geometric inconsistency is a fact; whether smoothing removes signal at 4096 samples per column is a judgment); history: deliberate-but-expired (the smoothing's premise at 322-324 is that the anchor is an endpoint; 2efff149 added interior click-to-anchor without revisiting it)
- Owner-gated: yes (a presentation choice)

The widget's own rule is to present shapes faithfully. For interior quantiles the drawn trace is `smoothSeries(raw, 2, 0.9)` (center weight about 0.44, neighbors 0.24, second neighbors 0.04), while the guide crossing, the slider handle, and the readout use the raw per-column quantile; the mirror extension preserves endpoints, so the default rightmost anchor agrees, but once a reader locks an interior column the handle and the guide crossing sit off the drawn line by the smoothing residual, and a line labeled "median" or "pNN" is not the per-column quantile there. The comment at 1165-1167 promises guides intersect the probe at the anchor column; with an interior anchor they intersect the raw value, not the drawn trace.

Evidence:

       472	  traceValues(q) {
       473	    const raw = this.data.cols.map(col => this.quantAt(col, q));
       474	    const sm = (q <= 0 || q >= 1) ? raw.slice() : smoothSeries(raw, 2, 0.9);
       475	    return { raw, sm };
       476	  }

Resolution: draw the raw quantiles (the natural choice at 4096 samples per column), or keep the smoothing and anchor guides and the handle to `tv.sm`, disclosing it in the y-label or probe tooltip ("median, smoothed across neighboring columns"). Acceptance: with an interior column locked, the slider handle, the active guide's crossing, and the drawn trace coincide at that column, and the label names what is drawn.
Construction: open any island (`version_tick`), select the `n` hypothesis, click the size-64 column to lock it, and compare the handle's y (raw median) with the trace's y at the same x (smoothed); they differ by the five-point residual, largest where the median's slope changes between columns.

### fuelscape-render-31: `build.rs`'s re-validation compares untyped JSON values, so non-integer sizes and counts pass, and an `expect("validated")` names a check that never ran
- Where: crates/before/build.rs:303-315 (related: crates/before/build.rs:58-63, :216, :280-282; crates/before-fuelscape/src/compact.rs:379-412)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read; `Option<u64>` orders `None` below every `Some`, so `w[0].as_u64() < w[1].as_u64()` is true when the first size is not an integer; `v.as_u64() != Some(0)` is true for any non-integer end bin; lines 58-63 compare `Value == Value`, so `Null == Null` passes when both files lack a parameter, and line 216 then panics with "validated"); executed: no
- Seen by: adequacy [24], instrument-correctness [55]; refutation: confirmed; history: no rationale found (design note §4:171 fixes "Build-deps: serde_json only", which explains the absence of derive typing but not the `Option` comparisons)
- Owner-gated: no

A check that passes a malformed artifact is decoration, and every `expect` message is a one-line proof. The validator's stated reason to exist (280-282) is that it cannot rely on the compactor's reader, yet it is weaker than that reader on exactly the shapes it names; the malformed values then ride verbatim into the island JSON, where the widget computes `Math.log2(null)` with nothing reporting it. Reachable only through a hand-edited committed file at gate tier (`fuelscape-verify` in `ci` would catch the byte difference), hence low.

Evidence:

       303	    assert!(
       304	        sizes.windows(2).all(|w| w[0].as_u64() < w[1].as_u64()),
       305	        "{file}: the size axis must be strictly ascending"
       306	    );
       307	    for col in cols {
       308	        let c = col["c"].as_array().expect("histogram counts are a list");
       309	        let ends_nonzero = c.first().is_some_and(|v| v.as_u64() != Some(0))
       310	            && c.last().is_some_and(|v| v.as_u64() != Some(0));
       ...
       216	        "seed": format!("{:#x}", meta["base_seed"].as_u64().expect("validated")),

Resolution: demand the types before comparing: collect `sizes` and each `c` into `Vec<u64>`, panicking with the file name on any non-integer entry, then compare plain integers; validate `base_seed` and `samples_per_column` as `u64` in the meta loop so the `expect` at 216 becomes true or dissolves into a checked value (or deserialize `meta`, `sizes`, `cols` into small typed structs with `deny_unknown_fields`, mirroring the compactor's). Acceptance: a committed document with `"sizes":[null,2,...]` or `"c":["x",1]` fails `cargo build -p before` naming the file and check; `"base_seed":"7"` in both files fails naming the parameter rather than panicking with `validated`.
Construction: in a scratch copy of `crates/before/fuelscape/`, set `op.sizes[0]` to `null` in one document: line 304's `all(...)` returns true (`None < Some(2)`) and the emitted island carries the null. Delete `meta.base_seed` from both `index.json` and that document: lines 58-63 pass (`Null == Null`), then line 216 panics with `validated`.

Synthesis note: deps-5 below records the `expect("validated")` clause of this entry on its own; fuelscape-render-30 (verification-gap class) is the neighboring `rerun-if-changed` gap in the same script.

### deps-5: build.rs `expect("validated")` on `meta.base_seed` names a proof that does not exist
- Where: crates/before/build.rs:216-216 (related: crates/before/build.rs:58-67, crates/before/build.rs:283-316, crates/before/build.rs:19-21)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (traced every check applied to `meta`: lines 58-63 compare doc and index values for equality, 64-67 check `commit` is a string, `validate` at 283-316 checks `op` fields only); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The only check on `meta.base_seed` is equality with the index's value, which two string-typed or two absent values also satisfy. The `.as_u64().expect("validated")` therefore panics with the word "validated" on a dataset whose seed is a string or missing in both files, instead of the file-and-check message the module doc promises. The sibling expects at 202-203 are accurate: `validate` checks `contract` and `claim` as non-empty strings via `as_str()`.

Evidence:

        19	//! hold. Every failure here is a defect in the committed repository
        20	//! (malformed data, a stale derived header), so it panics naming the
        21	//! file and check rather than reporting errors to a caller.
        ...
        58	        for param in ["base_seed", "samples_per_column"] {
        59	            assert_eq!(
        60	                doc["meta"][param], index["meta"][param],
        61	                "{file}: run parameter {param} differs from the index's"
        62	            );
        63	        }
        ...
       216	        "seed": format!("{:#x}", meta["base_seed"].as_u64().expect("validated")),
        ...
       285	        let s = op[key].as_str().unwrap_or("");

The doctrine is that every expect message is a one-line proof; this one borrows its neighbors' wording without their backing.

Resolution: validate the index's `meta` once (`base_seed` and `samples_per_column` as u64, `commit` as a non-empty string) in a `validate_meta(file, meta)` beside `validate`, after which the expect message is true; or replace the expect with `unwrap_or_else(|| panic!("{file}: meta.base_seed must be an integer"))`. Acceptance: the construction below panics naming fuelscape/index.json and the field.

Construction: set `"base_seed": "0x1"` (a string) in fuelscape/index.json and in every fuelscape/<op>.json, then `cargo build -p before`: at HEAD the panic message is `validated`, naming neither file nor field.

Synthesis note: the third clause of fuelscape-render-31; the two resolutions are one `validate_meta`.

## Positives

What the review found correct is as much a result as what it found wrong, and several of these are the reason the list above is short. Each item names what was checked and by whom; "verified" means a reviewer read or traced it against the code, "executed" means a run.

- **No production kernel in `before` returns a wrong answer on a 64-bit target.** The party, version-core, rank, skyline-coding, comparison, watermark, fill-and-grow, codec-bits, and codec-base-text-tree partitions each traced their kernels arm by arm and report no reachable wrong value or panic on 64-bit hosts; the surviving production defects are all width boundaries (32-bit targets, `u32` caps) or cost claims. Every `expect`, `unreachable!`, and `panic!` message in the shipped code reads as a one-line proof that a reviewer could discharge, with two exceptions recorded above: `expect("freeze count fits u32")` in `EpochLedger::epoch` states its conclusion rather than its premise (skyline-query-24), and `expect("site count fits u32")` in the fill walk's frame ledger is reachable by input scale (inventory-1, skyline-fill-grow-23) (verified per partition; the inventory sweep counts 143 `expect` sites and 34 `unreachable!` and zero `unwrap()` outside doctests).
- **The no-depth-recursion rule holds.** The recursion sweep's call-graph scan over both crates finds no library function recursing on input tree depth, every explicit stack states its per-level cost at its declaration, and the deep-input proof is broader than AGENTS.md advertises: three depth-100k clock tests, the id text parser at 100k, the id diff ladder to 100k, and envelope rows at `ID_DEPTH = 250_000` including the public `without` (verified by the sweep's scan and reading). That breadth is on the operation and depth axes; on the shape axis the proof is narrower than its headline: the depth-100k tests drive a listed subset of operations over one left-only spine family, so both-present id frames and right descents have no overflow-depth witness, and the public shape iterators appear in no deep test (clock-22, recursion-6, meter-adequacy-1, in the verification document).
- **Canonical form is byte equality, and the storage seam enforces it.** Marker padding plus the one-bit-minimum grammars make the stored bytes injective on bit streams (codec-bits, verified); `Bits::freeze` and `from_canonical` are the only storage gates; `padding_is_canonical` and `require_marker_padding` split the `Truncated`/`TrailingBits` genres by pattern rather than arithmetic, and the reject corpus re-derives every accepted mutant through the oracle bridge, the one comparison a lax validator cannot pass (skyline-coding, verified). codec-bits-8 is the one corner where the two padding judges disagree, and every caller consumes a bit before reaching it.
- **The decode doors are strict, single-allocation, and genre-exact.** `Clock::decode` parses the id once and validates both components against one borrowed buffer; `Span::decode` pronounces the pair verdict after the padding check so structural defects win, with byte-level witnesses for the precedence; the borsh `ReaderCursor` pulls one byte per demanded bit with the word window proven unable to touch an unyielded byte; `Rank::decode` allocates only from bits actually read and assembles the numerator by byte concatenation rather than a value-width shift a 32-bit `usize` cannot hold (clock, span-causally, crate-root, rank; verified). The fresh-eyes sweep's hostile probes (empty input, a party alone, trailing bytes) each returned the documented variant with no panic (executed).
- **`DsiCursor` keeps the accept/reject boundary in the wrapper.** It refuses `dsi-bitstream`'s capped `read_gamma` with the reason stated, and the per-bit loop is the sole arbiter of every reject, pinned against the word-parallel path on arbitrary bytes and at every cut point (codec-bits, verified).
- **suanpan's kernel is reviewably correct on 64-bit targets.** `add_at`'s recentering (carry `(t + 2^31) >> 32`, remainder in `[-2^31, 2^31)`), `read_digits`' final-carry closure over `[-3, 2]`, the `SIGN_DECIDED = 3` margin against the `2.01` geometric tail, and the ledger's three maintainers were each traced by hand; `sign_dominates_at`'s register arm is total by construction (`checked_add`/`checked_mul`/`try_from`/`checked_shl`), and `assert_value` drives all three read-outs against an exact `IBig` oracle on every value check (suanpan, suanpan-tests; verified).
- **32-bit totality is handled correctly where it was considered.** `Base`'s `Shl`/`Shr` clamp rather than truncate, with the totality argument at the code; `IdIndex::build` degrades to the unindexed walk past `u32::MAX` bits instead of panicking, the pattern the frame ledger lacks; `Split` is iterative so a huge fork count cannot overflow the call stack; the wasm32-pins workspace keeps overflow checks on in release so a wrap traps (codec-base-text-tree, party, inventory sweep; verified; the 32-bit pass confirmed the last point by run: the checked guest traps on suanpan-24's constructions where a guest built with overflow checks off returns a wrong value).
- **Linearity is a compile-time fact.** `assert_not_impl_any!(Party: Clone, Copy)` and `assert_not_impl_any!(Clock: Clone, Copy)` sit at the definitions, and the `|` matrix's shape (no `Clock | Clock`, no borrowing `&Clock` receiver) is pinned by four static assertions with the rejected cells' reasons stated (clock, api-audit; verified).
- **Every fallible reunion hands identity back.** `Party::join`/`Clock::join` return the operand in `Err`, `join_all` returns the overlapping set, `Forks`' `Drop` rejoins unconsumed shares, and `sync`/`sync_all` leave every clock untouched on overlap (api-audit, clock; verified).
- **The instruments commit their own known-bad artifacts and convict them.** The semantic oracle's cell-dropping Riemann sum and mirrored embedding are committed behind inverted assertions; `join_all_differential_convicts_the_dropped_group_oracle` proves the differential criterion can fail; `merge_refuses_a_silently_shrunk_grid_for_every_family` sweeps a one-defect shard capture over the axis the refusal discriminates on; `dump::read` recomputes every stored grid and refuses disagreement with a committed tamper demonstration; `fuzz_decode` asserts byte identity on accept, refusing an accept-and-normalize decoder outright (testing-oracles, party, board-ops-render, fuelscape-render, fuzz-guests-pins; verified).
- **The measurement cores take their baselines correctly.** The envelope harness resets every counter, takes the heap baseline after the reset, runs only the operation, and reads before any formatting allocation; the board's `measure` does the same and settles I/O denominators from the actual result (envelopes-a, envelopes-b, board-frame; verified). gate-legs-1 is a defect in what the instrumented build allocates, not in the harness's arithmetic; fuzz-guests-pins-15 is the one heap reader that skips the baseline.
- **Where a review dispute could be settled by running, the witness pass ran it.** Nine correctness entries were demonstrated by constructed tests or programs against the shipped code: five in the main witness pass (crate-root-34, surface-roster-28, tests-other-26, tests-other-27, fuelscape-render-27) and four in the 32-bit pass in the `wasm32-pins` executor (suanpan-24, codec-bits-23, clock-17, party-13); fuelscape-render-22's node check was the partition finalizer's own run, recorded in that entry's provenance rather than in the witness record. Each construction, log, and cleanup record is archived under `scratchpad/before/witness/` (the 32-bit pass under `witness/wasm32/`); every worktree was restored to a clean tree afterwards, with the restoration verified by `git status --porcelain` (executed).

## Open questions for Finch

Each question is one decision that resolves one or more entries above; the recommendation is mine and the reasons are stated so a different ruling can be made on the same facts.

1. **The fork iterators' `ExactSizeIterator` on 32-bit targets** (clock-17, party-13). The choices are (a) drop the impl from `Forks`, `party::Forks`, and `Split` (an API removal; `len()` disappears and iter.rs:16's doc example changes), (b) take the count as `usize`, or (c) keep the impl and document under `# Panics` that `len()` panics past `usize` on narrow targets, or override `len()` to saturate. The clock reviewer recommended (a) and the party reviewer the saturating override; a saturating `len()` returns a count the trait promises is exact, so it trades a panic for a wrong value. Recommendation: (c) as `# Panics` now, since the API is declared stable and a documented panic is the smallest accurate contract without a breaking change; schedule (a) for the first breaking release. Either way, a red-first wasm32 pin on `forks(1u64 << 32)` lands with the choice.
2. **The frame ledger's `u32` link index** (inventory-1, skyline-fill-grow-23, recursion-5) **and the `u32` epoch** (skyline-query-24). Widen both to `usize` (8 bytes per queue cell on 64-bit, still niche-packed; a few bytes of alignment per stacked `Reign`), re-pinning the memo rows' heap columns with attribution, or keep the caps and state them in the public `tick`/`ticks` `# Panics` sections and in the `expect` messages as derived bounds. Three of four reviewers recommend widening; the recursion sweep recommends ruling and documenting. Recommendation: widen. The crate's own sentence at lib.rs:351-353 makes "for all input sizes" the promise, the 4-byte cell buys a constant factor, and `IdIndex` already shows the crate's preferred shape when a `u32` table is exceeded.
3. **A first-party 32-bit run for suanpan and the borsh path** (suanpan-24, codec-bits-23, clock-17). The `wasm32-pins` guest names suanpan nowhere and prices `Version::decode`'s working set only. Recommendation: extend the guest with a suanpan leg (the two `add_limbs_shl`/`shl` constructions in suanpan-24 plus the `Limbs` round trip that suanpan-26 asks for), a `forks(1u64 << 32).len()` pin, and a synthetic `BitCursor` yielding `2^32 - 32` zero bits for the gamma guard. The fixes for suanpan-24 and codec-bits-23 land regardless; the pins are what keep them landed. The 32-bit pass's guest exports and harness tests (`witness/wasm32/witness.rs`, `zz_witness.rs`) are these constructions already written in the pins workspace's idiom; each asserts the defect as the tree stands, so they are red-first pins for the fixed tree once their expected outcomes are flipped to the chosen contract.
4. **The serde deserialize door** (crate-root-34). A new optional `serde_bytes` dependency under the `serde` feature, or a local two-way visitor to keep the dependency surface fixed? Recommendation: `serde_bytes` (mature, tiny, and exactly this problem), with `serde_test` as a dev-dependency so the strict-format witness is a one-line `assert_tokens` rather than the witness pass's hand-rolled Deserializer. The fresh-eyes sweep's separate question about a human-readable serde form is a different decision and should not block this one.
5. **The coverage leg's heap reading** (gate-legs-1). What allocates the extra 772 B in `masked_cmp_hole_envelope` under `cargo llvm-cov nextest` and not under `cargo nextest run`, and why did the previous coverage run on an identical tree pass? Only the coverage leg can answer. Recommendation: run `just coverage-kernel` twice at `9e5784fb` before any other action; if the instrumented reading moves between runs, isolate the meter or exclude the heap column under coverage builds with the reason at the exclusion; if it is stable but differs, filter the meter suite out of the coverage recipes. Never widen the 480 B pin.
6. **The line-scan extractor** (surface-roster-28, -29, -21; surface-roster-9 in the scaffolding class). Retiring before's line scan in favor of surfacecheck dissolves surface-roster-21 and the before-side exposure of -28 and -29, but suanpan's claims roster runs on the same extractor with no rustdoc-JSON leg. Recommendation: land the catch-all panic and `pub const fn` acceptance in `surface-scan` now (they are owed for suanpan whatever happens to before), then retire before's line scan as surface-roster-9 proposes.
7. **The fuelscape overlay's self-pairs** (fuelscape-pipeline-23). Replace the `[Version, Version]` and `[Party, Party]` self-pairs with perturbed twins that defeat the equality rung, or drop them and rely on the three committed pair generators? Recommendation: perturbed twins (the family after one tick on its first leaf), with the committed allowlist test naming the rows that legitimately have no equality rung; the committed compact datasets then re-generate, which is the point.
8. **`typesetDocMath`'s reach** (fuelscape-render-27). Gate the pass on `data-current-crate === "before"`, or accept workspace-wide typesetting and correct the justfile and Cargo.toml comments? Recommendation: gate; the other crates' authors did not opt into italic math for their lone-letter code spans, and the justfile already states the gated behavior as the intent.
9. **The masked height debug-asserts** (skyline-sweep-place-masked-3). Delete the two `debug_assert_ne!` lines, or amend the `# Panics` contract to name them? Recommendation: delete; the asserts check an input property the validator owns, the differential laws separate any fold-orientation bug, and the pair sweep's identical sentence is true without them.
10. **The clock text door's whitespace class** (codec-base-text-tree-20; owner-gated because it narrows a public `FromStr`). Trim with the cursor's ASCII predicate so the three doors agree, or document the stamp door's Unicode edge as intended? Recommendation: the ASCII predicate, pinned beside `id_text_parser_error_precedence_pins`; codec-base-text-tree-18's precedence question is the same "three doors spell one notation" decision and should be ruled with it.
11. **The probe trace's smoothing** (fuelscape-render-23; owner-gated as presentation). Draw raw quantiles, or keep the five-column smooth and anchor the guides and handle to it with the smoothing disclosed? Recommendation: raw quantiles; at 4096 samples per column the smoothing removes column-to-column structure rather than noise, and interior anchoring exposes the inconsistency.
12. **CI toolchain and tool pinning** (surface-roster-1, gate-legs-2). Derive the nightly from `just --evaluate nightly_toolchain` wherever a `+{{ nightly_toolchain }}` recipe runs, and pin `cargo-mutants@27.1.0` beside `cargo-rdme@2.1.0`? These are not design questions; they are recorded here because the reworded step comment (which tools are committed expectations) is a small policy statement worth your wording. Recommendation: both, in one workflow change, with `tools/workflowlint` holding the cargo-mutants version to `tools/mutantcheck-expected.json`.

## Counts

By severity, over the 51 correctness entries:

| Severity | Count | Ids |
|---|---|---|
| high | 2 | suanpan-24, gate-legs-1 |
| medium | 13 | board-ops-render-12, clock-17, codec-bits-23, crate-root-34, fuelscape-pipeline-23, fuelscape-render-27, party-13, surface-roster-28, tests-other-26, tests-other-27, gate-legs-2, inventory-1, tools-22 |
| low | 29 | benches-examples-8, board-families-floors-judge-27, board-frame-15, board-frame-21, board-frame-23, codec-base-text-tree-20, fuelscape-render-3, -6, -8, -22, -23, -31, fuzz-guests-pins-15, -27, -32, fuzzfit-bands-7, skyline-fill-grow-23, skyline-query-24, skyline-sweep-place-masked-3, surface-roster-1, -21, -29, testing-oracles-4, api-audit-2, clippy-pedantic-2, deps-5, recursion-5, tools-17, tools-25 |
| nit | 7 | benches-examples-24, board-ops-render-31, codec-bits-8, envelopes-b-28, fuelscape-pipeline-19, fuzzfit-bands-16, inventory-7 |

By module (the section order above), with severities:

| Module | Entries | high | medium | low | nit |
|---|---|---|---|---|---|
| Crate root and public types (clock, party, error) | 3 | 0 | 2 | 1 | 0 |
| Skyline coding (fill and grow, comparison kernels, query) | 5 | 0 | 1 | 4 | 0 |
| Codec (bits; base, text, and tree) | 5 | 0 | 1 | 2 | 2 |
| Cross-cutting (serde) | 1 | 0 | 1 | 0 | 0 |
| suanpan | 1 | 1 | 0 | 0 | 0 |
| The board | 6 | 0 | 1 | 4 | 1 |
| The surface roster | 4 | 0 | 1 | 3 | 0 |
| The test harness | 1 | 0 | 0 | 1 | 0 |
| The envelopes | 2 | 1 | 0 | 0 | 1 |
| The gate and CI workflow | 1 | 0 | 1 | 0 | 0 |
| Workspace tools (tools/) | 3 | 0 | 1 | 2 | 0 |
| Other suites | 2 | 0 | 2 | 0 | 0 |
| Benches and examples | 2 | 0 | 0 | 1 | 1 |
| Fuzz and pins | 3 | 0 | 0 | 3 | 0 |
| Fuzz-fit | 2 | 0 | 0 | 1 | 1 |
| Fuelscape | 10 | 0 | 2 | 7 | 1 |
| Total | 51 | 2 | 13 | 29 | 7 |

Distinct defects are fewer than entries: clock-17 and party-13 are one defect; inventory-1, skyline-fill-grow-23, and recursion-5 are one; clippy-pedantic-2 and inventory-7 are one; board-families-floors-judge-27 and board-frame-15 are one; deps-5 is a clause of fuelscape-render-31. Counted that way there are 45 distinct defects (the three `tools/` entries are distinct from every other correctness entry; tools-22 shares its doclint half with gate-legs-10 in the simplification document). Six entries are owner-gated (clock-17, party-13, codec-base-text-tree-20, skyline-fill-grow-23, recursion-5, fuelscape-render-23); the rest need no API decision, though inventory-1's two resolutions are the owner's choice between a wider index and a documented cap.

## Refuted by construction

No correctness entry was refuted. The witness pass constructed twelve of the 51 (the three `tools/` entries postdate the witness pass; each was executed by the partition's finalizer against the tools' own code, as its provenance line says). In the main pass five were demonstrated (crate-root-34, surface-roster-28, tests-other-26, tests-other-27, fuelscape-render-27) and seven were inconclusive for platform reasons rather than on the merits (party-13, clock-17, codec-bits-23, and suanpan-24 needed a 32-bit target; inventory-1 needs a multi-GiB input and a link store the host cannot hold; fuelscape-pipeline-23 needs the detached fuelscape guest; gate-legs-2 needs a second cargo-mutants release installed); fuelscape-render-22's node check is the finalizer's own run, recorded in that entry's provenance and not a witness section. The 32-bit pass then ran the four 32-bit constructions in the tree's `wasm32-pins` executor and demonstrated each, so the final tally over the twelve is nine demonstrated and three inconclusive (inventory-1, fuelscape-pipeline-23, gate-legs-2). For each entry still inconclusive the witness re-read the cited lines and the pinned dependency source and found the mechanism consistent with the code; the runtime effect was not observed, and the entries above say so.
