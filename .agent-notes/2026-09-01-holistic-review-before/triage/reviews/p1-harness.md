<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as the review packet of the before triage lane p1-harness; the annotations under each hunk are the lane agent's own account, the sections above them the coordinator's; not authored, audited, or endorsed by Finch. Reply inline with lines beginning `>> finch:`. -->

# Review packet: lane `p1-harness` (branch `before/p1-harness`)

## Goal

`crates/before/tests/meter.rs` ran one measurement procedure through four envelope structs and four harness bodies that differed only in which columns existed, so a row could pin only the columns its struct carried. This lane unifies them into one envelope type carrying every column, pinned on every row, one harness, one statement of the pin convention; deletes the stack-segments column as a currency; derives scan floors from irreducible work with a committed negative control; and retires the kernel rows whose public-entry twins pin the same shape and operation. Every pre-existing reading is byte-identical before and after; every newly pinned cell was measured at the parent and is attributed in its commit.

## Rulings landed

4 (the harness unifies first; every column pinned), 38 (the segments currency dissolves; this lane's half), 50 (scan floors on every nonzero row, envelopes-a-6), 51 as amended by 108 (the suite's feature guard is `required-features`), 108 (the rank twins retire; the wide-arming render row), 105 (prose).

## Stack position

Parent: `main`. Children: `before/p2-widths` (its two memo-row re-pins are written against this lane's harness), then `before/p1-board`, `before/p1-suites`, `before/p2-rows` in that order, each editing `tests/meter.rs`.

## Acceptance

| Entry | Commit | Command | Decisive output |
|---|---|---|---|
| envelopes-a-4, envelopes-b-8 (ruling 4) | `ff8788f6` | `grep -c '^fn [a-z_]*metered' / '^struct [A-Za-z]*Envelope' / 'tripwire (measured x0.75)' crates/before/tests/meter.rs` | 1 / 1 / 5 (one harness copy plus the four band bodies; the parent had 10) |
| envelopes-a-2, crate-root-32 (ruling 38) | `85849ce0` | `grep -ci 'stack.segment\|grown segment\|segments:' crates/before/tests/meter.rs` | 0; the file doc names the three clock::tests stack-safety tests and what each drives |
| envelopes-a-6 (ruling 50) | `2d509f21` | `the meter suite on the box; the two probe rows' outputs` | validate_stub_probe and validate_stopped_probe each fail the liveness floor by name (coord-acceptance-final.log) |
| envelopes-a-8, envelopes-a-11 (rulings 4, 108) | `2c1af31c` | `grep -c 'meter::skyline::decode(' crates/before/tests/meter.rs; grep -c SKYLINE_RANK_ crates/before/tests/meter.rs` | 1 (the round-trip unit test); 0 |
| meter-adequacy-11 (rulings 51, 108) | `09724161` | `cargo nextest run --locked -p before --test meter --no-run (default features); cargo check --locked -p before --all-targets; cargo clippy --locked -p before --lib --tests -- -D warnings` | exit 101: "target `meter` in package `before` requires the features: `limb-meter`, `scan-meter`"; exit 0; exit 0 |
| the `just test` recipe line (ruling 108) | `4d5c88ee` | `just test meter (on the box)` | builds the meter suite under the lit features; the run itself reads red only in the rumors proxy test file that is red on main for every workspace build (see the queue's open item), not in anything this lane touches |
| the suite itself | `6f13ce53` | `cargo nextest run --locked -p before --all-features -E 'binary(meter)' on the box (coordinator's run at the rebased tip)` | 169 tests run: 169 passed; all 295 MEASURED lines byte-identical to the lane's final run |
| gate of record (lane's run inside a processor set; coordinator read the log) | `6f13ce53` | `pset-run -n 32 -- just gate on the box (gate-round2.log)` | every leg ok except fuzz (libFuzzer has no illumos port: the expected, sole red leg); no timeout-only failures |

## Fresh-eyes rounds

Two rounds, each by a reviewer that had read neither the brief nor the lane's transcript, briefed with the invariants and told to dispute.

**Round 1 (surface correctness).** Three defects and seven weaknesses. Repaired: the `RANK_*`/`SKYLINE_RANK_*` twin pair (retired under ruling 108); `CMP_BIGROOT` carrying a looser heap ceiling than its retired twin (now 39_540); the file doc misattributing the scan floor's premise for the id walks; the whole-input floor on `ID_COVERS`/`ID_DISJOINT` leaving four bits of margin against a legitimate early exit (now a `LiveBits` floor with a stated decision-point tail); the stub negative control failing on the touch tripwire before the scan floor was judged (both probes now reach the scan floor); the harness self-test reading counters after `Debug` formatting; a portability claim for the touch and scan columns nothing demonstrated (now: deterministic on a target); a hand-maintained dependency version string in prose; the no-recursion attribution overclaiming the named test's coverage; the dropped `WideArming` render coverage (a row under ruling 108).

**Round 2 (blast radius of the manifest change, operational validity of the repairs, attribution, prose).** No defect in the blast radius: every gate and CI leg that judges the suite lights both meter features (`just check` and the default-feature clippy leg skip the target, by design). Two prose defects and four weaknesses, repaired in the final round: a false sentence about deep-input witnesses (three stack-safety tests cover what it said nothing covered); twelve feature gates that can never be false under `required-features`; the decision-point tail (4 bits) contradicting its own 2-bit derivation; the `LiveBits` doc overstating what the scan counter attests on rows that also write; two justfile recipe comments (what `just test` builds under feature unification; `just check` no longer type-checking the suite).

**Reviewer notes not acted on.**
- `JOIN_WIDE_TOOTH`'s limb ceiling derives from an older pin-time reading while its floor derives from the parent's reading, an unattributed pre-existing rise of about 8.6% under the standing ceiling. Not this lane's to re-pin (it moved no reading); carried to `p2-rows`, which measures every row, to attribute or re-pin.
- The `id_walk_scan_cost` band's own liveness floor (one bit per byte) now sits under the envelope's floor with a different premise. The bands are the suites lane's; carried to `p1-suites`.
- Deep-input rows for project, distance, lag, and the masked comparisons (the envelope suite has none): carried to `p2-rows`.
- `Column::pin` and `pin_mut` duplicate one accessor: a setter is needed either way; left.
- The stopped-validator control proves the floor sits above half the input, not that it is tight; adequate for a floor derived rather than measured; left, noted here.
- `covcheck` (CI-only, skyline kernel scope) could in principle lose a kernel line to the six retired rows; the door rows call the same kernels and read identically, so no movement is expected; CI's coverage job is the check.

## Stops

None open. Ruling 108 resolved the three the lane raised.

## Reading order

### new tests and negative controls

- envelopes-a-4 (4) at `crates/before/tests/meter.rs:553` ([hunk](#hunk-7))
- fresh-eyes-taste (105) at `crates/before/tests/meter.rs:526` ([hunk](#hunk-7))
- envelopes-a-6 (50) at `crates/before/tests/meter.rs:1533` ([hunk](#hunk-33))

### tests and prose

- meter-adequacy-11 (108) at `crates/before/Cargo.toml:133` ([hunk](#hunk-2))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1` ([hunk](#hunk-3))
- envelopes-a-2 (38) at `crates/before/tests/meter.rs:31` ([hunk](#hunk-3))
- envelopes-a-2 (38) at `crates/before/tests/meter.rs:1` ([hunk](#hunk-3))
- envelopes-b-8 (4) at `crates/before/tests/meter.rs:78` ([hunk](#hunk-3))
- meter-adequacy-11 (108) at `crates/before/tests/meter.rs:47` ([hunk](#hunk-3))
- fresh-eyes-F2 (105) at `crates/before/tests/meter.rs:69` ([hunk](#hunk-3))
- fresh-eyes-F4 (105) at `crates/before/tests/meter.rs:33` ([hunk](#hunk-3))
- fresh-eyes-F7 (105) at `crates/before/tests/meter.rs:88` ([hunk](#hunk-3))
- fresh-eyes-D1 (105) at `crates/before/tests/meter.rs:32` ([hunk](#hunk-3))
- fresh-eyes-D2 (108) at `crates/before/tests/meter.rs:154` ([hunk](#hunk-4))
- envelopes-a-4 (108) at `crates/before/tests/meter.rs:215` ([hunk](#hunk-5))
- envelopes-b-8 (4) at `crates/before/tests/meter.rs:235` ([hunk](#hunk-6))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:248` ([hunk](#hunk-6))
- envelopes-a-6 (50) at `crates/before/tests/meter.rs:258` ([hunk](#hunk-6))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:293` ([hunk](#hunk-6))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:323` ([hunk](#hunk-6))
- envelopes-b-8 (4) at `crates/before/tests/meter.rs:336` ([hunk](#hunk-6))
- envelopes-b-8 (4) at `crates/before/tests/meter.rs:239` ([hunk](#hunk-6))
- envelopes-b-8 (4) at `crates/before/tests/meter.rs:341` ([hunk](#hunk-6))
- envelopes-a-6 (50) at `crates/before/tests/meter.rs:258` ([hunk](#hunk-6))
- envelopes-a-6 (50) at `crates/before/tests/meter.rs:302` ([hunk](#hunk-6))
- envelopes-a-6 (50) at `crates/before/tests/meter.rs:337` ([hunk](#hunk-6))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:339` ([hunk](#hunk-6))
- envelopes-a-11 (4) at `crates/before/tests/meter.rs:355` ([hunk](#hunk-6))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:356` ([hunk](#hunk-6))
- fresh-eyes-F1 (4) at `crates/before/tests/meter.rs:342` ([hunk](#hunk-6))
- fresh-eyes-F3 (88) at `crates/before/tests/meter.rs:284` ([hunk](#hunk-6))
- fresh-eyes-F3 (88) at `crates/before/tests/meter.rs:315` ([hunk](#hunk-6))
- fresh-eyes-F3 (88) at `crates/before/tests/meter.rs:348` ([hunk](#hunk-6))
- fresh-eyes-F7 (105) at `crates/before/tests/meter.rs:358` ([hunk](#hunk-6))
- fresh-eyes-F3 (105) at `crates/before/tests/meter.rs:265` ([hunk](#hunk-6))
- fresh-eyes-W4 (50) at `crates/before/tests/meter.rs:270` ([hunk](#hunk-6))
- fresh-eyes-taste (105) at `crates/before/tests/meter.rs:350` ([hunk](#hunk-6))
- fresh-eyes-W3 (88) at `crates/before/tests/meter.rs:281` ([hunk](#hunk-6))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:437` ([hunk](#hunk-7))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:454` ([hunk](#hunk-7))
- meter-adequacy-11 (108) at `crates/before/tests/meter.rs:458` ([hunk](#hunk-7))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:487` ([hunk](#hunk-7))
- crate-root-32 (38) at `crates/before/tests/meter.rs:487` ([hunk](#hunk-7))
- envelopes-a-6 (50) at `crates/before/tests/meter.rs:522` ([hunk](#hunk-7))
- meter-adequacy-11 (108) at `crates/before/tests/meter.rs:445` ([hunk](#hunk-7))
- meter-adequacy-11 (108) at `crates/before/tests/meter.rs:459` ([hunk](#hunk-7))
- fresh-eyes-F3 (88) at `crates/before/tests/meter.rs:523` ([hunk](#hunk-7))
- fresh-eyes-F6 (4) at `crates/before/tests/meter.rs:565` ([hunk](#hunk-7))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:630` ([hunk](#hunk-8))
- envelopes-a-8 (108) at `crates/before/tests/meter.rs:644` ([hunk](#hunk-8))
- envelopes-a-2 (38) at `crates/before/tests/meter.rs:656` ([hunk](#hunk-9))
- crate-root-32 (38) at `crates/before/tests/meter.rs:668` ([hunk](#hunk-10))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:672` ([hunk](#hunk-10))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:692` ([hunk](#hunk-11))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:712` ([hunk](#hunk-11))
- fresh-eyes-D2 (108) at `crates/before/tests/meter.rs:738` ([hunk](#hunk-11))
- fresh-eyes-D2 (108) at `crates/before/tests/meter.rs:841` ([hunk](#hunk-12))
- fresh-eyes-D2 (108) at `crates/before/tests/meter.rs:862` ([hunk](#hunk-13))
- fresh-eyes-D2 (108) at `crates/before/tests/meter.rs:882` ([hunk](#hunk-14))
- fresh-eyes-D2 (108) at `crates/before/tests/meter.rs:950` ([hunk](#hunk-15))
- fresh-eyes-D2 (108) at `crates/before/tests/meter.rs:957` ([hunk](#hunk-16))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:984` ([hunk](#hunk-17))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:996` ([hunk](#hunk-18))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:1012` ([hunk](#hunk-19))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:1066` ([hunk](#hunk-20))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:1106` ([hunk](#hunk-21))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:1145` ([hunk](#hunk-22))
- envelopes-a-8 (4) at `crates/before/tests/meter.rs:1159` ([hunk](#hunk-22))
- envelopes-b-8 (4) at `crates/before/tests/meter.rs:1289` ([hunk](#hunk-23))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1307` ([hunk](#hunk-23))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1323` ([hunk](#hunk-23))
- envelopes-a-8 (108) at `crates/before/tests/meter.rs:1292` ([hunk](#hunk-23))
- envelopes-a-8 (108) at `crates/before/tests/meter.rs:1316` ([hunk](#hunk-23))
- envelopes-a-8 (105) at `crates/before/tests/meter.rs:1300` ([hunk](#hunk-23))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1343` ([hunk](#hunk-24))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1372` ([hunk](#hunk-25))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1417` ([hunk](#hunk-26))
- envelopes-a-11 (4) at `crates/before/tests/meter.rs:1427` ([hunk](#hunk-27))
- envelopes-a-11 (4) at `crates/before/tests/meter.rs:1432` ([hunk](#hunk-27))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1447` ([hunk](#hunk-28))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1465` ([hunk](#hunk-29))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1481` ([hunk](#hunk-30))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1499` ([hunk](#hunk-31))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1515` ([hunk](#hunk-32))
- envelopes-a-2 (38) at `crates/before/tests/meter.rs:1595` ([hunk](#hunk-33))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1601` ([hunk](#hunk-33))
- envelopes-a-11 (4) at `crates/before/tests/meter.rs:1570` ([hunk](#hunk-33))
- meter-adequacy-11 (108) at `crates/before/tests/meter.rs:1533` ([hunk](#hunk-33))
- fresh-eyes-F5 (50) at `crates/before/tests/meter.rs:1539` ([hunk](#hunk-33))
- fresh-eyes-taste (105) at `crates/before/tests/meter.rs:1567` ([hunk](#hunk-33))
- envelopes-a-4 (108) at `crates/before/tests/meter.rs:1615` ([hunk](#hunk-33))
- envelopes-a-4 (108) at `crates/before/tests/meter.rs:1704` ([hunk](#hunk-33))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1727` ([hunk](#hunk-33))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1750` ([hunk](#hunk-33))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1770` ([hunk](#hunk-33))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:1792` ([hunk](#hunk-33))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:2116` ([hunk](#hunk-34))
- envelopes-a-2 (38) at `crates/before/tests/meter.rs:5550` ([hunk](#hunk-35))
- envelopes-a-2 (38) at `crates/before/tests/meter.rs:5576` ([hunk](#hunk-36))
- envelopes-a-2 (38) at `crates/before/tests/meter.rs:5690` ([hunk](#hunk-37))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:5839` ([hunk](#hunk-38))
- envelopes-b-8 (4) at `crates/before/tests/meter.rs:5848` ([hunk](#hunk-39))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:5866` ([hunk](#hunk-40))
- envelopes-b-8 (4) at `crates/before/tests/meter.rs:6169` ([hunk](#hunk-41))
- envelopes-b-8 (4) at `crates/before/tests/meter.rs:6184` ([hunk](#hunk-42))
- envelopes-a-2 (38) at `crates/before/tests/meter.rs:6289` ([hunk](#hunk-43))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6410` ([hunk](#hunk-43))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6416` ([hunk](#hunk-43))
- fresh-eyes-taste (105) at `crates/before/tests/meter.rs:6613` ([hunk](#hunk-43))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6777` ([hunk](#hunk-44))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6807` ([hunk](#hunk-45))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6832` ([hunk](#hunk-46))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6857` ([hunk](#hunk-47))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6882` ([hunk](#hunk-48))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6921` ([hunk](#hunk-49))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6959` ([hunk](#hunk-50))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:6999` ([hunk](#hunk-51))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:7212` ([hunk](#hunk-52))
- envelopes-a-4 (4) at `crates/before/tests/meter.rs:7234` ([hunk](#hunk-53))
- fresh-eyes-D2 (108) at `crates/before/tests/meter.rs:7277` ([hunk](#hunk-54))
- meter-adequacy-11 (108) at `justfile:111` ([hunk](#hunk-55))
- fresh-eyes-W1 (108) at `justfile:106` ([hunk](#hunk-55))
- fresh-eyes-W2 (108) at `justfile:98` ([hunk](#hunk-55))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-before/triage/annotations/p1-harness.tsv `@@ -0,0 +1,123 @@`

```diff
@@ -0,0 +1,123 @@
+crates/before/tests/meter.rs	1	envelopes-a-4	4	The file doc now states the pin convention once (ceiling ×1.25 rounded up and only tightened; the ×0.75 improvement tripwire; the re-denomination sanction that lived only in the rank table's preamble). The five table preambles that each restated it collapse to a one-sentence citation. The history-laden contract paragraph ("driving toward", "today's implementation is far from that") is restated as what the rows pin and where the asymptotic claims are judged (the flatness bands), per ruling 45's direction that this doc speak of the present.
+crates/before/tests/meter.rs	31	envelopes-a-2	38	The segments bullet and the "segment counts track per-target frame sizes" clause are gone; in their place the doc names `clock::tests::deep_tree_stack_safety` as the committed no-recursion proof and states that a traversal regressed to recursion overflows the stack on the deep scenarios here rather than moving a counter this binary cannot write.
+crates/before/tests/meter.rs	235	envelopes-b-8	4	One envelope type for every table: a heap ceiling plus one `Bound` per counter column (limb, touch, scan). I did not use `ByCurrency<Bound>` from the board: it carries a segments slot ruling 38 dissolves and a heap slot whose floor genre differs (the heap column has canaries, not a tripwire), so the ready-made shape would have needed two carve-outs.
+crates/before/tests/meter.rs	248	envelopes-a-4	4	The per-column pin as a ceiling and a floor, replacing four structs whose only difference was which columns existed.
+crates/before/tests/meter.rs	258	envelopes-a-6	50	The floor genre is typed, so a row states which floor it carries by constructor (`band` for the ×0.75 tripwire, `whole_input` for the derived one-bit-per-input-byte liveness floor) rather than in a comment the code cannot check.
+crates/before/tests/meter.rs	293	envelopes-a-4	4	The one constructor for a tripwire-floored column; the four `const fn` builders whose only job was the cfg underscore dance are gone.
+crates/before/tests/meter.rs	323	envelopes-a-4	4	The one `const fn` builder of an `Envelope`.
+crates/before/tests/meter.rs	437	envelopes-a-4	4	The static column table `metered` walks: each counter's MEASURED key, message unit, reset, read, and the row accessor. A column added here cannot be skipped by the harness, and the self-test below proves each column's asserts fire.
+crates/before/tests/meter.rs	454	envelopes-a-4	4	MEASURED-line order is `limb_ops touches scan_bits`, the order the parent's query harness printed, so the largest table's lines stay byte-identical whole and every other row's parent fragments stay in place.
+crates/before/tests/meter.rs	458	meter-adequacy-11	108	The column table's reset and read are the crate's counter functions; the cfg-gated wrapper functions that returned `None` without a feature are deleted, since the manifest guarantees the features.
+crates/before/tests/meter.rs	487	envelopes-a-4	4	The one harness: reset every column, run, read the heap peak first (before any allocation of the harness's own), then the counters, print one MEASURED line composed from fragments, assert the heap ceiling, then each column's ceiling and floor. The tripwire message exists once.
+crates/before/tests/meter.rs	553	envelopes-a-4	4	The unified harness's own negative control: a body that moves every counter must fail under a ceiling one below its reading and a floor one above it on each column, and under a zero heap ceiling. The worst artifact a harness refactor can produce is one whose asserts are silently skipped; this is the committed demonstration that none are. It prints `MEASURED harness_probe` lines, which a re-pinner ignores by name.
+crates/before/tests/meter.rs	336	envelopes-b-8	4	Rows converted mechanically from the parent's four constructors (rows-step1.json in the lane's scratchpad is the parent's numbers, checked against this table); the tick-row pointer comment went with the four-column table it described.
+crates/before/tests/meter.rs	1	envelopes-a-2	38	The segments column is gone from the first sentence, the column list, and the portability sentence; no binary that judges this file can write the counter (its one increment is inside `cfg(test)` code the integration binary does not link), so the ceiling judged nothing.
+crates/before/tests/meter.rs	487	crate-root-32	38	The harness no longer resets or reads `meter::stack_segments` and prints no `segments=` fragment; every other fragment of every MEASURED line is byte-identical to step 1's (measured-step2.log against measured-step1.log in the lane's scratchpad, `--drop segments`). The readers themselves are the board lane's to delete.
+crates/before/tests/meter.rs	668	crate-root-32	38	The clause "the recursion-frame cost: heap stays flat, segments do not" is excised (the entry names this line): the sweep is iterative, and the doc now says what the row prices, restated from its table comment per ruling 45.
+crates/before/tests/meter.rs	656	envelopes-a-2	38	"pinned peak-heap and stack-segment envelope (the parse-stack cost, linear in depth today)" named a column that no longer exists and a mechanism that is not the decoder's; restated from the row's table comment, "today" gone (ruling 45).
+crates/before/tests/meter.rs	1595	envelopes-a-2	38	Same sweep in the text-kernel section.
+crates/before/tests/meter.rs	5550	envelopes-a-2	38	The id-spine docs (`id_join`, `id_covers`, `id_without`, `id_fork`) each promised "must grow no stack segments"; the promise is now the file doc's, so the docs keep only what the row prices.
+crates/before/tests/meter.rs	6289	envelopes-a-2	38	`skyline_min_ticks_dense`'s "zero grown segments at 125k levels" restated as the depth alone.
+crates/before/tests/meter.rs	78	envelopes-b-8	4	Every row pins every column (ruling 4's second shape); the transitional `None` sentence is gone with the placeholder.
+crates/before/tests/meter.rs	239	envelopes-b-8	4	The column's type is `Bound`, not `Option<Bound>`: the placeholder lived only inside this lane's series.
+crates/before/tests/meter.rs	341	envelopes-b-8	4	The 66 newly pinned cells across the seven tables are each the unified harness's reading at the parent library (the library is untouched in this series), ceiling ×1.25 rounded up, floor ×0.75 rounded down; every cell with its reading is attributed in the step-3 commit message.
+crates/before/tests/meter.rs	258	envelopes-a-6	50	The two floor genres the file doc names, typed: a row states which it carries by constructor. `WholeInput` is 8 × input_bytes less one byte of padding per operand stream, not one bit per byte: the review's own witness (the unary-run scan tap removed) drops the dense validator's reading from 375006 to 125005 bits, which a per-byte floor (46_876) would not catch and this one does.
+crates/before/tests/meter.rs	302	envelopes-a-6	50	The derived floor's constructor; used only on rows whose `input_bytes` is the byte length of exactly the streams the walk reads (`cmp_cliff`, denominated in packed generator bytes, reads 0.05 scan bits per input byte and must keep the tripwire).
+crates/before/tests/meter.rs	522	envelopes-a-6	50	The harness computes the derived floor from the row's own `input_bytes` at run time; saturating so a tiny input cannot underflow.
+crates/before/tests/meter.rs	1533	envelopes-a-6	50	The negative control ruling 50 names, committed in the binary: the ruled reversible library mutation (stubbing `skyline::validate_bits`) was refused by this session's permission system, so the same known-bad bodies are judged against the `SKYLINE_VALIDATE_DENSE` row here. The stub trips the touch tripwire first (touches 0 under floor 2); a validator reading half the stream trips exactly the whole-input scan floor (187506 under 375000). Both messages are in the step-4 commit.
+crates/before/tests/meter.rs	337	envelopes-a-6	50	The three dense row comments now state the liveness signal their rows carry (a whole-input scan floor on decode; touch and scan tripwires on cmp and join), where the parent's `DECODE_DENSE` comment named floors its four-column table did not have.
+crates/before/tests/meter.rs	1601	envelopes-a-4	4	`skyline_render_records_zero_touches` folds into the `SKYLINE_RENDER_*` rows' touch cells (pinned 0/0 since step 3); its rationale, the conservation tripwire on the text seam, moves to the section that owns the rows. The test's wide-arming case had no envelope row and is not carried: reported in the lane report as a candidate row.
+crates/before/tests/meter.rs	6410	envelopes-a-4	4	The tick and multi-tick scenarios and the grow-branch section now sit beside `query_env`, the table their rows live in. The ticks flatness bands stay in the dense-spine section: they are the suites lane's and use no table.
+crates/before/tests/meter.rs	6416	envelopes-a-4	4	Restated from the row's comment while moving it; "linear in nodes today" is gone (ruling 45).
+crates/before/tests/meter.rs	339	envelopes-a-8	4	The kernel-only shapes ported to the public operations under the door roster: `CMP_DENSE_SELF` (`partial_cmp` on a byte-equal, buffer-distinct copy), `CMP_WIDE_TOOTH`, `JOIN_ABSORB`, `JOIN_WIDE_TOOTH`, `MEET_CLIFF`, `MEET_WIDE_TOOTH`. Every cell is the door's own reading, ceiling ×1.25, floor ×0.75, listed in the commit beside the retired twin's reading and ceiling.
+crates/before/tests/meter.rs	355	envelopes-a-11	4	`DECODE_WIDE_TOOTH` and `DECODE_ALT_SPINE` through `Version::decode`, so no shape loses its decode pin when the five `SKYLINE_DECODE_*` rows (a meter-only wrapper whose columns duplicated the validate rows) dissolve; their scan floors are whole-input like the other decode rows.
+crates/before/tests/meter.rs	356	envelopes-a-8	4	The Cargo.lock note travels with its row from the retired sweep table.
+crates/before/tests/meter.rs	630	envelopes-a-8	4	The byte-identity leg the retired join twins carried (kernel emission equals the door's stream) now lives on the door rows, computed outside measurement; the meet and absorb rows carry closed-form legs instead (the flat operand is the whole result) and the cmp rows the verdict.
+crates/before/tests/meter.rs	672	envelopes-a-8	4	The retired twin's verdict leg (`Some(Greater)`) replaces `consumed(r)`; likewise on `cmp_bigroot` and `cmp_cliff`. `cmp_bigroot`'s doc dropped "today the worst amplifier: ... quadratic", which described a replaced implementation (ruling 45).
+crates/before/tests/meter.rs	1570	envelopes-a-11	4	The round-trip equality the dissolved decoder rows carried stays as a plain unit test, since `skyline::decode` remains in the tests' vocabulary.
+crates/before/tests/meter.rs	1427	envelopes-a-11	4	The codec section no longer describes decoder rows that transcode into "the packed form", a coding the stored form no longer is.
+crates/before/tests/meter.rs	692	envelopes-a-8	4	The dense self-comparison ported from the kernel row to `partial_cmp` on a byte-equal, buffer-distinct copy (the door runs the full sweep; `PartialOrd` has no equality short-circuit). The twelve tick and multi-tick scenarios that followed the dense join here moved whole to the tick section beside `query_env`.
+crates/before/tests/meter.rs	984	envelopes-a-8	4	`cmp_bigroot`'s doc restated from the row's mechanism; the sentence "today the worst amplifier: per-frame owned path sums, quadratic in the root magnitude × depth" described a replaced implementation (ruling 45).
+crates/before/tests/meter.rs	996	envelopes-a-8	4	The retired `SKYLINE_CMP_BIGROOT` twin's verdict leg moves onto the door row in place of `consumed(r)`; its doc is restated from the row's mechanism, dropping the "today the worst amplifier" sentence about a replaced implementation.
+crates/before/tests/meter.rs	1012	envelopes-a-8	4	The retired `SKYLINE_JOIN_BIGROOT` twin's byte-identity leg (the public join's stream equals the emit kernel's) moves onto the door row in place of `drop(joined)`.
+crates/before/tests/meter.rs	1066	envelopes-a-8	4	`JOIN_ABSORB` ported to the public join of the dense spine with the flat hugeleaf operand; the closed-form leg (the join is the flat operand) is the retired kernel test's own assertion. Placed in the hugeleaf section, whose operand it shares.
+crates/before/tests/meter.rs	1106	envelopes-a-8	4	The retired `SKYLINE_CMP_CLIFF` twin's verdict leg on the door row, in place of `consumed(r)`.
+crates/before/tests/meter.rs	1145	envelopes-a-8	4	`join_cliff` gains the retired twin's byte-identity leg; `MEET_CLIFF` is ported to the public meet. My first closed-form leg (the meet is the one-tick leaf) was wrong: the comb has zero-height leaves, so the meet keeps the comb's shape clamped to height 1; the leg is the kernel byte-identity the retired twin carried, and the doc says what the meet is.
+crates/before/tests/meter.rs	1159	envelopes-a-8	4	The wide-tooth comb gets a section of its own for the four door rows ported here (`DECODE_WIDE_TOOTH`, `CMP_WIDE_TOOTH`, `JOIN_WIDE_TOOTH`, `MEET_WIDE_TOOTH`) and the alternating spine one for `DECODE_ALT_SPINE`; the kernel sections they came from are gone.
+crates/before/tests/meter.rs	1289	envelopes-b-8	4	`TouchEnvelope`, `touch_envelope`, the rank table's own restatement of the pin convention, and `touch_metered` are deleted; the table cites the file doc and its rows are the unified `envelope(...)` form with the same numbers. The re-denomination sanction that lived only here moved to the file doc.
+crates/before/tests/meter.rs	1307	envelopes-a-4	4	Call site renamed from `touch_metered` to `metered`; no reading moved. The same holds for every `metered(` call-site hunk in this file.
+crates/before/tests/meter.rs	1323	envelopes-a-4	4	Call site renamed from `touch_metered` to `metered`, reflowed by rustfmt; no reading moved.
+crates/before/tests/meter.rs	1343	envelopes-a-4	4	Call site renamed from `touch_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1372	envelopes-a-4	4	Call site renamed from `touch_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1417	envelopes-a-4	4	Call site renamed from `touch_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1432	envelopes-a-11	4	The codec section no longer describes decoder rows transcoding into "the packed form" (a coding the stored form is not); it states what the validate rows pin and that the public decode rows price the wrap.
+crates/before/tests/meter.rs	1447	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1465	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1481	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1499	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	2116	envelopes-a-4	4	Deletion: `skyline_render_records_zero_touches` stood here; the `SKYLINE_RENDER_*` rows' touch cells pin the zero it asserted, and the text-kernel section carries its rationale. The band module's other tests are untouched.
+crates/before/tests/meter.rs	5576	envelopes-a-2	38	`id_covers`'s doc promised "its iterative frames must grow no stack segments"; the promise is the file doc's now, so the doc keeps only the walk it prices.
+crates/before/tests/meter.rs	5690	envelopes-a-2	38	`id_without`'s doc promised "must grow no stack segments"; restated to what the row prices.
+crates/before/tests/meter.rs	5839	envelopes-a-4	4	The fork table's doc no longer restates the pin convention; the file doc holds it once.
+crates/before/tests/meter.rs	5848	envelopes-b-8	4	`ID_FORK` in the unified form, every column pinned (its touch cell measured 0, its scan cell keeps the raw-path pin of 3 over a tripwire of 1); `id_fork`'s doc drops "(zero segments)".
+crates/before/tests/meter.rs	5866	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	6169	envelopes-b-8	4	The query section no longer says these rows "carry all five columns" as the reason for a separate harness: every table carries every column now, so the sentence states only where the kernels' work lives.
+crates/before/tests/meter.rs	6184	envelopes-b-8	4	`QueryEnvelope`, `query_envelope`, the table's own preamble, and `query_metered` are deleted; the table cites the file doc, its rows keep their numbers in the unified form (scan floors added in step 4), and the two inner comments that restated the convention or the tables' history are shortened to what the rows are.
+crates/before/tests/meter.rs	6777	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	6807	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	6832	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	6857	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	6882	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	6921	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	6959	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	6999	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved, and the row's 480 B heap pin is untouched (ruling 24).
+crates/before/tests/meter.rs	7212	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	7234	envelopes-a-4	4	Call site renamed from `query_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1515	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	47	meter-adequacy-11	108	The suite builds only with its meter features (`required-features` on the `meter` test target in the crate manifest, the `amp_board` idiom), so the doc no longer describes a configuration with columns compiled out.
+crates/before/tests/meter.rs	445	meter-adequacy-11	108	With the features guaranteed by the manifest, a column's read is the counter's own function and the `Option` plumbing, the six cfg-gated wrapper functions, and the harness's and self-test's `None` skips are deleted.
+crates/before/tests/meter.rs	459	meter-adequacy-11	108	The column table names the crate's counter functions directly.
+crates/before/tests/meter.rs	1533	meter-adequacy-11	108	The probe's `cfg(feature = "scan-meter")` gate is gone with the guard: the feature is always on.
+crates/before/tests/meter.rs	1292	envelopes-a-8	108	`SKYLINE_RANK_{DENSE,BIGROOT,HARMONIC}` retire as twins of the public `RANK_*` rows (identical readings on every column; `Version::rank` is the kernel on the live skyline). `RANK_BIGROOT`'s heap ceiling carries the twin's tighter 67_145 (reading 57_604) in place of 72_005, the rule the ported door rows follow; the other cells were already equal.
+crates/before/tests/meter.rs	644	envelopes-a-8	108	The retired twins' identity leg (kernel rank equals the public rank) moves onto the door rows in place of `consumed(r)`, computed outside measurement, as the join rows' `emitted` leg does.
+crates/before/tests/meter.rs	1316	envelopes-a-8	108	The door docs absorb the retired kernel docs' mechanism sentences.
+crates/before/tests/meter.rs	342	fresh-eyes-F1	4	The retired `SKYLINE_CMP_BIGROOT` twin's tighter heap ceiling (39_540) carries onto the door row in place of 40_340; both rows read 32_272 B, so the tightest committed number on this shape and operation does not go up when the twin goes.
+crates/before/tests/meter.rs	284	fresh-eyes-F3	88	The derived floor now states its premise and carries a per-stream tail allowance: validators, decoders, and the id walks whose output depends on every tag read to the end (tail 0); the diverted pair's covers and disjoint walks are decided at the last unary node's tag pair and may leave each stream's 2-bit terminal unread, so their floor allows a 4-bit tail per stream (999_992 bits against the 1_000_004 reading) and an early exit at the decision is an improvement the floor admits, per ruling 88.
+crates/before/tests/meter.rs	315	fresh-eyes-F3	88	The decision-point constructor; `whole_input` keeps tail 0.
+crates/before/tests/meter.rs	523	fresh-eyes-F3	88	The harness subtracts the padding and the tail per stream.
+crates/before/tests/meter.rs	348	fresh-eyes-F3	88	`ID_COVERS` and `ID_DISJOINT` move to `to_decision(…, 2, 4)`; ceilings unchanged.
+crates/before/tests/meter.rs	1539	fresh-eyes-F5	50	Both known-bad validators are judged against the row with its limb and touch bands opened, so the scan floor is the assert that fires for each (the stub had tripped the touch tripwire first); both messages must name the whole-input liveness floor.
+crates/before/tests/meter.rs	565	fresh-eyes-F6	4	The self-test reads the counters and the heap peak before `consumed` formats the probe's result, so a `Debug` impl that did metered work could not fail the test for a reason unrelated to the harness.
+crates/before/tests/meter.rs	69	fresh-eyes-F2	105	The floor's premise is attributed correctly: contract for validators and decoders; construction for the id walks, naming `ID_WITHOUT` (not on the diverted pair) and the pair rows' decision point, so a reader re-derives the right premise.
+crates/before/tests/meter.rs	33	fresh-eyes-F4	105	The no-recursion attribution states the actual coverage: what the named test drives at 100k (read from its body: it also meets two distinct deep versions and checks `is_disjoint`), which rows here run at overflow depth, and which operations have no deep-input witness anywhere (carried to the p2-rows lane by the coordinator).
+crates/before/tests/meter.rs	88	fresh-eyes-F7	105	The doc claims only what is shown: determinism on a target; portability is CI's second-target run to check (this lane measured on x86_64 illumos only).
+crates/before/tests/meter.rs	358	fresh-eyes-F7	105	The hand-maintained dependency version string leaves the prose; the note says what the pin depends on.
+crates/before/tests/meter.rs	1567	fresh-eyes-taste	105	The doc no longer enumerates the shapes the loop already lists.
+crates/before/tests/meter.rs	6613	fresh-eyes-taste	105	The moved tick doc names the constant instead of restating its value.
+crates/before/tests/meter.rs	265	fresh-eyes-F3	105	Summary split after its first sentence for doclint's limit.
+crates/before/tests/meter.rs	1300	envelopes-a-8	105	Summary split after its first sentence for doclint's limit; likewise the bigroot rank doc.
+crates/before/tests/meter.rs	1615	envelopes-a-4	108	The render side of the wide-arming dual, which the folded test had covered and no row did: `Shape::WideArming.packed2(512, 512)` rendered through the text kernel, every cell pinned from the parent library's reading (in the commit message), the touch cell at zero.
+crates/before/tests/meter.rs	1704	envelopes-a-4	108	The scenario beside the other render rows; the scale constant sits with the scenario sizes.
+crates/before/Cargo.toml	133	meter-adequacy-11	108	Ruling 108's mechanism: `required-features = ["limb-meter", "scan-meter"]` on the `meter` test target. An explicit `--test meter` without the features is a cargo error naming them; blanket `check`, `clippy --tests`, and `nextest --workspace` runs skip the target, so `just check` and the gate's default-feature clippy leg stay green, which ruling 51's `compile_error!` broke.
+justfile	111	meter-adequacy-11	108	The inner loop passes the meter features for the `before` package (the `package/feature` form, since the recipe is `--workspace`), so `just test` runs the suite the manifest would otherwise skip; the comment says why.
+crates/before/tests/meter.rs	215	envelopes-a-4	108	The wide-arming render row's scale, with the other scenario sizes: the parse band's small point, the shape the folded test rendered.
+crates/before/tests/meter.rs	712	envelopes-a-4	4	Deletion: the tick and multi-tick scenarios stood here, between the ticks flatness bands; they moved whole to the tick section beside `query_env`. The bands themselves are untouched.
+crates/before/tests/meter.rs	1727	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1750	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1770	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	1792	envelopes-a-4	4	Call site renamed from `sweep_metered` to `metered`; no reading moved.
+crates/before/tests/meter.rs	32	fresh-eyes-D1	105	The no-recursion attribution names all three stack-safety tests and what each drives (read from their bodies); the claim that project, distance, lag, and the masked comparisons had no deep witness was false: the query-and-causal test drives them through a 100k-deep id and the quotient view.
+crates/before/tests/meter.rs	270	fresh-eyes-W4	50	The floor's doc says what it proves on which rows: the scan counter also holds writes and splices, so on `ID_JOIN` (2_500_018 counted against 1_000_004 input bits) and `ID_WITHOUT` (2_000_004 against 500_002) the floor attests only total bypass, while on the read-only validator, decoder, covers, and disjoint rows it attests the reads; the file doc's genre bullet says the same in one clause.
+crates/before/tests/meter.rs	526	fresh-eyes-taste	105	The floor's failure message no longer says "whole-input" for a `to_decision` floor; the negative control's two greps match the new wording.
+crates/before/tests/meter.rs	350	fresh-eyes-taste	105	The row says its `input_bytes` omits the seed's two-bit stream, so the floor is conservative.
+justfile	106	fresh-eyes-W1	108	The recipe comment states what the invocation builds: feature unification lights both meters on the before lib for every dependent, so every before suite gated on them joins `just test`.
+justfile	98	fresh-eyes-W2	108	`just check` skips a required-features target silently; the comment says where the meter suite is type-checked instead.
+crates/before/tests/meter.rs	154	fresh-eyes-D2	108	With the features required by the manifest the gate could never be false, and its doc sentence contradicted the file doc's "every column is always compiled in"; both go.
+crates/before/tests/meter.rs	738	fresh-eyes-D2	108	The twelve `cfg(all(limb-meter, scan-meter))` gates (eleven on the ticks flatness bands, their probes, and their constants; one on the `fold_stagger` band module) are deleted for the same reason. The band modules' single-feature `cfg(limb-meter)` gates are equally dead; they are the suites lane's.
+crates/before/tests/meter.rs	281	fresh-eyes-W3	88	The tail matches the derivation: the pair is decided at the last unary node's tag pair, and only the 2-bit terminal lies below it, so the floor admits an exit at the decision (999_996 against the 1_000_004 reading) and rejects a walk that skipped the deciding tags. The walk in `party/ops/compare.rs` reads the tags it decides on and the terminals only when it does not exit, which the derivation matches.
+crates/before/tests/meter.rs	841	fresh-eyes-D2	108	Deletion: an always-true `cfg(all(limb-meter, scan-meter))` gate stood above this probe.
+crates/before/tests/meter.rs	862	fresh-eyes-D2	108	Deletion: the always-true gates above this constant and the one before it.
+crates/before/tests/meter.rs	882	fresh-eyes-D2	108	Deletion: the always-true gate above this band.
+crates/before/tests/meter.rs	950	fresh-eyes-D2	108	Deletion: the always-true gate above this band.
+crates/before/tests/meter.rs	957	fresh-eyes-D2	108	Deletion: the always-true gates above the flatness pin's point and band constants.
+crates/before/tests/meter.rs	7277	fresh-eyes-D2	108	Deletion: the twelfth always-true `cfg(all(...))` gate, on this band module.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### crates/before/Cargo.toml `@@ -124,6 +124,16 @@ required-features = ["limb-meter", "scan-meter"]`

```diff
@@ -124,6 +124,16 @@ required-features = ["limb-meter", "scan-meter"]
 name = "code_study"
 required-features = ["meter"]
 
+# The envelope suite pins limb, touch, and scan columns; built without the
+# counter features every one of those cells would judge nothing. Requiring
+# the features makes an explicit `--test meter` without them a loud cargo
+# error naming them, while blanket `check`, `clippy --tests`, and
+# `nextest --workspace` runs skip the target; `just test` passes the
+# features for the package so the inner loop runs the suite.
+[[test]]
+name = "meter"
+required-features = ["limb-meter", "scan-meter"]
+
 [[bench]]
 name = "party"
 harness = false
```

<!-- annotation -->
> **meter-adequacy-11** (108), line 133:
>
> Ruling 108's mechanism: `required-features = ["limb-meter", "scan-meter"]` on the `meter` test target. An explicit `--test meter` without the features is a cargo error naming them; blanket `check`, `clippy --tests`, and `nextest --workspace` runs skip the target, so `just check` and the gate's default-feature clippy leg stay green, which ruling 51's `compile_error!` broke.

<a id="hunk-3"></a>
### crates/before/tests/meter.rs `@@ -1,72 +1,96 @@`

```diff
@@ -1,72 +1,96 @@
-//! Resource envelopes: peak transient heap, grown stack segments, and
-//! big-integer limb work per operation on the adversarial input families.
+//! Resource envelopes: each operation's peak transient heap, big-integer
+//! limb work, accumulator digit touches, and packed-stream bits scanned on
+//! the adversarial input families, pinned as ceilings.
 //!
-//! The contract this suite is driving toward: no operation materializes
-//! transient state asymptotically larger than its packed operands, and every
-//! operation is amortized O(n + m) in the packed input bits — with no bound
-//! on value magnitude, tree depth, or encoded size. Today's implementation
-//! is far from that — several operations amplify their input by large
-//! constants or worse — so every scenario here pins the *current* measured
-//! cost, with ×1.25 slack, as a ceiling. A regression fails loudly now; each
-//! improvement tightens a committed number.
+//! A regression fails a pinned row loudly; an improvement tightens a
+//! committed number. The rows pin constants on fixed shapes; the flatness
+//! bands beside them hold each cost flat per unit across a scale doubling,
+//! which is where the asymptotic claims are judged.
 //!
-//! Three deterministic meters, asserted together per scenario:
+//! # The columns
+//!
+//! Deterministic meters, read over the scenario body alone and asserted
+//! together by [`metered`]:
 //!
 //! - **Peak heap bytes**: the binary-wide counting allocator
-//!   ([`PeakAlloc`]), read as a delta over the scenario body. One global
-//!   allocator exists per test binary, and the counters are process-global,
-//!   so per-scenario peaks are meaningful **only under nextest's
-//!   process-per-test isolation** — this workspace's runner. Under a runner
-//!   that shares one process across tests, concurrent allocation would bleed
-//!   between scenarios.
-//! - **Grown stack segments** ([`meter::stack_segments`]): the deep
-//!   traversals grow the stack onto the heap in fixed-size segments that
-//!   bypass any allocator meter; the segment counter is the honest stand-in
-//!   for recursion-driven stack cost. Process-global, same isolation
-//!   requirement.
-//! - **Big-integer limb operations** ([`meter::limb_ops`], only when the
-//!   `limb-meter` feature compiles the counter into the arithmetic):
-//!   operand limbs per `Base` operation plus one value-width record per
-//!   decoded wide-gamma value. Arithmetic-width cost is invisible to the
-//!   other two meters — the work is wider, not more frequent — so this is
-//!   the only column that sees a magnitude-quadratic regression. Without
-//!   the feature the scenarios still run and assert the other two columns.
+//!   ([`PeakAlloc`]), read as a delta over the scenario body. The canaries
+//!   below prove the meter live.
+//! - **Limb operations** ([`meter::limb_ops`], under `limb-meter`): operand
+//!   limbs per `Base` operation plus one value-width record per decoded
+//!   wide-gamma value. Arithmetic-width cost is invisible to the other
+//!   meters (the work is wider, not more frequent), so this is the column
+//!   that sees a magnitude-quadratic regression.
+//! - **Accumulator digit touches** (`suanpan::touch_meter`, under
+//!   `limb-meter`): the cliff-free accumulator's own currency, where the
+//!   folds and the tick walk do their arithmetic.
+//! - **Scanned bits** ([`meter::scan_bits`], under `scan-meter`):
+//!   packed-stream bits read and written through the metered primitives,
+//!   the column that sees traversal work that allocates nothing, recurses
+//!   nothing, and does no arithmetic.
+//!
+//! No column meters stack depth: library traversals are iterative by
+//! construction, and three tests in `clock::tests` prove it at depth 100k.
+//! `deep_tree_stack_safety` drives tick, fork, join, meet, the
+//! comparisons, encode, decode, send/recv, sync, `is_disjoint`, and
+//! Debug; `deep_tree_query_and_causal_stack_safety` drives rank,
+//! distance, lag, `Ranked` ordering, the span hulls, algebra, and
+//! quotient view (the masked co-walks), query membership and coverage,
+//! and projection through a deep id; and
+//! `deep_tree_text_and_min_ticks_stack_safety` drives render, parse, and
+//! `min_ticks`. The rows here on 125k-level spines and 250k-level id
+//! spines overflow the stack on a traversal regressed to recursion
+//! instead of moving a number.
+//!
+//! The counters are process-global, so per-scenario readings are meaningful
+//! only under nextest's process-per-test isolation, this workspace's
+//! runner. The binary builds only with the `limb-meter` and `scan-meter`
+//! features (`required-features` in the crate manifest), so every column
+//! is always compiled in.
+//!
+//! # The pin convention
 //!
-//! Every row whose measured limb count is nonzero also carries a limb
-//! lower bound — the measured value ×0.75, rounded down, a column in the
-//! same tables as the ceilings. Two lower-bound genres appear in this
-//! suite, named apart because their trips mean opposite things: a derived
-//! *liveness floor* states a mechanism's irreducible work, never a
-//! measured basis — an honest improvement can approach but never cross
-//! it, so a trip means the work left the metered representation
-//! (investigate the meter) — while a measured-×0.75 *improvement
-//! tripwire*, the envelope columns' genre, bands the pinned reading — a
-//! trip means the reading dropped more than 25% below the pin: attribute
-//! it, and an honest improvement re-pins the band while a dead meter is
-//! the bypass the column exists to catch. A limb ceiling passes vacuously
-//! when the counter stops counting (a meter hook deleted from one `Base`
-//! operation reads a near-zero column with every ceiling green), and the
-//! tripwire is what fails instead. Like the board's floors, these detect
-//! *total* bypass, not partial rerouting: an implementation that routes
-//! some width-scale work through metered operations and the rest around
-//! them still reads green, so the column is a bypass tripwire, never a
-//! full-liveness proof.
+//! A ceiling is the measured reading ×1.25, rounded up, and only ever
+//! tightened: where a re-measure rises while staying inside the ceiling
+//! (heap cells whose backend growth headroom varies), the tighter ceiling
+//! stands. A re-denomination of a column, the same work newly counted at a
+//! metered seam, is a sanctioned rise, recorded in its pin commit. Every
+//! row pins every column, and under each counter column sits a floor of
+//! one of two genres, named apart because their trips mean opposite
+//! things:
 //!
-//! Wall time is deliberately never asserted *in this suite*: it is the one
-//! number here that is not deterministic (the bench judge fits the time
-//! *exponent* over criterion medians across two bench scales — see
-//! `meter::board`'s module docs and `tools/benchjudge`; its wide-display
-//! pair judges the conversion class no counter column can see — that is
-//! the wall leg of record). The envelope constants are **measured** on the
-//! development target (aarch64-apple-darwin, dev profile); heap byte counts
-//! and limb counts are deterministic and portable across 64-bit targets
-//! (limb counts shrink under release, where `debug_assert!` comparisons
-//! vanish, so the dev-profile pin is the binding one), while segment counts
-//! track per-target frame sizes, and the slack absorbs modest variation.
+//! - An **improvement tripwire** ([`Floor::Tripwire`]) is the measured
+//!   reading ×0.75, rounded down. A trip is a drop of more than 25% from
+//!   the pinned reading: attribute it. An improvement re-pins the band; a
+//!   dead meter is the bypass the column exists to catch, since a ceiling
+//!   passes vacuously once a counter stops counting.
+//! - A **liveness floor** ([`Floor::LiveBits`]) states a mechanism's
+//!   irreducible work, never a measured basis: a walk that must read every
+//!   live input bit, by contract (a strict validator, a decoder) or by
+//!   construction (`ID_JOIN` and `ID_WITHOUT`, whose output depends on
+//!   every tag; `ID_COVERS` and `ID_DISJOINT`, whose diverted pair is
+//!   decided only at the last unary node), scans each at least once, less
+//!   the tail the floor's definition allows past a decision. An improvement
+//!   can approach but never cross it, so a trip means the work left the
+//!   metered primitives; on a row whose walk also writes, the counter
+//!   holds those writes too, and the floor attests total bypass alone.
+//!
+//! Both genres detect total bypass, not partial rerouting: work routed
+//! around the metered primitives in part still reads green.
+//!
+//! The measurements of record, and every re-pin's movement and
+//! attribution, live in the pin commits (`git log -S` the constant). Re-pin
+//! by rerunning this binary under `--no-capture` with `--all-features` and
+//! reading the MEASURED lines.
+//!
+//! Wall time is never asserted here: it is the one number that is not
+//! deterministic. The pins are dev-profile (limb counts shrink under
+//! release, where `debug_assert!` comparisons vanish, so the dev pin
+//! binds). Every column's reading is deterministic on a given target; a
+//! pin's portability is what CI's run on a second target checks.
 
 use before::meter::registry::Shape;
 use std::cmp::Ordering;
-use std::fmt::Debug;
+use std::fmt::{Debug, Write as _};
 
 use before::{meter, Party, Version};
 use peak_alloc::PeakAlloc;
```

<!-- annotation -->
> **envelopes-a-4** (4), line 1:
>
> The file doc now states the pin convention once (ceiling ×1.25 rounded up and only tightened; the ×0.75 improvement tripwire; the re-denomination sanction that lived only in the rank table's preamble). The five table preambles that each restated it collapse to a one-sentence citation. The history-laden contract paragraph ("driving toward", "today's implementation is far from that") is restated as what the rows pin and where the asymptotic claims are judged (the flatness bands), per ruling 45's direction that this doc speak of the present.

<!-- annotation -->
> **envelopes-a-2** (38), line 31:
>
> The segments bullet and the "segment counts track per-target frame sizes" clause are gone; in their place the doc names `clock::tests::deep_tree_stack_safety` as the committed no-recursion proof and states that a traversal regressed to recursion overflows the stack on the deep scenarios here rather than moving a counter this binary cannot write.

<!-- annotation -->
> **envelopes-a-2** (38), line 1:
>
> The segments column is gone from the first sentence, the column list, and the portability sentence; no binary that judges this file can write the counter (its one increment is inside `cfg(test)` code the integration binary does not link), so the ceiling judged nothing.

<!-- annotation -->
> **envelopes-b-8** (4), line 78:
>
> Every row pins every column (ruling 4's second shape); the transitional `None` sentence is gone with the placeholder.

<!-- annotation -->
> **meter-adequacy-11** (108), line 47:
>
> The suite builds only with its meter features (`required-features` on the `meter` test target in the crate manifest, the `amp_board` idiom), so the doc no longer describes a configuration with columns compiled out.

<!-- annotation -->
> **fresh-eyes-F2** (105), line 69:
>
> The floor's premise is attributed correctly: contract for validators and decoders; construction for the id walks, naming `ID_WITHOUT` (not on the diverted pair) and the pair rows' decision point, so a reader re-derives the right premise.

<!-- annotation -->
> **fresh-eyes-F4** (105), line 33:
>
> The no-recursion attribution states the actual coverage: what the named test drives at 100k (read from its body: it also meets two distinct deep versions and checks `is_disjoint`), which rows here run at overflow depth, and which operations have no deep-input witness anywhere (carried to the p2-rows lane by the coordinator).

<!-- annotation -->
> **fresh-eyes-F7** (105), line 88:
>
> The doc claims only what is shown: determinism on a target; portability is CI's second-target run to check (this lane measured on x86_64 illumos only).

<!-- annotation -->
> **fresh-eyes-D1** (105), line 32:
>
> The no-recursion attribution names all three stack-safety tests and what each drives (read from their bodies); the claim that project, distance, lag, and the masked comparisons had no deep witness was false: the query-and-causal test drives them through a 100k-deep id and the quotient view.

<a id="hunk-4"></a>
### crates/before/tests/meter.rs `@@ -127,10 +151,6 @@ const SCAN_HOLE_STEPS: usize = 128;`

```diff
@@ -127,10 +151,6 @@ const SCAN_HOLE_STEPS: usize = 128;
 
 /// Spine-depth pair of the masked-hole scenarios: the depth band holds
 /// the fused comparison's accumulator readings flat across this doubling.
-///
-/// The band needs the touch meter, so the smaller point is read only
-/// when the `limb-meter` feature compiles it in.
-#[cfg(feature = "limb-meter")]
 const MASK_HOLE_DEPTH_LO: usize = 1_000;
 
 /// The masked-hole depth pair's larger point (the envelope row's scale).
```

<!-- annotation -->
> **fresh-eyes-D2** (108), line 154:
>
> With the features required by the manifest the gate could never be false, and its doc sentence contradicted the file doc's "every column is always compiled in"; both go.

<a id="hunk-5"></a>
### crates/before/tests/meter.rs `@@ -190,6 +210,10 @@ const MASK_DRIFT_MAGNITUDE_BITS: usize = 512;`

```diff
@@ -190,6 +210,10 @@ const MASK_DRIFT_MAGNITUDE_BITS: usize = 512;
 /// Tooth count of the mask-drift families' envelope scenarios.
 const MASK_DRIFT_TEETH: usize = 1_024;
 
+/// Scale (magnitude bits and trail length) of the wide-arming render
+/// scenario: the parse direction's band shape at its small point.
+const WIDE_ARMING_RENDER_SCALE: usize = 512;
+
 /// Depth of the harmonic spine `H(d)` rank scenario: deep enough that the
 /// fold's per-level numerator re-shifts dominate every constant.
 const RANK_HARMONIC_DEPTH: usize = 65_536;
```

<!-- annotation -->
> **envelopes-a-4** (108), line 215:
>
> The wide-arming render row's scale, with the other scenario sizes: the parse band's small point, the shape the folded test rendered.

<a id="hunk-6"></a>
### crates/before/tests/meter.rs `@@ -205,99 +229,152 @@ const RANK_SUM_EXP_DEPTH: usize = 250_000;`

```diff
@@ -205,99 +229,152 @@ const RANK_SUM_EXP_DEPTH: usize = 250_000;
 
 // ─── pinned envelopes ───────────────────────────────────────────────────────
 
-/// One scenario's pinned ceilings (the measured value ×1.25, rounded up)
-/// and its limb improvement tripwire (measured ×0.75, rounded down — the
-/// file doc's tripwire genre).
+/// One scenario's pins: a peak-heap ceiling and one [`Bound`] per counter
+/// column, per the file doc's convention.
+#[derive(Clone, Copy)]
 struct Envelope {
     /// Peak heap delta over the scenario body, in bytes.
     peak_heap: usize,
-    /// Stack segments grown during the scenario body.
-    segments: u64,
-    /// Big-integer limb operations counted during the scenario body.
-    #[cfg(feature = "limb-meter")]
-    limb_ops: u64,
-    /// Improvement tripwire under the limb column.
-    ///
-    /// A reading below it is a drop of more than 25% from the pinned
-    /// reading — attribute it, re-pinning an honest improvement or curing a
-    /// dead meter (zero where the measured count is zero, under which the
-    /// bound asserts nothing).
-    #[cfg(feature = "limb-meter")]
-    limb_floor: u64,
+    /// Big-integer limb operations.
+    limb: Bound,
+    /// Accumulator digit touches.
+    touch: Bound,
+    /// Packed-stream bits scanned.
+    scan: Bound,
+}
+
+/// One counter column's pin: its ceiling and the floor under it.
+#[derive(Clone, Copy)]
+struct Bound {
+    /// The measured reading ×1.25, rounded up; only ever tightened.
+    ceiling: u64,
+    /// The floor under the reading.
+    floor: Floor,
+}
+
+/// The floor under a counter column's reading: the file doc's two genres,
+/// whose trips mean opposite things.
+#[derive(Clone, Copy)]
+enum Floor {
+    /// The improvement tripwire: the measured reading ×0.75, rounded down
+    /// (zero where the reading is zero, under which it asserts nothing). A
+    /// trip is a drop of more than 25% from the pinned reading: attribute
+    /// it.
+    Tripwire(u64),
+    /// The liveness floor of a walk that must read every live input bit,
+    /// less a stated tail.
+    ///
+    /// The floor is `8 × input_bytes` minus, per operand stream, at most
+    /// one byte of padding and `tail_bits` the walk may leave unread once
+    /// its verdict is decided. A trip means the work left the metered
+    /// primitives. The scan counter counts the builder's writes and splices
+    /// as well as reads, so on a row whose walk also writes (`ID_JOIN`,
+    /// `ID_WITHOUT`) the floor attests total bypass, not that the reads
+    /// stayed metered; on a read-only row (the validators, the decoders,
+    /// `ID_COVERS`, `ID_DISJOINT`) it attests the reads. Only for a row
+    /// whose `input_bytes` is the byte length of exactly the streams the
+    /// walk reads, and whose walk must read them by contract (a strict
+    /// validator or decoder: `tail_bits` 0), by construction (an id walk
+    /// whose output depends on every tag: `tail_bits` 0), or to a decision
+    /// point (the diverted id pair, decided at the last unary node's tag
+    /// pair, leaving exactly each stream's 2-bit terminal below it:
+    /// `tail_bits` 2, so an early exit at the decision is an improvement
+    /// the floor admits, while a walk that skipped the deciding tag pair
+    /// is not).
+    LiveBits {
+        /// The operand streams `input_bytes` counts.
+        streams: u64,
+        /// Bits per stream the walk may leave unread.
+        tail_bits: u64,
+    },
+}
+
+/// A column pinned at `ceiling` with its improvement tripwire at `floor`.
+const fn band(ceiling: u64, floor: u64) -> Bound {
+    Bound {
+        ceiling,
+        floor: Floor::Tripwire(floor),
+    }
+}
+
+/// A scan column pinned at `ceiling` over the liveness floor of a walk that
+/// reads `streams` operand streams whole.
+const fn whole_input(ceiling: u64, streams: u64) -> Bound {
+    Bound {
+        ceiling,
+        floor: Floor::LiveBits {
+            streams,
+            tail_bits: 0,
+        },
+    }
 }
 
-/// Build an [`Envelope`] from the three pinned columns and the limb floor.
-///
-/// The limb columns are carried only when the `limb-meter` feature
-/// compiles the counter into the arithmetic; the leading underscores keep
-/// the parameters warning-free in the other configuration.
-const fn envelope(peak_heap: usize, segments: u64, _limb_ops: u64, _limb_floor: u64) -> Envelope {
+/// A scan column pinned at `ceiling` over the liveness floor of a walk that
+/// reads `streams` operand streams to a decision point, leaving at most
+/// `tail_bits` of each unread.
+const fn to_decision(ceiling: u64, streams: u64, tail_bits: u64) -> Bound {
+    Bound {
+        ceiling,
+        floor: Floor::LiveBits { streams, tail_bits },
+    }
+}
+
+/// Build an [`Envelope`] from a row's columns.
+const fn envelope(peak_heap: usize, limb: Bound, touch: Bound, scan: Bound) -> Envelope {
     Envelope {
         peak_heap,
-        segments,
-        #[cfg(feature = "limb-meter")]
-        limb_ops: _limb_ops,
-        #[cfg(feature = "limb-meter")]
-        limb_floor: _limb_floor,
-    }
-}
-
-// The envelope table: pinned ceiling = measured ×1.25, rounded up
-// (aarch64-apple-darwin, dev profile, three identical runs), and only ever
-// tightened: where a remeasure rises while staying inside an existing
-// ceiling (the spilled-magnitude heap cells, which carry the backend's
-// `len/8 + 2` words of growth headroom per heap allocation), the older,
-// tighter ceiling stands. The trailing comment on each line states the
-// mechanism that prices the row; the measurements of record — and every
-// re-pin's movement and attribution — live in the pin commits (`git log
-// -S` the constant), never in this prose. Re-pin by rerunning this binary
-// under `--no-capture` and reading the MEASURED lines (the limb column
-// needs `--all-features` or `--features limb-meter`).
-// The limb floor column is the measured value ×0.75, rounded down (the
-// file doc's improvement-tripwire convention).
+        limb,
+        touch,
+        scan,
+    }
+}
+
+// Pins per the file doc's convention; each row's trailing comment states
+// the mechanism that prices it.
 #[rustfmt::skip]
 mod envelope {
-    use super::{envelope, sweep_envelope, Envelope, SweepEnvelope};
-    //                                              peak heap,  segments, limb ops, limb floor
-    pub const DECODE_DENSE: Envelope = envelope(120_035, 0, 0, 0); // wire decode is validate + wrap on the skyline kernels; decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
-    pub const CMP_DENSE: Envelope = envelope(30_720, 0, 0, 0); // the iterative sweep over the Bytes-backed at-rest form (OpenedPair states the pair walk's opening move once); word-valued payloads keep the limb column at zero
-    pub const JOIN_DENSE: Envelope = envelope(130_277, 0, 0, 0); // the emit kernel's peak alone: the value-operator cell's lhs clone is a refcount bump, not a byte copy of the operand; word-valued payloads keep the limb column at zero
-    // The tick rows live in `query_env`: the tick walk's cost currency
-    // is accumulator digit touches (with scanned bits beside it), which
-    // this four-column table never watched.
-    pub const DECODE_BIGROOT: Envelope = envelope(60_090, 0, 783, 469); // wire decode is validate + wrap; the one wide root magnitude keeps a linear limb record while the word-valued form carries the narrow codes
-    pub const CMP_BIGROOT: Envelope = envelope(40_340, 0, 783, 469); // the iterative sweep over the Bytes-backed at-rest form; the wide root's decode is the limb record
-    pub const JOIN_BIGROOT: Envelope = envelope(85_060, 0, 1_565, 939); // the emit kernel's peak alone (the lhs clone is a refcount bump); the wide root decodes on both sides carry the limb record
-    pub const DECODE_HUGELEAF: Envelope = envelope(   122_504,        0,         2_443, 1_465); // the validating wire decode holds the running height; one wide gamma code's linear limb work
-    pub const JOIN_HUGELEAF: Envelope   = envelope(   185_494,        0,         4_887, 2_931); // the emit kernel holds both payload buffers, and the lhs clone is a refcount bump, so the public join's peak is the emit kernel's alone
-    pub const ID_JOIN: Envelope         = envelope(   279_132,        0,             0, 0); // iterative id walks: frame bits on the heap, no grown segments
-    pub const ID_COVERS: Envelope       = envelope(        10,        0,             0, 0); // iterative id walks
-    pub const ID_DISJOINT: Envelope     = envelope(        10,        0,             0, 0); // iterative id walks
-    pub const ID_WITHOUT: Envelope      = envelope(   521_110,        0,             0, 0); // iterative complement over the Bytes-backed at-rest form; dev builds run no shadow re-parse of the diff emission (the differential suites carry the normal-form check)
-    pub const DECODE_CLIFF: Envelope = envelope(4_052, 0, 88, 52); // wire decode is validate + wrap; each cliff crossing's limb work is paid by its own wide stored code
-    pub const CMP_CLIFF: Envelope = envelope(1_330, 0, 88, 52); // the cliff-free sweep (two accumulators, opened once) over the Bytes-backed at-rest form
-    pub const JOIN_CLIFF: Envelope = envelope(5_362, 0, 308, 184); // the emit kernel's peak alone (the lhs clone is a refcount bump); each re-coded tooth's limb work is paid by its comparably-wide input code
+    use super::{band, envelope, to_decision, whole_input, Envelope};
+    pub const DECODE_DENSE: Envelope                = envelope(120_035,           band(0, 0),            band(4, 2),      whole_input(468_758, 1)); // wire decode is validate + wrap; the payloads ride the word-valued form, so the limb column reads zero and the whole-input scan floor is the liveness signal
+    pub const CMP_DENSE: Envelope                   = envelope( 30_720,           band(0, 0), band(156_254, 93_752),       band(468_760, 281_256)); // the iterative sweep over the Bytes-backed at-rest form (OpenedPair states the pair walk's opening move once); word-valued payloads keep the limb column at zero, and the touch and scan tripwires are the liveness signal
+    pub const CMP_DENSE_SELF: Envelope              = envelope( 51_200,           band(0, 0), band(156_257, 93_753),       band(937_515, 562_509)); // aligned ties in lockstep to full depth: both streams' bits scanned whole
+    pub const JOIN_DENSE: Envelope                  = envelope(130_277,           band(0, 0), band(156_255, 93_753),       band(625_018, 375_010)); // the emit kernel's peak alone: the value-operator cell's lhs clone is a refcount bump, not a byte copy of the operand; word-valued payloads keep the limb column at zero, and the touch and scan tripwires are the liveness signal
+    pub const DECODE_BIGROOT: Envelope              = envelope( 60_090,       band(783, 469),    band(2_348, 1_408),      whole_input(137_512, 1)); // wire decode is validate + wrap; the one wide root magnitude keeps a linear limb record while the word-valued form carries the narrow codes
+    pub const CMP_BIGROOT: Envelope                 = envelope( 39_540,       band(783, 469),   band(14_849, 8_909),        band(137_514, 82_508)); // the iterative sweep over the Bytes-backed at-rest form; the wide root's decode is the limb record
+    pub const JOIN_BIGROOT: Envelope                = envelope( 85_060,     band(1_565, 939),   band(14_850, 8_910),       band(275_028, 165_016)); // the emit kernel's peak alone (the lhs clone is a refcount bump); the wide root decodes on both sides carry the limb record
+    pub const DECODE_HUGELEAF: Envelope             = envelope(122_504,   band(2_443, 1_465),    band(7_327, 4_395),      whole_input(312_503, 1)); // the validating wire decode holds the running height; one wide gamma code's linear limb work
+    pub const JOIN_HUGELEAF: Envelope               = envelope(185_494,   band(4_887, 2_931),    band(7_329, 4_397),       band(625_010, 375_006)); // the emit kernel holds both payload buffers, and the lhs clone is a refcount bump, so the public join's peak is the emit kernel's alone
+    pub const JOIN_ABSORB: Envelope                 = envelope(270_798,   band(4_887, 2_931), band(163_580, 98_148),     band(1_250_013, 750_007)); // the collapse-heavy extreme: one truncation per level around a held wide code, which absorb never moves
+    pub const ID_JOIN: Envelope                     = envelope(279_132,           band(0, 0),            band(0, 0),    whole_input(3_125_023, 2)); // iterative id walks: frame bits on the heap
+    pub const ID_COVERS: Envelope                   = envelope(     10,           band(0, 0),            band(0, 0), to_decision(1_250_005, 2, 2)); // iterative id walks; the diverted pair is decided at the last unary node's tag pair, so the scan floor leaves each stream's terminal unread
+    pub const ID_DISJOINT: Envelope                 = envelope(     10,           band(0, 0),            band(0, 0), to_decision(1_250_005, 2, 2)); // iterative id walks; the diverted pair is decided at the last unary node's tag pair, so the scan floor leaves each stream's terminal unread
+    pub const ID_WITHOUT: Envelope                  = envelope(521_110,           band(0, 0),            band(0, 0),    whole_input(2_500_005, 1)); // iterative complement over the Bytes-backed at-rest form; `input_bytes` counts the subtrahend alone, not the seed's two-bit stream, so the floor errs on the low side; dev builds run no shadow re-parse of the diff emission (the differential suites carry the normal-form check)
+    pub const DECODE_CLIFF: Envelope                = envelope(  4_052,         band(88, 52),    band(4_003, 2_401),       whole_input(17_923, 1)); // wire decode is validate + wrap; each cliff crossing's limb work is paid by its own wide stored code
+    pub const CMP_CLIFF: Envelope                   = envelope(  1_330,         band(88, 52),    band(5_284, 3_170),         band(17_925, 10_755)); // the cliff-free sweep (two accumulators, opened once) over the Bytes-backed at-rest form
+    pub const JOIN_CLIFF: Envelope                  = envelope(  5_362,       band(308, 184),    band(5_289, 3_173),         band(35_848, 21_508)); // the emit kernel's peak alone (the lhs clone is a refcount bump); each re-coded tooth's limb work is paid by its comparably-wide input code
+    pub const MEET_CLIFF: Envelope                  = envelope(  4_422,         band(88, 52),    band(5_289, 3_173),         band(23_055, 13_833)); // the pointwise minimum clamps every tooth to the flat operand's height while every delta still crosses the carry boundary in the accumulator
+    pub const DECODE_WIDE_TOOTH: Envelope           = envelope(125_100, band(29_509, 17_705),   band(14_218, 8_530),    whole_input(1_000_480, 1)); // wire decode is validate + wrap; each wide delta's limb work is paid by its own zigzag code, and the adopted buffer prices the wide payloads
+    // CMP_WIDE_TOOTH's deliberately thin heap margin is a change-detector
+    // on the backend's and the accumulator's allocation policies: the
+    // measurement depends on the big-integer backend's allocation policy at
+    // the locked version, so a dependency bump is a deliberate re-measure
+    // event, not noise.
+    pub const CMP_WIDE_TOOTH: Envelope              = envelope(  1_250, band(29_509, 17_705),   band(15_499, 9_299),     band(1_000_483, 600_289)); // each wide delta's limb work paid by its own zigzag code; heap stays at the stacks, the accumulator, and the zero-run ledger's map node
+    pub const JOIN_WIDE_TOOTH: Envelope             = envelope(128_312, band(74_477, 48_534),   band(15_504, 9_302),   band(2_000_963, 1_200_577)); // each wide delta re-coded into the output, paid by its own zigzag code
+    pub const MEET_WIDE_TOOTH: Envelope             = envelope(127_087, band(29_509, 17_705),   band(15_504, 9_302),     band(1_005_613, 603_367)); // wide deltas folded but never re-emitted: the collapse discipline at spilled operand widths
+    pub const DECODE_ALT_SPINE: Envelope            = envelope(120_035,           band(0, 0),            band(4, 2),      whole_input(468_758, 1)); // wire decode is validate + wrap; per-level state stays two bits however the descent direction flips
     // Skyline validator rows: the validator's transient is the
-    // open-ancestor bit stack plus reallocation growth — bits per level,
-    // not frames. The validator and decoder rows carry
-    // the sweep tables' scanned-bits column: their work is cursor reads
-    // end to end (the validator allocates near-nothing and, off the wide
-    // families, does little arithmetic), so scan is the column that sees
-    // a re-read the others cannot. Decode is validate plus the wrap, so
-    // each shape's scan reading equals its validate row's.
-    pub const SKYLINE_VALIDATE_DENSE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); // the open-ancestor bit stack; word-valued payloads keep the limb column at zero
-    pub const SKYLINE_VALIDATE_CLIFF: SweepEnvelope = sweep_envelope(1_770, 0, 88, 17_923, 52); // the cliff-free accumulator: amortized O(1) per delta
-    pub const SKYLINE_VALIDATE_WIDE_TOOTH: SweepEnvelope = sweep_envelope(1_520, 0, 29_509, 1_000_480, 17_705); // each wide delta's limb work is paid by its own zigzag code; heap stays at the bit stack plus the zero-run ledger's map node
-    pub const SKYLINE_VALIDATE_HUGELEAF: SweepEnvelope   = sweep_envelope(    80_980,        0,         2_443, 312_503, 1_465); // one wide decode and one wide accumulator load, both linear in the code's width
-    pub const SKYLINE_VALIDATE_ALT_SPINE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); // per-level state stays two bits however the descent direction flips
-    // Skyline decoder rows: validation plus the wrap into storage — the
-    // stored coding is the skyline stream itself, so decode materializes
-    // nothing beyond the copy and stays priced by the wire input.
-    pub const SKYLINE_DECODE_DENSE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); // decode is validate + wrap: the wrap allocates the copy once, exactly sized
-    pub const SKYLINE_DECODE_CLIFF: SweepEnvelope = sweep_envelope(2_250, 0, 88, 17_923, 52); // decode is validate + wrap: the wrap allocates the copy once, exactly sized
-    pub const SKYLINE_DECODE_WIDE_TOOTH: SweepEnvelope = sweep_envelope(125_100, 0, 29_509, 1_000_480, 17_705); // decode is validate + wrap; the once-allocated copy prices the wide payloads
-    pub const SKYLINE_DECODE_HUGELEAF: SweepEnvelope     = sweep_envelope(    83_440,        0,         2_443, 312_503, 1_465); // decode is validate + wrap
-    pub const SKYLINE_DECODE_ALT_SPINE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); // decode is validate + wrap: the wrap allocates the copy once, exactly sized
+    // open-ancestor bit stack plus reallocation growth, bits per level,
+    // not frames. Its work is cursor reads end to end (it allocates
+    // near-nothing and, off the wide families, does little arithmetic),
+    // so scan is the column that sees a re-read the others cannot, and
+    // the whole-input floor under it is what a validator that stops
+    // reading fails. Decode is validate plus the wrap, so each shape's
+    // scan reading equals its validate row's.
+    pub const SKYLINE_VALIDATE_DENSE: Envelope      = envelope( 61_440,           band(0, 0),            band(4, 2),      whole_input(468_758, 1)); // the open-ancestor bit stack; word-valued payloads keep the limb column at zero
+    pub const SKYLINE_VALIDATE_CLIFF: Envelope      = envelope(  1_770,         band(88, 52),    band(4_003, 2_401),       whole_input(17_923, 1)); // the cliff-free accumulator: amortized O(1) per delta
+    pub const SKYLINE_VALIDATE_WIDE_TOOTH: Envelope = envelope(  1_520, band(29_509, 17_705),   band(14_218, 8_530),    whole_input(1_000_480, 1)); // each wide delta's limb work is paid by its own zigzag code; heap stays at the bit stack plus the zero-run ledger's map node
+    pub const SKYLINE_VALIDATE_HUGELEAF: Envelope   = envelope( 80_980,   band(2_443, 1_465),    band(7_327, 4_395),      whole_input(312_503, 1)); // one wide decode and one wide accumulator load, both linear in the code's width
+    pub const SKYLINE_VALIDATE_ALT_SPINE: Envelope  = envelope( 61_440,           band(0, 0),            band(4, 2),      whole_input(468_758, 1)); // per-level state stays two bits however the descent direction flips
 }
 
 // ─── meter liveness canaries ────────────────────────────────────────────────
```

<!-- annotation -->
> **envelopes-b-8** (4), line 235:
>
> One envelope type for every table: a heap ceiling plus one `Bound` per counter column (limb, touch, scan). I did not use `ByCurrency<Bound>` from the board: it carries a segments slot ruling 38 dissolves and a heap slot whose floor genre differs (the heap column has canaries, not a tripwire), so the ready-made shape would have needed two carve-outs.

<!-- annotation -->
> **envelopes-a-4** (4), line 248:
>
> The per-column pin as a ceiling and a floor, replacing four structs whose only difference was which columns existed.

<!-- annotation -->
> **envelopes-a-6** (50), line 258:
>
> The floor genre is typed, so a row states which floor it carries by constructor (`band` for the ×0.75 tripwire, `whole_input` for the derived one-bit-per-input-byte liveness floor) rather than in a comment the code cannot check.

<!-- annotation -->
> **envelopes-a-4** (4), line 293:
>
> The one constructor for a tripwire-floored column; the four `const fn` builders whose only job was the cfg underscore dance are gone.

<!-- annotation -->
> **envelopes-a-4** (4), line 323:
>
> The one `const fn` builder of an `Envelope`.

<!-- annotation -->
> **envelopes-b-8** (4), line 336:
>
> Rows converted mechanically from the parent's four constructors (rows-step1.json in the lane's scratchpad is the parent's numbers, checked against this table); the tick-row pointer comment went with the four-column table it described.

<!-- annotation -->
> **envelopes-b-8** (4), line 239:
>
> The column's type is `Bound`, not `Option<Bound>`: the placeholder lived only inside this lane's series.

<!-- annotation -->
> **envelopes-b-8** (4), line 341:
>
> The 66 newly pinned cells across the seven tables are each the unified harness's reading at the parent library (the library is untouched in this series), ceiling ×1.25 rounded up, floor ×0.75 rounded down; every cell with its reading is attributed in the step-3 commit message.

<!-- annotation -->
> **envelopes-a-6** (50), line 258:
>
> The two floor genres the file doc names, typed: a row states which it carries by constructor. `WholeInput` is 8 × input_bytes less one byte of padding per operand stream, not one bit per byte: the review's own witness (the unary-run scan tap removed) drops the dense validator's reading from 375006 to 125005 bits, which a per-byte floor (46_876) would not catch and this one does.

<!-- annotation -->
> **envelopes-a-6** (50), line 302:
>
> The derived floor's constructor; used only on rows whose `input_bytes` is the byte length of exactly the streams the walk reads (`cmp_cliff`, denominated in packed generator bytes, reads 0.05 scan bits per input byte and must keep the tripwire).

<!-- annotation -->
> **envelopes-a-6** (50), line 337:
>
> The three dense row comments now state the liveness signal their rows carry (a whole-input scan floor on decode; touch and scan tripwires on cmp and join), where the parent's `DECODE_DENSE` comment named floors its four-column table did not have.

<!-- annotation -->
> **envelopes-a-8** (4), line 339:
>
> The kernel-only shapes ported to the public operations under the door roster: `CMP_DENSE_SELF` (`partial_cmp` on a byte-equal, buffer-distinct copy), `CMP_WIDE_TOOTH`, `JOIN_ABSORB`, `JOIN_WIDE_TOOTH`, `MEET_CLIFF`, `MEET_WIDE_TOOTH`. Every cell is the door's own reading, ceiling ×1.25, floor ×0.75, listed in the commit beside the retired twin's reading and ceiling.

<!-- annotation -->
> **envelopes-a-11** (4), line 355:
>
> `DECODE_WIDE_TOOTH` and `DECODE_ALT_SPINE` through `Version::decode`, so no shape loses its decode pin when the five `SKYLINE_DECODE_*` rows (a meter-only wrapper whose columns duplicated the validate rows) dissolve; their scan floors are whole-input like the other decode rows.

<!-- annotation -->
> **envelopes-a-8** (4), line 356:
>
> The Cargo.lock note travels with its row from the retired sweep table.

<!-- annotation -->
> **fresh-eyes-F1** (4), line 342:
>
> The retired `SKYLINE_CMP_BIGROOT` twin's tighter heap ceiling (39_540) carries onto the door row in place of 40_340; both rows read 32_272 B, so the tightest committed number on this shape and operation does not go up when the twin goes.

<!-- annotation -->
> **fresh-eyes-F3** (88), line 284:
>
> The derived floor now states its premise and carries a per-stream tail allowance: validators, decoders, and the id walks whose output depends on every tag read to the end (tail 0); the diverted pair's covers and disjoint walks are decided at the last unary node's tag pair and may leave each stream's 2-bit terminal unread, so their floor allows a 4-bit tail per stream (999_992 bits against the 1_000_004 reading) and an early exit at the decision is an improvement the floor admits, per ruling 88.

<!-- annotation -->
> **fresh-eyes-F3** (88), line 315:
>
> The decision-point constructor; `whole_input` keeps tail 0.

<!-- annotation -->
> **fresh-eyes-F3** (88), line 348:
>
> `ID_COVERS` and `ID_DISJOINT` move to `to_decision(…, 2, 4)`; ceilings unchanged.

<!-- annotation -->
> **fresh-eyes-F7** (105), line 358:
>
> The hand-maintained dependency version string leaves the prose; the note says what the pin depends on.

<!-- annotation -->
> **fresh-eyes-F3** (105), line 265:
>
> Summary split after its first sentence for doclint's limit.

<!-- annotation -->
> **fresh-eyes-W4** (50), line 270:
>
> The floor's doc says what it proves on which rows: the scan counter also holds writes and splices, so on `ID_JOIN` (2_500_018 counted against 1_000_004 input bits) and `ID_WITHOUT` (2_000_004 against 500_002) the floor attests only total bypass, while on the read-only validator, decoder, covers, and disjoint rows it attests the reads; the file doc's genre bullet says the same in one clause.

<!-- annotation -->
> **fresh-eyes-taste** (105), line 350:
>
> The row says its `input_bytes` omits the seed's two-bit stream, so the floor is conservative.

<!-- annotation -->
> **fresh-eyes-W3** (88), line 281:
>
> The tail matches the derivation: the pair is decided at the last unary node's tag pair, and only the 2-bit terminal lies below it, so the floor admits an exit at the decision (999_996 against the 1_000_004 reading) and rejects a walk that skipped the deciding tags. The walk in `party/ops/compare.rs` reads the tags it decides on and the terminals only when it does not exit, which the derivation matches.

<a id="hunk-7"></a>
### crates/before/tests/meter.rs `@@ -354,57 +431,188 @@ fn heap_meter_floor_on_decode_dense() {`

```diff
@@ -354,57 +431,188 @@ fn heap_meter_floor_on_decode_dense() {
 const ISOLATION_NOTE: &str = "note: the meters are process-global and meaningful only one \
      scenario per process: run under cargo nextest, not a shared-process cargo test";
 
-/// Run one scenario body under both meters and assert its envelope.
+/// One counter column of the harness: its MEASURED key, its unit in
+/// failure messages, its counter's reset and read, and the row's pin.
+#[derive(Clone, Copy)]
+struct Column {
+    /// The key of the column's `key=reading` fragment on the MEASURED line.
+    key: &'static str,
+    /// The unit named when the column's ceiling is exceeded.
+    unit: &'static str,
+    /// Reset the counter before the scenario body.
+    reset: fn(),
+    /// Read the counter after the body.
+    read: fn() -> u64,
+    /// The row's pin for this column.
+    pin: fn(&Envelope) -> Bound,
+    /// The row's pin for this column, writable (the harness self-test's
+    /// probe).
+    pin_mut: fn(&mut Envelope) -> &mut Bound,
+}
+
+/// The counter columns, in MEASURED-line order.
+const COLUMNS: [Column; 3] = [
+    Column {
+        key: "limb_ops",
+        unit: "limb operations",
+        reset: meter::reset_limb_ops,
+        read: meter::limb_ops,
+        pin: |env| env.limb,
+        pin_mut: |env| &mut env.limb,
+    },
+    Column {
+        key: "touches",
+        unit: "accumulator digit touches",
+        reset: suanpan::touch_meter::reset,
+        read: suanpan::touch_meter::touches,
+        pin: |env| env.touch,
+        pin_mut: |env| &mut env.touch,
+    },
+    Column {
+        key: "scan_bits",
+        unit: "scanned bits",
+        reset: meter::reset_scan_bits,
+        read: meter::scan_bits,
+        pin: |env| env.scan,
+        pin_mut: |env| &mut env.scan,
+    },
+];
+
+/// Run one scenario body under every meter and assert its envelope.
 ///
-/// Prints the measured numbers (visible under `--no-capture` or on failure)
-/// so re-pinning an envelope never requires editing the harness. The
-/// scenario's result is returned alive, so the peak includes the fully
-/// materialized output, and is dropped by the caller after measurement.
+/// Prints the MEASURED line (visible under `--no-capture` or on failure) so
+/// re-pinning never requires editing the harness. The scenario's result is
+/// returned alive, so the peak includes the fully materialized output, and
+/// is dropped by the caller after measurement.
 fn metered<R>(name: &str, input_bytes: usize, env: &Envelope, f: impl FnOnce() -> R) -> R {
-    meter::reset_stack_segments();
-    #[cfg(feature = "limb-meter")]
-    meter::reset_limb_ops();
+    for column in &COLUMNS {
+        (column.reset)();
+    }
     HEAP.reset_peak_usage();
     let baseline = HEAP.current_usage();
     let r = f();
     let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
-    let segments = meter::stack_segments();
-    #[cfg(feature = "limb-meter")]
-    let limb_ops = meter::limb_ops();
-    #[cfg(feature = "limb-meter")]
-    eprintln!(
-        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments} limb_ops={limb_ops}"
-    );
-    #[cfg(not(feature = "limb-meter"))]
-    eprintln!(
-        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments}"
-    );
+    let readings = COLUMNS.map(|column| (column.read)());
+    let mut line = format!("MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap}");
+    for (column, reading) in COLUMNS.iter().zip(readings) {
+        write!(line, " {}={reading}", column.key).expect("a String write cannot fail");
+    }
+    eprintln!("{line}");
     assert!(
         peak_heap <= env.peak_heap,
         "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B): {ISOLATION_NOTE}",
         env.peak_heap,
     );
+    for (column, reading) in COLUMNS.iter().zip(readings) {
+        let bound = (column.pin)(env);
+        assert!(
+            reading <= bound.ceiling,
+            "{name}: {reading} {} exceed the pinned envelope {}: {ISOLATION_NOTE}",
+            column.unit,
+            bound.ceiling,
+        );
+        match bound.floor {
+            Floor::Tripwire(floor) => assert!(
+                reading >= floor,
+                "{name}: the {} counter reads {reading}, below the {floor} improvement \
+                 tripwire (measured x0.75): attribute the drop; an improvement re-pins \
+                 the band, and a dead meter is the bypass this column exists to catch",
+                column.key,
+            ),
+            Floor::LiveBits { streams, tail_bits } => {
+                let floor = (8 * input_bytes as u64).saturating_sub(streams * (8 + tail_bits));
+                assert!(
+                    reading >= floor,
+                    "{name}: the {} counter reads {reading}, under the {floor}-bit liveness floor \
+                     over {input_bytes} input bytes: the walk left the metered \
+                     primitives",
+                    column.key,
+                );
+            }
+        }
+    }
+    r
+}
+
+/// Magnitude (bits) of the harness self-test's probe operand.
+const HARNESS_PROBE_MAGNITUDE_BITS: usize = 1_024;
+
+/// Depth of the harness self-test's probe operand.
+const HARNESS_PROBE_DEPTH: usize = 64;
+
+/// The harness judges every column.
+///
+/// A body that moves every counter fails under a ceiling below its
+/// reading on any one column, under a floor of either genre above its
+/// reading on any one column, and under a zero heap ceiling.
+///
+/// The harness's own negative control: a column whose assert is skipped, a
+/// reset that leaves a stale reading, or a read wired to a counter the body
+/// never moves shows up here as a probe that passes when it must not.
+#[test]
+fn harness_judges_every_column() {
+    let v = version_of(&Shape::Bigroot.packed2(HARNESS_PROBE_MAGNITUDE_BITS, HARNESS_PROBE_DEPTH));
+    let input = v.encode().len();
+    let open = Envelope {
+        peak_heap: usize::MAX,
+        limb: band(u64::MAX, 0),
+        touch: band(u64::MAX, 0),
+        scan: band(u64::MAX, 0),
+    };
+    HEAP.reset_peak_usage();
+    let baseline = HEAP.current_usage();
+    let r = metered("harness_probe", input, &open, || v.rank());
+    // Read the counters before anything formats the result: the readings
+    // must be the probe body's alone.
+    let readings = COLUMNS.map(|column| (column.read)());
     assert!(
-        segments <= env.segments,
-        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.segments,
-    );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        limb_ops <= env.limb_ops,
-        "{name}: {limb_ops} limb operations exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.limb_ops,
-    );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        limb_ops >= env.limb_floor,
-        "{name}: limb counter reads {limb_ops}, below the {} improvement \
-         tripwire (measured x0.75): attribute the drop — an honest \
-         improvement re-pins the band; a dead meter is the bypass this \
-         column exists to catch",
-        env.limb_floor,
+        HEAP.peak_usage().saturating_sub(baseline) > 0,
+        "the probe body must allocate"
     );
-    r
+    consumed(r);
+    let fails_over = |env: &Envelope, input: usize| {
+        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
+            consumed(metered("harness_probe", input, env, || v.rank()))
+        }))
+        .is_err()
+    };
+    let fails = |env: &Envelope| fails_over(env, input);
+    let closed = Envelope {
+        peak_heap: 0,
+        ..open
+    };
+    assert!(fails(&closed), "a zero heap ceiling must fail the probe");
+    for (column, reading) in COLUMNS.iter().zip(readings) {
+        assert!(
+            reading > 0,
+            "the probe body must move the {} counter",
+            column.key
+        );
+        let mut over = open;
+        *(column.pin_mut)(&mut over) = band(reading - 1, 0);
+        assert!(
+            fails(&over),
+            "{}: a ceiling under the reading must fail the probe",
+            column.key
+        );
+        let mut under = open;
+        *(column.pin_mut)(&mut under) = band(u64::MAX, reading + 1);
+        assert!(
+            fails(&under),
+            "{}: a floor over the reading must fail the probe",
+            column.key
+        );
+        let mut unread = open;
+        *(column.pin_mut)(&mut unread) = whole_input(u64::MAX, 1);
+        assert!(
+            fails_over(
+                &unread,
+                usize::try_from(reading + 1).expect("a reading fits usize")
+            ),
+            "{}: a whole-input floor over the reading must fail the probe",
+            column.key
+        );
+    }
 }
 
 /// Lift a generated shape into a [`Version`], outside any measurement.
```

<!-- annotation -->
> **envelopes-a-4** (4), line 437:
>
> The static column table `metered` walks: each counter's MEASURED key, message unit, reset, read, and the row accessor. A column added here cannot be skipped by the harness, and the self-test below proves each column's asserts fire.

<!-- annotation -->
> **envelopes-a-4** (4), line 454:
>
> MEASURED-line order is `limb_ops touches scan_bits`, the order the parent's query harness printed, so the largest table's lines stay byte-identical whole and every other row's parent fragments stay in place.

<!-- annotation -->
> **meter-adequacy-11** (108), line 458:
>
> The column table's reset and read are the crate's counter functions; the cfg-gated wrapper functions that returned `None` without a feature are deleted, since the manifest guarantees the features.

<!-- annotation -->
> **envelopes-a-4** (4), line 487:
>
> The one harness: reset every column, run, read the heap peak first (before any allocation of the harness's own), then the counters, print one MEASURED line composed from fragments, assert the heap ceiling, then each column's ceiling and floor. The tripwire message exists once.

<!-- annotation -->
> **envelopes-a-4** (4), line 553:
>
> The unified harness's own negative control: a body that moves every counter must fail under a ceiling one below its reading and a floor one above it on each column, and under a zero heap ceiling. The worst artifact a harness refactor can produce is one whose asserts are silently skipped; this is the committed demonstration that none are. It prints `MEASURED harness_probe` lines, which a re-pinner ignores by name.

<!-- annotation -->
> **crate-root-32** (38), line 487:
>
> The harness no longer resets or reads `meter::stack_segments` and prints no `segments=` fragment; every other fragment of every MEASURED line is byte-identical to step 1's (measured-step2.log against measured-step1.log in the lane's scratchpad, `--drop segments`). The readers themselves are the board lane's to delete.

<!-- annotation -->
> **envelopes-a-6** (50), line 522:
>
> The harness computes the derived floor from the row's own `input_bytes` at run time; saturating so a tiny input cannot underflow.

<!-- annotation -->
> **meter-adequacy-11** (108), line 445:
>
> With the features guaranteed by the manifest, a column's read is the counter's own function and the `Option` plumbing, the six cfg-gated wrapper functions, and the harness's and self-test's `None` skips are deleted.

<!-- annotation -->
> **meter-adequacy-11** (108), line 459:
>
> The column table names the crate's counter functions directly.

<!-- annotation -->
> **fresh-eyes-F3** (88), line 523:
>
> The harness subtracts the padding and the tail per stream.

<!-- annotation -->
> **fresh-eyes-F6** (4), line 565:
>
> The self-test reads the counters and the heap peak before `consumed` formats the probe's result, so a `Debug` impl that did metered work could not fail the test for a reason unrelated to the harness.

<!-- annotation -->
> **fresh-eyes-taste** (105), line 526:
>
> The floor's failure message no longer says "whole-input" for a `to_decision` floor; the negative control's two greps match the new wording.

<a id="hunk-8"></a>
### crates/before/tests/meter.rs `@@ -417,6 +625,26 @@ fn party_of(p: &meter::Packed) -> Party {`

```diff
@@ -417,6 +625,26 @@ fn party_of(p: &meter::Packed) -> Party {
     Party::decode(&p.bytes[..]).expect("generated shape is strict normal form")
 }
 
+/// An emit kernel's output on two versions' skyline streams, built outside
+/// measurement: the byte-identity leg of the public join and meet rows.
+fn emitted(
+    kernel: fn(
+        meter::skyline::BitsView<'_>,
+        meter::skyline::BitsView<'_>,
+    ) -> meter::skyline::BitsBuf,
+    a: &Version,
+    b: &Version,
+) -> meter::skyline::BitsBuf {
+    let (a, b) = (meter::skyline::encode(a), meter::skyline::encode(b));
+    kernel(meter::skyline::view(&a), meter::skyline::view(&b))
+}
+
+/// The rank kernel's answer on a version's skyline stream, built outside
+/// measurement: the identity leg of the public rank rows.
+fn kernel_rank(v: &Version) -> before::Rank {
+    meter::skyline::query::rank(meter::skyline::view(&meter::skyline::encode(v)))
+}
+
 /// Assert a scenario result is consumed, so the operation cannot be
 /// dead-code-eliminated and the walk provably ran to completion.
 fn consumed<T: Debug>(v: T) -> String {
```

<!-- annotation -->
> **envelopes-a-8** (4), line 630:
>
> The byte-identity leg the retired join twins carried (kernel emission equals the door's stream) now lives on the door rows, computed outside measurement; the meet and absorb rows carry closed-form legs instead (the flat operand is the whole result) and the cmp rows the verdict.

<!-- annotation -->
> **envelopes-a-8** (108), line 644:
>
> The retired twins' identity leg (kernel rank equals the public rank) moves onto the door rows in place of `consumed(r)`, computed outside measurement, as the join rows' `emitted` leg does.

<a id="hunk-9"></a>
### crates/before/tests/meter.rs `@@ -425,8 +653,8 @@ fn consumed<T: Debug>(v: T) -> String {`

```diff
@@ -425,8 +653,8 @@ fn consumed<T: Debug>(v: T) -> String {
 
 // ─── dense spine scenarios ──────────────────────────────────────────────────
 
-/// Decoding the dense spine stays within its pinned peak-heap and
-/// stack-segment envelope (the parse-stack cost, linear in depth today).
+/// Decoding the dense spine stays within its envelope: wire decode is
+/// validate plus the wrap, and the payloads ride the word-valued form.
 #[test]
 fn decode_dense_envelope() {
     let p = Shape::Dense.packed1(DENSE_DEPTH);
```

<!-- annotation -->
> **envelopes-a-2** (38), line 656:
>
> "pinned peak-heap and stack-segment envelope (the parse-stack cost, linear in depth today)" named a column that no longer exists and a mechanism that is not the decoder's; restated from the row's table comment, "today" gone (ruling 45).

<a id="hunk-10"></a>
### crates/before/tests/meter.rs `@@ -438,7 +666,8 @@ fn decode_dense_envelope() {`

```diff
@@ -438,7 +666,8 @@ fn decode_dense_envelope() {
 }
 
 /// Comparing the dense spine against the empty version stays within its
-/// envelope (the recursion-frame cost: heap stays flat, segments do not).
+/// envelope: the iterative sweep over the at-rest form, the whole deep
+/// side consumed against one depth-0 plateau.
 #[test]
 fn cmp_dense_envelope() {
     let p = Shape::Dense.packed1(DENSE_DEPTH);
```

<!-- annotation -->
> **crate-root-32** (38), line 668:
>
> The clause "the recursion-frame cost: heap stays flat, segments do not" is excised (the entry names this line): the sweep is iterative, and the doc now says what the row prices, restated from its table comment per ruling 45.

<!-- annotation -->
> **envelopes-a-8** (4), line 672:
>
> The retired twin's verdict leg (`Some(Greater)`) replaces `consumed(r)`; likewise on `cmp_bigroot` and `cmp_cliff`. `cmp_bigroot`'s doc dropped "today the worst amplifier: ... quadratic", which described a replaced implementation (ruling 45).

<a id="hunk-11"></a>
### crates/before/tests/meter.rs `@@ -446,372 +675,151 @@ fn cmp_dense_envelope() {`

```diff
@@ -446,372 +675,151 @@ fn cmp_dense_envelope() {
     let r = metered("cmp_dense", p.bytes.len(), &envelope::CMP_DENSE, || {
         v.partial_cmp(&Version::new())
     });
-    consumed(r);
+    assert_eq!(
+        r,
+        Some(Ordering::Greater),
+        "the dense spine strictly dominates the empty version"
+    );
 }
 
-/// Joining the dense spine with a one-tick version stays within its envelope
-/// (the emit-path cost, linear in nodes).
+/// Comparing the dense spine against a byte-equal, buffer-distinct copy
+/// stays within its envelope.
+///
+/// Every boundary is an aligned tie, both cursors advance in lockstep to
+/// full depth, and the verdict is Equal only after both streams are
+/// wholly consumed.
 #[test]
-fn join_dense_envelope() {
+fn cmp_dense_self_envelope() {
     let p = Shape::Dense.packed1(DENSE_DEPTH);
     let v = version_of(&p);
-    let one = Version::try_from(1u64).expect("a one-tick version is valid");
-    let joined = metered("join_dense", p.bytes.len(), &envelope::JOIN_DENSE, || {
-        &v | &one
-    });
-    drop(joined);
+    let w = version_of(&p);
+    let r = metered(
+        "cmp_dense_self",
+        2 * p.bytes.len(),
+        &envelope::CMP_DENSE_SELF,
+        || v.partial_cmp(&w),
+    );
+    assert_eq!(r, Some(Ordering::Equal), "identical streams read equal");
 }
 
-/// Ticking the dense spine stays within its envelope (the fill-splice
-/// round-trip cost, linear in nodes today).
+/// Joining the dense spine with a one-tick version stays within its
+/// envelope.
+///
+/// The 125k-level walk emits and collapses on path-bit stacks and one
+/// accumulator, with the peak in the emitted stream itself. The join is
+/// the emit kernel's stream, byte for byte.
 #[test]
-fn tick_dense_envelope() {
+fn join_dense_envelope() {
     let p = Shape::Dense.packed1(DENSE_DEPTH);
-    let mut v = version_of(&p);
-    let seed = Party::seed();
-    query_metered("tick_dense", p.bytes.len(), &query_env::TICK_DENSE, || {
-        v.tick(&seed)
+    let v = version_of(&p);
+    let one = Version::try_from(1u64).expect("a one-tick version is valid");
+    let joined = metered("join_dense", p.bytes.len(), &envelope::JOIN_DENSE, || {
+        &v | &one
     });
-    drop(v);
+    assert_eq!(
+        meter::skyline::encode(&joined),
+        emitted(meter::skyline::emit::join, &v, &one),
+        "the public join must be the emit kernel's stream"
+    );
 }
 
-/// Ticking the wide right-full chain (a bigroot magnitude over the
-/// nested-full id) stays within its envelope: the anchor-web walk
-/// touches the wide first payload O(1) times, never once per shortcut
-/// level.
+/// The flatness pin: `O(|v| + |p| + log n)` as a committed two-point
+/// check, not prose.
+///
+/// On each tick-designated family the whole cost
+/// movement from `ticks(512)` to `ticks(4096)` — three doublings — must
+/// sit inside the boundary codes' gamma-width delta band: the two codes
+/// that carry the count widen by 2 bits per doubling each, and no other
+/// column may move beyond a word of slack. An implementation iterating
+/// any fraction of the count moves every column by ~8x here and cannot
+/// hide in a constant; a dead meter reads zero movement AND a zero
+/// point, which the envelope rows' improvement tripwires already reject.
 #[test]
-fn tick_nested_wide_envelope() {
-    let ev = Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
-    let id = Shape::NestedFullId.packed1(TICK_CROSS_SCALE);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered(
-        "tick_nested_wide",
-        input,
-        &query_env::TICK_NESTED_WIDE,
-        || v.tick(&p),
-    );
-    drop(v);
+fn ticks_flatness_holds_the_log_band() {
+    let cases: Vec<(&str, Version, Party)> = vec![
+        (
+            "dense",
+            version_of(&Shape::Dense.packed1(DENSE_DEPTH)),
+            Party::seed(),
+        ),
+        (
+            "nested-wide",
+            version_of(&Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
+            party_of(&Shape::NestedFullId.packed1(TICK_CROSS_SCALE)),
+        ),
+        (
+            "mirror-wide",
+            version_of(&Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
+            party_of(&Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE)),
+        ),
+    ];
+    for (name, v, p) in &cases {
+        let lo = ticks_counters(v, p, TICKS_POINT_LO);
+        let hi = ticks_counters(v, p, TICKS_POINT_HI);
+        let moved = [
+            ("scan", lo.0, hi.0, TICKS_FLATNESS_SCAN_BAND),
+            ("limb", lo.1, hi.1, TICKS_FLATNESS_LIMB_BAND),
+            ("touch", lo.2, hi.2, TICKS_FLATNESS_TOUCH_BAND),
+        ];
+        for (col, at_lo, at_hi, band) in moved {
+            let delta = at_hi.abs_diff(at_lo);
+            eprintln!("MEASURED ticks_flatness {name}/{col}: lo={at_lo} hi={at_hi} delta={delta}");
+            assert!(
+                delta <= band,
+                "{name}/{col}: ticks({}) -> ticks({}) moved {delta}                  (from {at_lo} to {at_hi}), outside the gamma-width band {band}",
+                TICKS_POINT_LO,
+                TICKS_POINT_HI,
+            );
+        }
+    }
 }
 
-/// Ticking the wide memo chain (a wide-tail spine under the
-/// nested-left-full id) stays within its envelope: the pre-scan's
-/// frame ledger stores no link for the shared wide minimum, so
-/// nothing is materialized per site.
-#[test]
-fn tick_mirror_wide_envelope() {
-    let ev = Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
-    let id = Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered(
-        "tick_mirror_wide",
-        input,
-        &query_env::TICK_MIRROR_WIDE,
-        || v.tick(&p),
-    );
-    drop(v);
+/// One `ticks(n)` run's `(scan bits, limb ops, touches)` on fresh
+/// counters — the flatness pin's probe.
+fn ticks_counters(v: &Version, p: &Party, n: u64) -> (u64, u64, u64) {
+    let mut v = v.clone();
+    meter::reset_scan_bits();
+    meter::reset_limb_ops();
+    suanpan::touch_meter::reset();
+    v.ticks(p, n);
+    (
+        meter::scan_bits(),
+        meter::limb_ops(),
+        suanpan::touch_meter::touches(),
+    )
 }
 
-/// Ticking the descending staircase under a party owning one deep
-/// diverted fragment (the ownership-hole family) stays within an
-/// envelope the leaf-by-leaf walk exceeds.
+/// One `ticks(n)` run at an arbitrary-width count, on the operand's
+/// post-fill tree, with the exact `min_ticks` movement as the value
+/// leg.
 ///
-/// The fill walk's unowned regions are whole staircase runs, and the
-/// block scan must fold each into O(1) accumulator work instead of
-/// per-leaf freight. The touch ceiling is the skip's liveness signal
-/// — it sits below the per-leaf mechanism's reading, so the fast path
-/// must demonstrably engage; the scan column pins that every skipped
-/// bit is still read.
-#[test]
-fn tick_ownership_hole_envelope() {
-    let ev = Shape::Staircase.packed1(HOLE_STAIR_DEPTH);
-    let id = Shape::IdSpine.packed_flagged(HOLE_ID_DEPTH, true);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered(
-        "tick_ownership_hole",
-        input,
-        &query_env::TICK_OWNERSHIP_HOLE,
-        || v.tick(&p),
+/// One public tick is applied *outside* the metered body first: on the
+/// once-ticked tree the fill is the identity (fill is idempotent, and
+/// a grow opens no fillable structure), so the metered `ticks(n)` is
+/// the pure grow branch, where registering `n` events grows the
+/// minimum tick count by exactly `n` — the fill branch instead
+/// collapses owned structure and moves `min_ticks` by a
+/// shape-dependent amount, which the committed small-count
+/// differentials pin byte-for-byte against iterated ticks.
+fn ticks_counters_wide(v: &Version, p: &Party, n: &before::Ticks) -> (u64, u64, u64) {
+    let mut v = v.clone();
+    v.tick(p);
+    let before_ticks = v.min_ticks();
+    meter::reset_scan_bits();
+    meter::reset_limb_ops();
+    suanpan::touch_meter::reset();
+    v.ticks(p, n.clone());
+    let counters = (
+        meter::scan_bits(),
+        meter::limb_ops(),
+        suanpan::touch_meter::touches(),
     );
-    drop(v);
-}
-
-/// Ticking the alternating spine under the scattered id (the
-/// alternating-ownership comb: owned fragments and absent gaps
-/// interleaved at every level, so every unowned region is a single
-/// leaf) stays within its envelope.
-///
-/// The comb is the region gate's worst case — the block scan can
-/// never engage — and the pin holds the gated walk to the per-leaf
-/// walk's own readings: a gate that costs anything when closed moves
-/// this envelope.
-#[test]
-fn tick_ownership_comb_envelope() {
-    let ev = Shape::AltSpine.packed1(DENSE_DEPTH);
-    let id = Shape::ScatteredId.packed1(COMB_FRAGMENTS);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered(
-        "tick_ownership_comb",
-        input,
-        &query_env::TICK_OWNERSHIP_COMB,
-        || v.tick(&p),
-    );
-    drop(v);
-}
-
-/// Ticking the collapse-hole pair (deep descending collapse ranges under
-/// left-full sites with absent siblings) stays within an envelope the
-/// per-leaf consuming max scan exceeds.
-///
-/// Each unit's fully-owned range is crossed exactly once, by the walk's
-/// consuming max scan at its descend arm, and the crossing must ride the
-/// block summary: the touch ceiling sits below what per-leaf register
-/// freight over the same ranges reads, and the scan column holds every
-/// folded bit still read.
-#[test]
-fn tick_collapse_hole_envelope() {
-    let (ev, id) = Shape::CollapseHole.packed_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered(
-        "tick_collapse_hole",
-        input,
-        &query_env::TICK_COLLAPSE_HOLE,
-        || v.tick(&p),
-    );
-    drop(v);
-}
-
-/// Ticking the copy-hole pair (deep descending absent-child ranges inside
-/// one covering pre-scan) stays within an envelope the per-leaf sub-scan
-/// mechanism exceeds.
-///
-/// Each unit's untouched range is copied once by the pre-scan, and the
-/// copy must ride the block summary — one net movement and one watermark
-/// emission per range, never a virtual emission per leaf: the touch
-/// ceiling sits below the per-leaf mechanism's reading, and the scan
-/// column holds every folded bit still read.
-#[test]
-fn tick_copy_hole_envelope() {
-    let (ev, id) = Shape::CopyHole.packed_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered("tick_copy_hole", input, &query_env::TICK_COPY_HOLE, || {
-        v.tick(&p)
-    });
-    drop(v);
-}
-
-/// Ticking the site-hole pair (deep descending collapse ranges under
-/// interior left-full sites inside one covering pre-scan) stays within an
-/// envelope the extremum-streaming block fold exceeds.
-///
-/// Each unit's collapse range is crossed exactly twice — once by the
-/// pre-scan's collapse skip, once by the walk's consuming max scan at the
-/// site's own consume — and the pre-scan's crossing owes the web only the
-/// range's net height movement: the touch ceiling sits below what a block
-/// fold that also streams the range's unread minimum reads over the same
-/// ranges, and the scan column holds every folded bit still read.
-#[test]
-fn tick_site_hole_envelope() {
-    let (ev, id) = Shape::SiteHole.packed_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered("tick_site_hole", input, &query_env::TICK_SITE_HOLE, || {
-        v.tick(&p)
-    });
-    drop(v);
-}
-
-/// Ticking the raise-hole pair (deep descending raised ranges under
-/// right-full sites) stays within an envelope the per-leaf consuming max
-/// scan exceeds.
-///
-/// Each unit's fully-owned right range is crossed exactly once, by the
-/// walk's consuming max scan at its ascend arm, and the crossing must
-/// ride the block summary: the touch ceiling sits below the per-leaf
-/// mechanism's reading, and the scan column holds every folded bit still
-/// read.
-#[test]
-fn tick_raise_hole_envelope() {
-    let (ev, id) = Shape::RaiseHole.packed_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered(
-        "tick_raise_hole",
-        input,
-        &query_env::TICK_RAISE_HOLE,
-        || v.tick(&p),
-    );
-    drop(v);
-}
-
-/// The fused multi-tick on the dense spine stays within its envelope:
-/// registering 512 events costs the single tick's walk and splice plus
-/// only the count's gamma-width boundary codes.
-#[test]
-fn ticks_dense_envelope() {
-    let p = Shape::Dense.packed1(DENSE_DEPTH);
-    let mut v = version_of(&p);
-    let seed = Party::seed();
-    query_metered(
-        "ticks_dense",
-        p.bytes.len(),
-        &query_env::TICKS_DENSE,
-        || v.ticks(&seed, TICKS_POINT_LO),
-    );
-    drop(v);
-}
-
-/// The fused multi-tick on the wide right-full chain stays within its
-/// envelope: the `+n` splice compounds at the same site the single
-/// tick's does, and the wide first payload is still touched O(1) times.
-#[test]
-fn ticks_nested_wide_envelope() {
-    let ev = Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
-    let id = Shape::NestedFullId.packed1(TICK_CROSS_SCALE);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered(
-        "ticks_nested_wide",
-        input,
-        &query_env::TICKS_NESTED_WIDE,
-        || v.ticks(&p, TICKS_POINT_LO),
-    );
-    drop(v);
-}
-
-/// The fused multi-tick on the wide memo chain stays within its
-/// envelope: the pre-scan's frame ledger behaves exactly as the single
-/// tick's, count notwithstanding.
-#[test]
-fn ticks_mirror_wide_envelope() {
-    let ev = Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
-    let id = Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE);
-    let mut v = version_of(&ev);
-    let p = party_of(&id);
-    let input = ev.bytes.len() + id.bytes.len();
-    query_metered(
-        "ticks_mirror_wide",
-        input,
-        &query_env::TICKS_MIRROR_WIDE,
-        || v.ticks(&p, TICKS_POINT_LO),
-    );
-    drop(v);
-}
-
-/// The flatness pin: `O(|v| + |p| + log n)` as a committed two-point
-/// check, not prose.
-///
-/// On each tick-designated family the whole cost
-/// movement from `ticks(512)` to `ticks(4096)` — three doublings — must
-/// sit inside the boundary codes' gamma-width delta band: the two codes
-/// that carry the count widen by 2 bits per doubling each, and no other
-/// column may move beyond a word of slack. An implementation iterating
-/// any fraction of the count moves every column by ~8x here and cannot
-/// hide in a constant; a dead meter reads zero movement AND a zero
-/// point, which the envelope rows' improvement tripwires already reject.
-#[test]
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
-fn ticks_flatness_holds_the_log_band() {
-    let cases: Vec<(&str, Version, Party)> = vec![
-        (
-            "dense",
-            version_of(&Shape::Dense.packed1(DENSE_DEPTH)),
-            Party::seed(),
-        ),
-        (
-            "nested-wide",
-            version_of(&Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
-            party_of(&Shape::NestedFullId.packed1(TICK_CROSS_SCALE)),
-        ),
-        (
-            "mirror-wide",
-            version_of(&Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
-            party_of(&Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE)),
-        ),
-    ];
-    for (name, v, p) in &cases {
-        let lo = ticks_counters(v, p, TICKS_POINT_LO);
-        let hi = ticks_counters(v, p, TICKS_POINT_HI);
-        let moved = [
-            ("scan", lo.0, hi.0, TICKS_FLATNESS_SCAN_BAND),
-            ("limb", lo.1, hi.1, TICKS_FLATNESS_LIMB_BAND),
-            ("touch", lo.2, hi.2, TICKS_FLATNESS_TOUCH_BAND),
-        ];
-        for (col, at_lo, at_hi, band) in moved {
-            let delta = at_hi.abs_diff(at_lo);
-            eprintln!("MEASURED ticks_flatness {name}/{col}: lo={at_lo} hi={at_hi} delta={delta}");
-            assert!(
-                delta <= band,
-                "{name}/{col}: ticks({}) -> ticks({}) moved {delta}                  (from {at_lo} to {at_hi}), outside the gamma-width band {band}",
-                TICKS_POINT_LO,
-                TICKS_POINT_HI,
-            );
-        }
-    }
-}
-
-/// One `ticks(n)` run's `(scan bits, limb ops, touches)` on fresh
-/// counters — the flatness pin's probe.
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
-fn ticks_counters(v: &Version, p: &Party, n: u64) -> (u64, u64, u64) {
-    let mut v = v.clone();
-    meter::reset_scan_bits();
-    meter::reset_limb_ops();
-    suanpan::touch_meter::reset();
-    v.ticks(p, n);
-    (
-        meter::scan_bits(),
-        meter::limb_ops(),
-        suanpan::touch_meter::touches(),
-    )
-}
-
-/// One `ticks(n)` run at an arbitrary-width count, on the operand's
-/// post-fill tree, with the exact `min_ticks` movement as the value
-/// leg.
-///
-/// One public tick is applied *outside* the metered body first: on the
-/// once-ticked tree the fill is the identity (fill is idempotent, and
-/// a grow opens no fillable structure), so the metered `ticks(n)` is
-/// the pure grow branch, where registering `n` events grows the
-/// minimum tick count by exactly `n` — the fill branch instead
-/// collapses owned structure and moves `min_ticks` by a
-/// shape-dependent amount, which the committed small-count
-/// differentials pin byte-for-byte against iterated ticks.
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
-fn ticks_counters_wide(v: &Version, p: &Party, n: &before::Ticks) -> (u64, u64, u64) {
-    let mut v = v.clone();
-    v.tick(p);
-    let before_ticks = v.min_ticks();
-    meter::reset_scan_bits();
-    meter::reset_limb_ops();
-    suanpan::touch_meter::reset();
-    v.ticks(p, n.clone());
-    let counters = (
-        meter::scan_bits(),
-        meter::limb_ops(),
-        suanpan::touch_meter::touches(),
-    );
-    assert_eq!(
-        v.min_ticks(),
-        before_ticks + n.clone(),
-        "a grow-branch ticks(n) must grow the minimum tick count by exactly n"
-    );
-    counters
+    assert_eq!(
+        v.min_ticks(),
+        before_ticks + n.clone(),
+        "a grow-branch ticks(n) must grow the minimum tick count by exactly n"
+    );
+    counters
 }
 
 /// The wide-count pin's count width in bits (the second wide point
```

<!-- annotation -->
> **envelopes-a-8** (4), line 692:
>
> The dense self-comparison ported from the kernel row to `partial_cmp` on a byte-equal, buffer-distinct copy (the door runs the full sweep; `PartialOrd` has no equality short-circuit). The twelve tick and multi-tick scenarios that followed the dense join here moved whole to the tick section beside `query_env`.

<!-- annotation -->
> **envelopes-a-4** (4), line 712:
>
> Deletion: the tick and multi-tick scenarios stood here, between the ticks flatness bands; they moved whole to the tick section beside `query_env`. The bands themselves are untouched.

<!-- annotation -->
> **fresh-eyes-D2** (108), line 738:
>
> The twelve `cfg(all(limb-meter, scan-meter))` gates (eleven on the ticks flatness bands, their probes, and their constants; one on the `fold_stagger` band module) are deleted for the same reason. The band modules' single-feature `cfg(limb-meter)` gates are equally dead; they are the suites lane's.

<a id="hunk-12"></a>
### crates/before/tests/meter.rs `@@ -830,7 +838,6 @@ fn ticks_counters_wide(v: &Version, p: &Party, n: &before::Ticks) -> (u64, u64,`

```diff
@@ -830,7 +838,6 @@ fn ticks_counters_wide(v: &Version, p: &Party, n: &before::Ticks) -> (u64, u64,
 /// law the wide points' scan spans track). Judging two points in one
 /// regime keeps the ratio band tight; a probe straddling the knee
 /// legitimately reads up to ×3 without any superlinearity.
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
 const TICKS_WIDE_COUNT_BITS: usize = 8_192;
 
 /// The count-attributable growth bound: doubling the count's width may
```

<!-- annotation -->
> **fresh-eyes-D2** (108), line 841:
>
> Deletion: an always-true `cfg(all(limb-meter, scan-meter))` gate stood above this probe.

<a id="hunk-13"></a>
### crates/before/tests/meter.rs `@@ -852,11 +859,9 @@ const TICKS_WIDE_COUNT_BITS: usize = 8_192;`

```diff
@@ -852,11 +859,9 @@ const TICKS_WIDE_COUNT_BITS: usize = 8_192;
 /// regimes are width-linear, so the ratio band holds across all
 /// three. The touch span carries no count dependence anywhere: the
 /// count's arithmetic lives on `Base`, never the accumulator.
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
 const TICKS_WIDE_GROWTH_NUM: u64 = 5;
 
 /// See [`TICKS_WIDE_GROWTH_NUM`]: the ratio denominator.
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
 const TICKS_WIDE_GROWTH_DEN: u64 = 2;
 
 /// The wide-count flatness pin: `ticks(n)` stays width-linear in the
```

<!-- annotation -->
> **fresh-eyes-D2** (108), line 862:
>
> Deletion: the always-true gates above this constant and the one before it.

<a id="hunk-14"></a>
### crates/before/tests/meter.rs `@@ -874,7 +879,6 @@ const TICKS_WIDE_GROWTH_DEN: u64 = 2;`

```diff
@@ -874,7 +879,6 @@ const TICKS_WIDE_GROWTH_DEN: u64 = 2;
 /// equal to `n` at every point, so the wide registration is proven to
 /// have happened before any cost is judged.
 #[test]
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
 fn ticks_wide_count_flatness_holds_the_width_band() {
     use dashu_int::UBig;
     let wide = |bits: usize| -> before::Ticks {
```

<!-- annotation -->
> **fresh-eyes-D2** (108), line 882:
>
> Deletion: the always-true gate above this band.

<a id="hunk-15"></a>
### crates/before/tests/meter.rs `@@ -943,7 +947,6 @@ fn ticks_wide_count_flatness_holds_the_width_band() {`

```diff
@@ -943,7 +947,6 @@ fn ticks_wide_count_flatness_holds_the_width_band() {
 /// legible against the band.
 const TICKS_POINT_LO: u64 = 512;
 /// See [`TICKS_POINT_LO`].
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
 const TICKS_POINT_HI: u64 = 4_096;
 /// The scan movement band: up to two count-carrying codes x 2 bits per
 /// doubling x 3 doublings.
```

<!-- annotation -->
> **fresh-eyes-D2** (108), line 950:
>
> Deletion: the always-true gate above this band.

<a id="hunk-16"></a>
### crates/before/tests/meter.rs `@@ -951,16 +954,13 @@ const TICKS_POINT_HI: u64 = 4_096;`

```diff
@@ -951,16 +954,13 @@ const TICKS_POINT_HI: u64 = 4_096;
 /// One code carries the count on the committed families; the second
 /// code's budget covers operand shapes where the successor repair
 /// carries it too.
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
 const TICKS_FLATNESS_SCAN_BAND: u64 = 12;
 /// The limb movement band: the count's arithmetic stays inside one
 /// digit across the band; a word of slack covers a digit-boundary
 /// crossing.
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
 const TICKS_FLATNESS_LIMB_BAND: u64 = 8;
 /// The touch movement band: see the limb band (the count's arithmetic
 /// never lands on the accumulator).
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
 const TICKS_FLATNESS_TOUCH_BAND: u64 = 8;
 
 // ─── bigroot scenarios ──────────────────────────────────────────────────────
```

<!-- annotation -->
> **fresh-eyes-D2** (108), line 957:
>
> Deletion: the always-true gates above the flatness pin's point and band constants.

<a id="hunk-17"></a>
### crates/before/tests/meter.rs `@@ -980,9 +980,9 @@ fn decode_bigroot_envelope() {`

```diff
@@ -980,9 +980,9 @@ fn decode_bigroot_envelope() {
     drop(v);
 }
 
-/// Comparing bigroot against the empty version stays within its envelope
-/// (today the worst amplifier: per-frame owned path sums, quadratic in the
-/// root magnitude × depth).
+/// Comparing bigroot against the empty version stays within its envelope:
+/// the difference accumulator absorbs the wide first height once, paid by
+/// its own code, and every later delta is small.
 #[test]
 fn cmp_bigroot_envelope() {
     let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
```

<!-- annotation -->
> **envelopes-a-8** (4), line 984:
>
> `cmp_bigroot`'s doc restated from the row's mechanism; the sentence "today the worst amplifier: per-frame owned path sums, quadratic in the root magnitude × depth" described a replaced implementation (ruling 45).

<a id="hunk-18"></a>
### crates/before/tests/meter.rs `@@ -990,11 +990,17 @@ fn cmp_bigroot_envelope() {`

```diff
@@ -990,11 +990,17 @@ fn cmp_bigroot_envelope() {
     let r = metered("cmp_bigroot", p.bytes.len(), &envelope::CMP_BIGROOT, || {
         v.partial_cmp(&Version::new())
     });
-    consumed(r);
+    assert_eq!(
+        r,
+        Some(Ordering::Greater),
+        "bigroot strictly dominates the empty version"
+    );
 }
 
-/// Joining bigroot with a one-tick version stays within its envelope (the
-/// same per-frame path-sum amplification on the combine path).
+/// Joining bigroot with a one-tick version stays within its envelope: the
+/// wide first height is absorbed once, paid by its own code, and every
+/// later delta is small. The join is the emit kernel's stream, byte for
+/// byte.
 #[test]
 fn join_bigroot_envelope() {
     let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
```

<!-- annotation -->
> **envelopes-a-8** (4), line 996:
>
> The retired `SKYLINE_CMP_BIGROOT` twin's verdict leg moves onto the door row in place of `consumed(r)`; its doc is restated from the row's mechanism, dropping the "today the worst amplifier" sentence about a replaced implementation.

<a id="hunk-19"></a>
### crates/before/tests/meter.rs `@@ -1006,7 +1012,11 @@ fn join_bigroot_envelope() {`

```diff
@@ -1006,7 +1012,11 @@ fn join_bigroot_envelope() {
         &envelope::JOIN_BIGROOT,
         || &v | &one,
     );
-    drop(joined);
+    assert_eq!(
+        meter::skyline::encode(&joined),
+        emitted(meter::skyline::emit::join, &v, &one),
+        "the public join must be the emit kernel's stream"
+    );
 }
 
 // ─── hugeleaf scenarios ─────────────────────────────────────────────────────
```

<!-- annotation -->
> **envelopes-a-8** (4), line 1012:
>
> The retired `SKYLINE_JOIN_BIGROOT` twin's byte-identity leg (the public join's stream equals the emit kernel's) moves onto the door row in place of `drop(joined)`.

<a id="hunk-20"></a>
### crates/before/tests/meter.rs `@@ -1046,6 +1056,27 @@ fn join_hugeleaf_envelope() {`

```diff
@@ -1046,6 +1056,27 @@ fn join_hugeleaf_envelope() {
     drop(joined);
 }
 
+/// Joining the dense spine with a dominating flat operand stays within its
+/// envelope.
+///
+/// The whole output collapses to one leaf through 125k absorb steps
+/// around a held 125k-bit code, linear only because absorb never moves
+/// the held code. The join is the flat operand, byte for byte.
+#[test]
+fn join_absorb_envelope() {
+    let p = Shape::Dense.packed1(DENSE_DEPTH);
+    let q = Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS);
+    let v = version_of(&p);
+    let flat = version_of(&q);
+    let joined = metered(
+        "join_absorb",
+        p.bytes.len() + q.bytes.len(),
+        &envelope::JOIN_ABSORB,
+        || &v | &flat,
+    );
+    assert_eq!(joined, flat, "a dominating flat operand is the whole join");
+}
+
 // ─── boundary comb scenarios ────────────────────────────────────────────────
 
 /// Decoding the boundary comb stays within its envelope (every carry-cliff
```

<!-- annotation -->
> **envelopes-a-8** (4), line 1066:
>
> `JOIN_ABSORB` ported to the public join of the dense spine with the flat hugeleaf operand; the closed-form leg (the join is the flat operand) is the retired kernel test's own assertion. Placed in the hugeleaf section, whose operand it shares.

<a id="hunk-21"></a>
### crates/before/tests/meter.rs `@@ -1075,12 +1106,20 @@ fn cmp_cliff_envelope() {`

```diff
@@ -1075,12 +1106,20 @@ fn cmp_cliff_envelope() {
     let r = metered("cmp_cliff", p.bytes.len(), &envelope::CMP_CLIFF, || {
         v.partial_cmp(&Version::new())
     });
-    consumed(r);
+    assert_eq!(
+        r,
+        Some(Ordering::Greater),
+        "the comb strictly dominates the empty version"
+    );
 }
 
 /// Joining the boundary comb with a one-tick version stays within its
-/// envelope (the emit path re-codes every tooth magnitude, each paid for by
-/// a comparably-wide input code).
+/// envelope.
+///
+/// The emit path re-codes every tooth magnitude, each paid for by a
+/// comparably-wide input code, and every 3-bit `±1` delta re-emits
+/// across the `2^k` carry boundary at amortized O(1). The join is the
+/// emit kernel's stream, byte for byte.
 #[test]
 fn join_cliff_envelope() {
     let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
```

<!-- annotation -->
> **envelopes-a-8** (4), line 1106:
>
> The retired `SKYLINE_CMP_CLIFF` twin's verdict leg on the door row, in place of `consumed(r)`.

<a id="hunk-22"></a>
### crates/before/tests/meter.rs `@@ -1089,7 +1128,140 @@ fn join_cliff_envelope() {`

```diff
@@ -1089,7 +1128,140 @@ fn join_cliff_envelope() {
     let joined = metered("join_cliff", p.bytes.len(), &envelope::JOIN_CLIFF, || {
         &v | &one
     });
-    drop(joined);
+    assert_eq!(
+        meter::skyline::encode(&joined),
+        emitted(meter::skyline::emit::join, &v, &one),
+        "the public join must be the emit kernel's stream"
+    );
+}
+
+/// Meeting the boundary comb with a one-tick version stays within its
+/// envelope.
+///
+/// The pointwise minimum clamps every tooth to the flat operand's height
+/// while every comb delta still crosses the carry boundary in the
+/// accumulator. The meet is the emit kernel's stream, byte for byte.
+#[test]
+fn meet_cliff_envelope() {
+    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
+    let v = version_of(&p);
+    let one = Version::try_from(1u64).expect("a one-tick version is valid");
+    let met = metered("meet_cliff", p.bytes.len(), &envelope::MEET_CLIFF, || {
+        &v & &one
+    });
+    assert_eq!(
+        meter::skyline::encode(&met),
+        emitted(meter::skyline::emit::meet, &v, &one),
+        "the public meet must be the emit kernel's stream"
+    );
+}
+
+// ─── wide-tooth comb scenarios ────────────────────────────────────────────
+//
+// The wide-tooth comb: every skyline delta is a `±2^w` operand wider than
+// any machine word, still oscillating across the `2^k` cliff, so limb
+// work must stay linear per input bit at every tooth width.
+
+/// Decoding the wide-tooth comb stays within its envelope: each wide
+/// delta's limb work is paid by its own zigzag code, and the once-adopted
+/// buffer prices the wide payloads.
+#[test]
+fn decode_wide_tooth_envelope() {
+    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
+    let wire = version_of(&p).encode();
+    let v = metered(
+        "decode_wide_tooth",
+        wire.len(),
+        &envelope::DECODE_WIDE_TOOTH,
+        || Version::decode(&wire[..]).expect("a stored version's wire bytes decode"),
+    );
+    drop(v);
+}
+
+/// Comparing the wide-tooth comb against the empty version stays within
+/// its envelope: each `±2^w` delta is a wide operand paid by its own
+/// zigzag code.
+#[test]
+fn cmp_wide_tooth_envelope() {
+    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
+    let v = version_of(&p);
+    let r = metered(
+        "cmp_wide_tooth",
+        p.bytes.len(),
+        &envelope::CMP_WIDE_TOOTH,
+        || v.partial_cmp(&Version::new()),
+    );
+    assert_eq!(
+        r,
+        Some(Ordering::Greater),
+        "the wide-tooth comb strictly dominates the empty version"
+    );
+}
+
+/// Joining the wide-tooth comb with a one-tick version stays within its
+/// envelope.
+///
+/// Each `±2^w` delta is a wide operand re-coded into the output, paid by
+/// its own zigzag code. The join is the emit kernel's stream, byte for
+/// byte.
+#[test]
+fn join_wide_tooth_envelope() {
+    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
+    let v = version_of(&p);
+    let one = Version::try_from(1u64).expect("a one-tick version is valid");
+    let joined = metered(
+        "join_wide_tooth",
+        p.bytes.len(),
+        &envelope::JOIN_WIDE_TOOTH,
+        || &v | &one,
+    );
+    assert_eq!(
+        meter::skyline::encode(&joined),
+        emitted(meter::skyline::emit::join, &v, &one),
+        "the public join must be the emit kernel's stream"
+    );
+}
+
+/// Meeting the wide-tooth comb with a one-tick version stays within its
+/// envelope.
+///
+/// Wide deltas are folded but never re-emitted (the flat side wins
+/// everywhere), so the collapse discipline runs at spilled operand
+/// widths. The meet is the emit kernel's stream, byte for byte.
+#[test]
+fn meet_wide_tooth_envelope() {
+    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
+    let v = version_of(&p);
+    let one = Version::try_from(1u64).expect("a one-tick version is valid");
+    let met = metered(
+        "meet_wide_tooth",
+        p.bytes.len(),
+        &envelope::MEET_WIDE_TOOTH,
+        || &v & &one,
+    );
+    assert_eq!(
+        meter::skyline::encode(&met),
+        emitted(meter::skyline::emit::meet, &v, &one),
+        "the public meet must be the emit kernel's stream"
+    );
+}
+
+// ─── alternating spine scenarios ──────────────────────────────────────────
+
+/// Decoding the alternating-binary spine stays within its envelope: the
+/// direction of descent flips every level, and per-level state stays two
+/// bits, not a frame.
+#[test]
+fn decode_alt_spine_envelope() {
+    let p = Shape::AltSpine.packed1(DENSE_DEPTH);
+    let wire = version_of(&p).encode();
+    let v = metered(
+        "decode_alt_spine",
+        wire.len(),
+        &envelope::DECODE_ALT_SPINE,
+        || Version::decode(&wire[..]).expect("a stored version's wire bytes decode"),
+    );
+    drop(v);
 }
 
 // ─── rank scenarios ─────────────────────────────────────────────────────────
```

<!-- annotation -->
> **envelopes-a-8** (4), line 1145:
>
> `join_cliff` gains the retired twin's byte-identity leg; `MEET_CLIFF` is ported to the public meet. My first closed-form leg (the meet is the one-tick leaf) was wrong: the comb has zero-height leaves, so the meet keeps the comb's shape clamped to height 1; the leg is the kernel byte-identity the retired twin carried, and the doc says what the meet is.

<!-- annotation -->
> **envelopes-a-8** (4), line 1159:
>
> The wide-tooth comb gets a section of its own for the four door rows ported here (`DECODE_WIDE_TOOTH`, `CMP_WIDE_TOOTH`, `JOIN_WIDE_TOOTH`, `MEET_WIDE_TOOTH`) and the alternating spine one for `DECODE_ALT_SPINE`; the kernel sections they came from are gone.

<a id="hunk-23"></a>
### crates/before/tests/meter.rs `@@ -1111,199 +1283,54 @@ fn join_cliff_envelope() {`

```diff
@@ -1111,199 +1283,54 @@ fn join_cliff_envelope() {
 // the raw-accumulator Sum (one normalization at the end, where a
 // per-summand renormalization reads magnitude-quadratic).
 
-/// One touch-priced scenario's pinned ceilings, asserted when the
-/// `limb-meter` feature is lit.
-///
-/// [`Envelope`]'s three columns plus accumulator digit touches — the
-/// rank folds' and the tick walk's own cost currency: wide content
-/// moves through `Accumulator`s that the heap and limb columns cannot see.
-struct TouchEnvelope {
-    /// Peak heap delta over the scenario body, in bytes.
-    peak_heap: usize,
-    /// Stack segments grown during the scenario body.
-    segments: u64,
-    /// Big-integer limb operations counted during the scenario body.
-    #[cfg(feature = "limb-meter")]
-    limb_ops: u64,
-    /// Accumulator digit touches counted during the scenario body.
-    #[cfg(feature = "limb-meter")]
-    touches: u64,
-    /// Improvement tripwire under the limb column: measured ×0.75, per
-    /// the file doc's tripwire convention.
-    #[cfg(feature = "limb-meter")]
-    limb_floor: u64,
-    /// Improvement tripwire under the touch column: measured ×0.75, per
-    /// the file doc's tripwire convention.
-    ///
-    /// A touch reading below it is a >25% drop from the pinned reading —
-    /// attribute it, re-pinning an honest improvement or curing a dead
-    /// meter, without which every touch ceiling above would hold
-    /// vacuously.
-    #[cfg(feature = "limb-meter")]
-    touch_floor: u64,
-}
-
-/// Build a [`TouchEnvelope`] from the four pinned columns and the two
-/// improvement tripwires.
-///
-/// The limb and touch columns are carried only when the `limb-meter`
-/// feature compiles their counters in; the leading underscores keep the
-/// parameters warning-free in the other configuration.
-const fn touch_envelope(
-    peak_heap: usize,
-    segments: u64,
-    _limb_ops: u64,
-    _touches: u64,
-    _limb_floor: u64,
-    _touch_floor: u64,
-) -> TouchEnvelope {
-    TouchEnvelope {
-        peak_heap,
-        segments,
-        #[cfg(feature = "limb-meter")]
-        limb_ops: _limb_ops,
-        #[cfg(feature = "limb-meter")]
-        touches: _touches,
-        #[cfg(feature = "limb-meter")]
-        limb_floor: _limb_floor,
-        #[cfg(feature = "limb-meter")]
-        touch_floor: _touch_floor,
-    }
-}
-
-// The touch-priced envelope table (the rank rows): pinned ceiling =
-// measured ×1.25, rounded up (aarch64-apple-darwin, dev profile, three
-// identical runs), and only ever tightened: where a remeasure rises while
-// staying inside an existing ceiling (the spilled-numerator heap cells,
-// which carry the backend's `len/8 + 2` words of growth headroom per heap
-// allocation), the older, tighter ceiling stands. The trailing comment on
-// each line states the mechanism that prices the row; the measurements of
-// record — and every re-pin's movement and attribution — live in the pin
-// commits (`git log -S` the constant). Re-pin by rerunning under
-// `--no-capture` with `--all-features` and reading the MEASURED lines.
-// The limb floor column is the measured value ×0.75, rounded down (the
-// file doc's improvement-tripwire convention).
-// A re-denomination of a column — the same work newly counted at the
-// metered seam (`Base::trailing_zeros`, widening shifts) — is a
-// sanctioned rise under the tightening rule, recorded in its pin commit,
-// never a weakening.
+// Pins per the file doc's convention; each row's trailing comment states
+// the mechanism that prices it.
 #[rustfmt::skip]
 mod rank_env {
-    use super::{touch_envelope, TouchEnvelope};
-    //                                                             peak heap, segments,    limb ops, touches, limb floor, touch floor
-    pub const RANK_DENSE: TouchEnvelope = touch_envelope(30_720, 0, 4, 7, 2, 3); // the depth control: word-scale numerators fold in the accumulator's quick register, so the work columns sit near zero and the heap is the at-rest form
-    pub const RANK_BIGROOT: TouchEnvelope = touch_envelope(72_005, 0, 2_739, 8_993, 1_643, 5_395); // the wide-magnitude control: one root-wide decode and one root-wide fold; the segment feed opens only at the first freeze
-    pub const RANK_HARMONIC: TouchEnvelope = touch_envelope(52_500, 0, 2_562, 248_285, 1_536, 148_971); // the separating family: each level's one-leaf sibling lands at the exponent gap, so touches stay linear in depth and no accumulated numerator is re-shifted
-    pub const RANK_PAIR_MISMATCH: TouchEnvelope = touch_envelope(     234_400,        0,      87_910,      0, 52_746, 0); // class-first cmp decides order in O(1); the limb record is checked_sub's and add's mandatory output content plus the metered exponent-alignment shifts
-    pub const RANK_SUM_MIXED: TouchEnvelope     = touch_envelope(      78_140,        0,       9_769, 22_268, 5_861, 13_360); // the raw accumulator: digit-routed summands, one normalization at the end
+    use super::{band, envelope, Envelope};
+    pub const RANK_DENSE: Envelope         = envelope( 30_720,           band(4, 2),             band(7, 3), band(937_515, 562_509)); // the depth control: word-scale numerators fold in the accumulator's quick register, so the work columns sit near zero and the heap is the at-rest form
+    pub const RANK_BIGROOT: Envelope       = envelope( 67_145,   band(2_739, 1_643),     band(8_993, 5_395), band(275_023, 165_013)); // the wide-magnitude control: one root-wide decode and one root-wide fold; the segment feed opens only at the first freeze
+    pub const RANK_HARMONIC: Envelope      = envelope( 52_500,   band(2_562, 1_536), band(248_285, 148_971), band(491_530, 294_918)); // the separating family: each level's one-leaf sibling lands at the exponent gap, so touches stay linear in depth and no accumulated numerator is re-shifted
+    pub const RANK_PAIR_MISMATCH: Envelope = envelope(234_400, band(87_910, 52_746),             band(0, 0),             band(0, 0)); // class-first cmp decides order in O(1); the limb record is checked_sub's and add's mandatory output content plus the metered exponent-alignment shifts
+    pub const RANK_SUM_MIXED: Envelope     = envelope( 78_140,   band(9_769, 5_861),   band(22_268, 13_360),             band(0, 0)); // the raw accumulator: digit-routed summands, one normalization at the end
 }
 
-/// Run one touch-priced scenario body under all four meters and assert
-/// its envelope, both improvement tripwires included.
+/// The rank fold on the dense spine stays within its envelope.
 ///
-/// [`metered`]'s harness plus the accumulator touch column; prints the
-/// measured numbers so re-pinning never requires editing the harness.
-fn touch_metered<R>(
-    name: &str,
-    input_bytes: usize,
-    env: &TouchEnvelope,
-    f: impl FnOnce() -> R,
-) -> R {
-    meter::reset_stack_segments();
-    #[cfg(feature = "limb-meter")]
-    meter::reset_limb_ops();
-    #[cfg(feature = "limb-meter")]
-    suanpan::touch_meter::reset();
-    HEAP.reset_peak_usage();
-    let baseline = HEAP.current_usage();
-    let r = f();
-    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
-    let segments = meter::stack_segments();
-    #[cfg(feature = "limb-meter")]
-    let limb_ops = meter::limb_ops();
-    #[cfg(feature = "limb-meter")]
-    let touches = suanpan::touch_meter::touches();
-    #[cfg(feature = "limb-meter")]
-    eprintln!(
-        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments} limb_ops={limb_ops} touches={touches}"
-    );
-    #[cfg(not(feature = "limb-meter"))]
-    eprintln!(
-        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments}"
-    );
-    assert!(
-        peak_heap <= env.peak_heap,
-        "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B): {ISOLATION_NOTE}",
-        env.peak_heap,
-    );
-    assert!(
-        segments <= env.segments,
-        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.segments,
-    );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        limb_ops <= env.limb_ops,
-        "{name}: {limb_ops} limb operations exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.limb_ops,
-    );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        touches <= env.touches,
-        "{name}: {touches} accumulator digit touches exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.touches,
-    );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        limb_ops >= env.limb_floor,
-        "{name}: limb counter reads {limb_ops}, below the {} improvement \
-         tripwire (measured x0.75): attribute the drop — an honest \
-         improvement re-pins the band; a dead meter is the bypass this \
-         column exists to catch",
-        env.limb_floor,
-    );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        touches >= env.touch_floor,
-        "{name}: touch counter reads {touches}, below the {} improvement \
-         tripwire (measured x0.75): attribute the drop — an honest \
-         improvement re-pins the band; a dead meter is the bypass this \
-         column exists to catch",
-        env.touch_floor,
-    );
-    r
-}
-
-/// The rank fold on the dense spine stays within its envelope (the
-/// control: the spine's numerator stays one bit wide, so the fold's
-/// per-level shifts are word-scale and the walk is linear).
+/// The control: the spine's numerator stays one bit wide, so the fold's
+/// per-level shifts are word-scale and the walk is linear. The rank is
+/// the skyline kernel's, exactly.
 #[test]
 fn rank_dense_envelope() {
     let p = Shape::Dense.packed1(DENSE_DEPTH);
     let v = version_of(&p);
-    let r = touch_metered("rank_dense", p.bytes.len(), &rank_env::RANK_DENSE, || {
+    let r = metered("rank_dense", p.bytes.len(), &rank_env::RANK_DENSE, || {
         v.rank()
     });
-    consumed(r);
+    assert_eq!(r, kernel_rank(&v), "the public rank must be the kernel's");
 }
 
-/// The rank fold on the bigroot spine stays within its envelope (the
-/// wide-magnitude control: one root-wide shift, then word-scale work).
+/// The rank fold on the bigroot spine stays within its envelope.
+///
+/// The wide-magnitude control: the first leaf's magnitude seeds the
+/// frozen component and is read once, in the closing shifted add. The
+/// rank is the skyline kernel's, exactly.
 #[test]
 fn rank_bigroot_envelope() {
     let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
     let v = version_of(&p);
-    let r = touch_metered(
+    let r = metered(
         "rank_bigroot",
         p.bytes.len(),
         &rank_env::RANK_BIGROOT,
         || v.rank(),
     );
-    consumed(r);
+    assert_eq!(r, kernel_rank(&v), "the public rank must be the kernel's");
 }
 
 /// The rank fold on the harmonic spine stays within its envelope — the
-/// fold's separating family, pinned linear.
+/// fold's separating family, pinned linear. The rank is the skyline
+/// kernel's, exactly.
 ///
 /// The accumulated numerator is as wide as the depth already walked at
 /// every level, and the digit-routed merge folds each level's one-leaf
```

<!-- annotation -->
> **envelopes-b-8** (4), line 1289:
>
> `TouchEnvelope`, `touch_envelope`, the rank table's own restatement of the pin convention, and `touch_metered` are deleted; the table cites the file doc and its rows are the unified `envelope(...)` form with the same numbers. The re-denomination sanction that lived only here moved to the file doc.

<!-- annotation -->
> **envelopes-a-4** (4), line 1307:
>
> Call site renamed from `touch_metered` to `metered`; no reading moved. The same holds for every `metered(` call-site hunk in this file.

<!-- annotation -->
> **envelopes-a-4** (4), line 1323:
>
> Call site renamed from `touch_metered` to `metered`, reflowed by rustfmt; no reading moved.

<!-- annotation -->
> **envelopes-a-8** (108), line 1292:
>
> `SKYLINE_RANK_{DENSE,BIGROOT,HARMONIC}` retire as twins of the public `RANK_*` rows (identical readings on every column; `Version::rank` is the kernel on the live skyline). `RANK_BIGROOT`'s heap ceiling carries the twin's tighter 67_145 (reading 57_604) in place of 72_005, the rule the ported door rows follow; the other cells were already equal.

<!-- annotation -->
> **envelopes-a-8** (108), line 1316:
>
> The door docs absorb the retired kernel docs' mechanism sentences.

<!-- annotation -->
> **envelopes-a-8** (105), line 1300:
>
> Summary split after its first sentence for doclint's limit; likewise the bigroot rank doc.

<a id="hunk-24"></a>
### crates/before/tests/meter.rs `@@ -1312,13 +1339,13 @@ fn rank_bigroot_envelope() {`

```diff
@@ -1312,13 +1339,13 @@ fn rank_bigroot_envelope() {
 fn rank_harmonic_envelope() {
     let p = Shape::Harmonic.packed1(RANK_HARMONIC_DEPTH);
     let v = version_of(&p);
-    let r = touch_metered(
+    let r = metered(
         "rank_harmonic",
         p.bytes.len(),
         &rank_env::RANK_HARMONIC,
         || v.rank(),
     );
-    consumed(r);
+    assert_eq!(r, kernel_rank(&v), "the public rank must be the kernel's");
 }
 
 /// `Rank::cmp` + `checked_sub` + `+` on the mismatched-exponent pair stay
```

<!-- annotation -->
> **envelopes-a-4** (4), line 1343:
>
> Call site renamed from `touch_metered` to `metered`; no reading moved.

<a id="hunk-25"></a>
### crates/before/tests/meter.rs `@@ -1341,7 +1368,7 @@ fn rank_pair_mismatch_envelope() {`

```diff
@@ -1341,7 +1368,7 @@ fn rank_pair_mismatch_envelope() {
     // Informational denominator: the pair's value content in bytes
     // (numerator bits + exponent, over eight).
     let content_bytes = RANK_PAIR_DEPTH / 8 + 1;
-    let r = touch_metered(
+    let r = metered(
         "rank_pair_mismatch",
         content_bytes,
         &rank_env::RANK_PAIR_MISMATCH,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 1372:
>
> Call site renamed from `touch_metered` to `metered`; no reading moved.

<a id="hunk-26"></a>
### crates/before/tests/meter.rs `@@ -1386,7 +1413,7 @@ fn rank_sum_mixed_envelope() {`

```diff
@@ -1386,7 +1413,7 @@ fn rank_sum_mixed_envelope() {
         .collect();
     let content_bytes = RANK_SUM_EXP_DEPTH / 8 + RANK_SUM_COUNT;
     let ranks: Vec<before::Rank> = std::iter::once(high).chain(ones).collect();
-    let r = touch_metered(
+    let r = metered(
         "rank_sum_mixed",
         content_bytes,
         &rank_env::RANK_SUM_MIXED,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 1417:
>
> Call site renamed from `touch_metered` to `metered`; no reading moved.

<a id="hunk-27"></a>
### crates/before/tests/meter.rs `@@ -1397,15 +1424,12 @@ fn rank_sum_mixed_envelope() {`

```diff
@@ -1397,15 +1424,12 @@ fn rank_sum_mixed_envelope() {
 
 // ─── skyline codec scenarios ────────────────────────────────────────────────
 //
-// The skyline validator and decoder over the adversarial event families,
-// with the stream transcoded outside measurement. The validator rows pin
-// the validator's transient — ~2 bits of open-ancestor stack per
-// level plus the cliff-free accumulator — denominated against skyline
-// input bytes; the decoder rows add the transcode back to the packed
-// form, whose materialized heights and floors are priced by that packed
-// output (on the comb it is quadratically larger than the skyline input,
-// so no transcode can be skyline-linear; the validator is the piece that
-// carries the wire-bit-linear claim).
+// The skyline validator over the adversarial event families, with the
+// stream transcoded outside measurement. The rows pin the validator's
+// transient (~2 bits of open-ancestor stack per level plus the
+// cliff-free accumulator), denominated against skyline input bytes: the
+// validator is the piece that carries the wire-bit-linear claim, and the
+// public decode rows price the wrap beside it.
 
 /// The skyline stream of a packed family shape, built outside measurement.
 fn skyline_of(p: &meter::Packed) -> meter::skyline::BitsBuf {
```

<!-- annotation -->
> **envelopes-a-11** (4), line 1427:
>
> The codec section no longer describes decoder rows that transcode into "the packed form", a coding the stored form no longer is.

<!-- annotation -->
> **envelopes-a-11** (4), line 1432:
>
> The codec section no longer describes decoder rows transcoding into "the packed form" (a coding the stored form is not); it states what the validate rows pin and that the public decode rows price the wrap.

<a id="hunk-28"></a>
### crates/before/tests/meter.rs `@@ -1415,12 +1439,11 @@ fn skyline_of(p: &meter::Packed) -> meter::skyline::BitsBuf {`

```diff
@@ -1415,12 +1439,11 @@ fn skyline_of(p: &meter::Packed) -> meter::skyline::BitsBuf {
 /// The skyline validator on the dense spine stays within its envelope.
 ///
 /// The transient is ~2 bits per open ancestor (bit stack plus
-/// reallocation growth) — bits per level, not frames — with zero grown
-/// segments.
+/// reallocation growth): bits per level, not frames.
 #[test]
 fn skyline_validate_dense_envelope() {
     let enc = skyline_of(&Shape::Dense.packed1(DENSE_DEPTH));
-    let r = sweep_metered(
+    let r = metered(
         "skyline_validate_dense",
         enc.as_raw_slice().len(),
         &envelope::SKYLINE_VALIDATE_DENSE,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 1447:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<a id="hunk-29"></a>
### crates/before/tests/meter.rs `@@ -1438,7 +1461,7 @@ fn skyline_validate_dense_envelope() {`

```diff
@@ -1438,7 +1461,7 @@ fn skyline_validate_dense_envelope() {
 #[test]
 fn skyline_validate_cliff_envelope() {
     let enc = skyline_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
-    let r = sweep_metered(
+    let r = metered(
         "skyline_validate_cliff",
         enc.as_raw_slice().len(),
         &envelope::SKYLINE_VALIDATE_CLIFF,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 1465:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<a id="hunk-30"></a>
### crates/before/tests/meter.rs `@@ -1454,7 +1477,7 @@ fn skyline_validate_cliff_envelope() {`

```diff
@@ -1454,7 +1477,7 @@ fn skyline_validate_cliff_envelope() {
 fn skyline_validate_wide_tooth_envelope() {
     let enc =
         skyline_of(&Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE));
-    let r = sweep_metered(
+    let r = metered(
         "skyline_validate_wide_tooth",
         enc.as_raw_slice().len(),
         &envelope::SKYLINE_VALIDATE_WIDE_TOOTH,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 1481:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<a id="hunk-31"></a>
### crates/before/tests/meter.rs `@@ -1472,7 +1495,7 @@ fn skyline_validate_wide_tooth_envelope() {`

```diff
@@ -1472,7 +1495,7 @@ fn skyline_validate_wide_tooth_envelope() {
 #[test]
 fn skyline_validate_hugeleaf_envelope() {
     let enc = skyline_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
-    let r = sweep_metered(
+    let r = metered(
         "skyline_validate_hugeleaf",
         enc.as_raw_slice().len(),
         &envelope::SKYLINE_VALIDATE_HUGELEAF,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 1499:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<a id="hunk-32"></a>
### crates/before/tests/meter.rs `@@ -1488,7 +1511,7 @@ fn skyline_validate_hugeleaf_envelope() {`

```diff
@@ -1488,7 +1511,7 @@ fn skyline_validate_hugeleaf_envelope() {
 #[test]
 fn skyline_validate_alt_spine_envelope() {
     let enc = skyline_of(&Shape::AltSpine.packed1(DENSE_DEPTH));
-    let r = sweep_metered(
+    let r = metered(
         "skyline_validate_alt_spine",
         enc.as_raw_slice().len(),
         &envelope::SKYLINE_VALIDATE_ALT_SPINE,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 1515:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<a id="hunk-33"></a>
### crates/before/tests/meter.rs `@@ -1497,897 +1520,355 @@ fn skyline_validate_alt_spine_envelope() {`

```diff
@@ -1497,897 +1520,355 @@ fn skyline_validate_alt_spine_envelope() {
     assert!(r.is_ok(), "the transcoded alternating spine is canonical");
 }
 
-/// The skyline decoder on the dense spine stays within its envelope (the
-/// transcode materializes per-node floors, priced by the packed output).
+/// The validator rows' scan floor is live.
+///
+/// Judged against `SKYLINE_VALIDATE_DENSE` with its limb and touch bands
+/// opened, over the whole dense stream's length, a validator stubbed to
+/// `Ok(())` and one that reads only half the stream each fail on the scan
+/// floor and nothing else.
+///
+/// The known-bad validators do less work than the row, so every ceiling
+/// passes them and only a floor can catch them.
 #[test]
-fn skyline_decode_dense_envelope() {
-    let p = Shape::Dense.packed1(DENSE_DEPTH);
-    let enc = skyline_of(&p);
-    let v = sweep_metered(
-        "skyline_decode_dense",
-        enc.as_raw_slice().len(),
-        &envelope::SKYLINE_DECODE_DENSE,
-        || meter::skyline::decode(meter::skyline::view(&enc)).expect("canonical"),
+fn stopped_validator_fails_the_validate_row() {
+    let whole = skyline_of(&Shape::Dense.packed1(DENSE_DEPTH));
+    let half = skyline_of(&Shape::Dense.packed1(DENSE_DEPTH / 2));
+    let input = whole.as_raw_slice().len();
+    // The row with its limb and touch bands opened, so the scan floor is
+    // the one judge either known-bad validator can fail.
+    let scan_only = Envelope {
+        limb: band(u64::MAX, 0),
+        touch: band(u64::MAX, 0),
+        ..envelope::SKYLINE_VALIDATE_DENSE
+    };
+    let failure = |name: &str, body: &dyn Fn()| {
+        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
+            metered(name, input, &scan_only, body)
+        }))
+        .err()
+        .and_then(|payload| payload.downcast_ref::<String>().cloned())
+        .expect("the row must fail, with a message")
+    };
+    let stubbed = failure("validate_stub_probe", &|| ());
+    assert!(
+        stubbed.contains("liveness floor"),
+        "a stubbed validator must fail the scan floor, not: {stubbed}"
+    );
+    let stopped = failure("validate_stopped_probe", &|| {
+        meter::skyline::validate(meter::skyline::view(&half))
+            .expect("the half-depth spine is canonical");
+    });
+    assert!(
+        stopped.contains("liveness floor"),
+        "a validator that reads half its input must fail the scan floor, not: {stopped}"
     );
-    assert_eq!(v, version_of(&p), "the transcode round-trips");
 }
 
-/// The skyline decoder on the boundary comb stays within its envelope.
-///
-/// The packed output stores a fresh `gamma(2^k − 1)` per tooth, so the
-/// materialized heights and floors are output-sized — quadratically above
-/// the skyline input, linearly within the packed form being rebuilt.
+/// The skyline decoder round-trips every envelope family the loop lists:
+/// `decode(encode(v)) == v`.
 #[test]
-fn skyline_decode_cliff_envelope() {
-    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
-    let enc = skyline_of(&p);
-    let v = sweep_metered(
-        "skyline_decode_cliff",
-        enc.as_raw_slice().len(),
-        &envelope::SKYLINE_DECODE_CLIFF,
-        || meter::skyline::decode(meter::skyline::view(&enc)).expect("canonical"),
-    );
-    assert_eq!(v, version_of(&p), "the transcode round-trips");
+fn skyline_decode_round_trips_the_families() {
+    for p in [
+        Shape::Dense.packed1(DENSE_DEPTH),
+        Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE),
+        Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE),
+        Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS),
+        Shape::AltSpine.packed1(DENSE_DEPTH),
+    ] {
+        let v = version_of(&p);
+        let enc = meter::skyline::encode(&v);
+        assert_eq!(
+            meter::skyline::decode(meter::skyline::view(&enc)).expect("canonical"),
+            v,
+            "the transcode round-trips"
+        );
+    }
 }
 
-/// The skyline decoder on the wide-tooth comb stays within its envelope
-/// (wide heights and floors, output-priced like the boundary comb's).
-#[test]
-fn skyline_decode_wide_tooth_envelope() {
-    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
-    let enc = skyline_of(&p);
-    let v = sweep_metered(
-        "skyline_decode_wide_tooth",
-        enc.as_raw_slice().len(),
-        &envelope::SKYLINE_DECODE_WIDE_TOOTH,
-        || meter::skyline::decode(meter::skyline::view(&enc)).expect("canonical"),
-    );
-    assert_eq!(v, version_of(&p), "the transcode round-trips");
+// ─── skyline text kernel scenarios ──────────────────────────────────────────
+//
+// The skyline-first text kernels (`meter::skyline::text`): rendering
+// derives every printed base in delta-sized relative coordinates and
+// sizes its output exactly before writing; parsing turns path-sum
+// movement into skyline payloads through the per-leaf delta accumulator
+// and the collapsing output builder. The columns carry the kernels'
+// contract: heap in the open-node stacks, the digit arena, and the output
+// itself, limb
+// work linear per I/O byte (radix conversion runs inside the backend;
+// the recorded ops are the delta algebra), and scan linear in the
+// skyline stream. The bigroot rows are the width separator: the 40k-bit
+// heights never materialize, so no summary or accumulator state carries
+// a copy of the wide magnitude per level. The render rows pin zero
+// accumulator touches: the renderer's relative-coordinate summaries carry
+// no running accumulator, so a render that adopts one, or parse-side
+// accumulator work leaking into the render path, moves a pinned constant
+// instead of arriving silently.
+
+// Pins per the file doc's convention; each row's trailing comment states
+// the mechanism that prices it.
+#[rustfmt::skip]
+mod text_env {
+    use super::{band, envelope, Envelope};
+    pub const SKYLINE_RENDER_DENSE: Envelope       = envelope(1_996_800, band(1_562_513, 937_507),             band(0, 0), band(468_758, 281_254)); // word-sized finalize summaries per open node; the output sized exactly before one byte is written
+    pub const SKYLINE_RENDER_BIGROOT: Envelope     = envelope(  249_600,    band(127_368, 76_420),             band(0, 0),  band(137_512, 82_506)); // leaf-delta-sized summaries: no per-level copy of the wide root value
+    pub const SKYLINE_RENDER_HUGELEAF: Envelope    = envelope(  171_310,       band(7_330, 4_398),             band(0, 0), band(312_503, 187_501)); // one delegated decimal rendering plus the exact-sized output, no tree state
+    pub const SKYLINE_RENDER_WIDE_ARMING: Envelope = envelope(  558_320,   band(482_700, 289_620),             band(0, 0),  band(146_033, 87_619)); // the parse direction's dual: the wide swing then the dense trail walked through the summary merges, no accumulator work
+    pub const SKYLINE_RENDER_CLIFF: Envelope       = envelope(1_113_202,   band(243_385, 146_031),             band(0, 0),   band(17_923, 10_753)); // each tooth's printed base re-derived from its 3-bit deltas, paid by its own rendered digits
+    pub const SKYLINE_PARSE_DENSE: Envelope        = envelope(4_041_052,   band(625_007, 375_003),  band(156_254, 93_752), band(468_758, 281_254)); // parallel chunked open-node stacks; the parse pipeline ends at the builder — the built stream's canonicality rides the committed render↔parse inverse pair and transcoder differential — so the scan column is the build pass's own, and word-valued payloads keep narrow-value work out of the limb denomination
+    pub const SKYLINE_PARSE_BIGROOT: Envelope      = envelope(  377_944,     band(51_574, 30_944),   band(18_754, 11_252),  band(137_512, 82_506)); // the wide root base converts once through the backend's divide-and-conquer parser; the scan column is the build pass's own
+    pub const SKYLINE_PARSE_HUGELEAF: Envelope     = envelope(  152_480,       band(4_887, 2_931),   band(19_537, 11_721), band(312_503, 187_501)); // one delegated conversion, one absolute payload out; no accumulator re-walks the built stream's wide payloads
+    pub const SKYLINE_PARSE_CLIFF: Envelope        = envelope(  344_152,     band(56_475, 33_885), band(172_839, 103_703),   band(17_923, 10_753)); // every tooth's base enters and leaves the cliff-free accumulator paid by its own digit run; the scan column is the build pass's own
 }
 
-/// The skyline decoder on the hugeleaf analog stays within its envelope
-/// (one wide height, one wide floor, one wide re-emitted gamma code).
+/// Rendering the dense spine's skyline stays within its envelope.
+///
+/// 125k open-node levels and ~250k single-digit printed bases finalize
+/// through word-sized summaries, the output is sized exactly before one
+/// byte is written, and nothing recurses.
 #[test]
-fn skyline_decode_hugeleaf_envelope() {
-    let p = Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS);
-    let enc = skyline_of(&p);
-    let v = sweep_metered(
-        "skyline_decode_hugeleaf",
-        enc.as_raw_slice().len(),
-        &envelope::SKYLINE_DECODE_HUGELEAF,
-        || meter::skyline::decode(meter::skyline::view(&enc)).expect("canonical"),
+fn skyline_render_dense_envelope() {
+    let v = version_of(&Shape::Dense.packed1(DENSE_DEPTH));
+    let a = meter::skyline::encode(&v);
+    let expected = v.to_string();
+    let out = metered(
+        "skyline_render_dense",
+        a.as_raw_slice().len(),
+        &text_env::SKYLINE_RENDER_DENSE,
+        || meter::skyline::text::render(meter::skyline::view(&a)),
     );
-    assert_eq!(v, version_of(&p), "the transcode round-trips");
+    assert_eq!(out, expected, "the kernel must render Display's bytes");
 }
 
-/// The skyline decoder on the alternating-binary spine stays within its
-/// envelope (small heights and floors, one per node, output-priced).
+/// Rendering bigroot's skyline stays within its envelope — the width
+/// separator.
+///
+/// Every leaf height carries the 40k-bit root magnitude, but the finalize
+/// pass's summaries are leaf-delta-sized, so the deep spine's transient
+/// holds no per-level copy of the wide value and the one wide printed
+/// base is paid by its own rendered digits.
 #[test]
-fn skyline_decode_alt_spine_envelope() {
-    let p = Shape::AltSpine.packed1(DENSE_DEPTH);
-    let enc = skyline_of(&p);
-    let v = sweep_metered(
-        "skyline_decode_alt_spine",
-        enc.as_raw_slice().len(),
-        &envelope::SKYLINE_DECODE_ALT_SPINE,
-        || meter::skyline::decode(meter::skyline::view(&enc)).expect("canonical"),
+fn skyline_render_bigroot_envelope() {
+    let v = version_of(&Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
+    let a = meter::skyline::encode(&v);
+    let expected = v.to_string();
+    let out = metered(
+        "skyline_render_bigroot",
+        a.as_raw_slice().len(),
+        &text_env::SKYLINE_RENDER_BIGROOT,
+        || meter::skyline::text::render(meter::skyline::view(&a)),
     );
-    assert_eq!(v, version_of(&p), "the transcode round-trips");
+    assert_eq!(out, expected, "the kernel must render Display's bytes");
 }
 
-// ─── skyline comparison sweep scenarios ─────────────────────────────────────
-//
-// The comparison sweep over skyline streams: one merge of the two leaf
-// sequences on two path-bit stacks and one cliff-free accumulator, no
-// recursion anywhere. Streams are transcoded outside measurement. Each
-// family scenario compares the shape against the empty version — the
-// shallow-operand shape, where the whole deep side is consumed
-// iteratively against a single depth-0 plateau — and the self scenario
-// compares identical dense streams, so every boundary is an aligned tie
-// and both cursors advance in lockstep to full depth. These rows carry a
-// fourth column, packed-stream bits scanned, because the sweep's work is
-// dominated by stream reads that allocate nothing, recurse nothing, and
-// (off the cliff families) do almost no arithmetic — the scan column is
-// the one that sees it.
-
-/// One sweep scenario's pinned ceilings: [`Envelope`]'s three columns
-/// plus scanned bits, asserted when the `scan-meter` feature is lit.
-struct SweepEnvelope {
-    /// Peak heap delta over the scenario body, in bytes.
-    peak_heap: usize,
-    /// Stack segments grown during the scenario body.
-    segments: u64,
-    /// Big-integer limb operations counted during the scenario body.
-    #[cfg(feature = "limb-meter")]
-    limb_ops: u64,
-    /// Packed-stream bits scanned during the scenario body.
-    #[cfg(feature = "scan-meter")]
-    scan_bits: u64,
-    /// Improvement tripwire under the limb column: measured ×0.75, per
-    /// the file doc's tripwire convention.
-    #[cfg(feature = "limb-meter")]
-    limb_floor: u64,
+/// Rendering hugeleaf's skyline stays within its envelope: one node, one
+/// 125k-bit magnitude, so the whole cost is the delegated decimal
+/// rendering plus the exact-sized output — no tree state at all.
+#[test]
+fn skyline_render_hugeleaf_envelope() {
+    let v = version_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
+    let a = meter::skyline::encode(&v);
+    let expected = v.to_string();
+    let out = metered(
+        "skyline_render_hugeleaf",
+        a.as_raw_slice().len(),
+        &text_env::SKYLINE_RENDER_HUGELEAF,
+        || meter::skyline::text::render(meter::skyline::view(&a)),
+    );
+    assert_eq!(out, expected, "the kernel must render Display's bytes");
 }
 
-/// Build a [`SweepEnvelope`] from the four pinned columns and the limb
-/// floor.
-///
-/// The limb and scan columns are carried only when their features
-/// compile the counters in; the leading underscores keep the parameters
-/// warning-free in the other configurations.
-const fn sweep_envelope(
-    peak_heap: usize,
-    segments: u64,
-    _limb_ops: u64,
-    _scan_bits: u64,
-    _limb_floor: u64,
-) -> SweepEnvelope {
-    SweepEnvelope {
-        peak_heap,
-        segments,
-        #[cfg(feature = "limb-meter")]
-        limb_ops: _limb_ops,
-        #[cfg(feature = "scan-meter")]
-        scan_bits: _scan_bits,
-        #[cfg(feature = "limb-meter")]
-        limb_floor: _limb_floor,
-    }
-}
-
-// The sweep envelope table: pinned ceiling = measured ×1.25, rounded up
-// (aarch64-apple-darwin, dev profile, three identical runs), and only
-// ever tightened: where a remeasure rises while staying inside an
-// existing ceiling (spilled-magnitude heap cells and their backend growth
-// headroom), the older, tighter ceiling stands. The trailing comment on
-// each line states the mechanism that prices the row; the measurements of
-// record — and every re-pin's movement and attribution — live in the pin
-// commits (`git log -S` the constant). Re-pin by rerunning under
-// `--no-capture` with `--all-features` and reading the MEASURED lines.
-// The limb floor column is the measured value ×0.75, rounded down (the
-// file doc's improvement-tripwire convention).
-#[rustfmt::skip]
-mod sweep_env {
-    use super::{sweep_envelope, SweepEnvelope};
-    //                                                               peak heap, segments, limb ops,  scan bits, limb floor
-    pub const SKYLINE_CMP_DENSE: SweepEnvelope = sweep_envelope(30_720, 0, 0, 468_760, 0); // path-bit stacks and one accumulator; word-valued payloads keep the limb column at zero
-    pub const SKYLINE_CMP_DENSE_SELF: SweepEnvelope = sweep_envelope(51_200, 0, 0, 937_515, 0); // aligned ties in lockstep to full depth: both streams' bits scanned whole
-    pub const SKYLINE_CMP_BIGROOT: SweepEnvelope = sweep_envelope(39_540, 0, 783, 137_514, 469); // the wide first height absorbed once, paid by its own code
-    pub const SKYLINE_CMP_CLIFF: SweepEnvelope = sweep_envelope(1_330, 0, 88, 17_925, 52); // the cliff-free accumulator: amortized O(1) per crossing (the shared emission-sweep step holds each consumed delta; OpenedPair states the opening move once)
-    // SKYLINE_CMP_WIDE_TOOTH's deliberately thin heap margin is a
-    // change-detector on the backend's and the accumulator's allocation
-    // policies: the committed Cargo.lock (dashu-int 0.5.0 exact) is what
-    // makes the measurement deterministic, and a cargo update to any other
-    // 0.5.x is a deliberate re-measure event, not noise.
-    pub const SKYLINE_CMP_WIDE_TOOTH: SweepEnvelope = sweep_envelope(    1_250,        0,    29_509, 1_000_483, 17_705); // each wide delta's limb work paid by its own zigzag code; heap stays at the stacks, the accumulator, and the zero-run ledger's map node
-}
-
-/// Run one sweep scenario body under all four meters and assert its
-/// envelope.
-///
-/// [`metered`]'s harness plus the scan column; prints the measured
-/// numbers so re-pinning never requires editing the harness.
-fn sweep_metered<R>(
-    name: &str,
-    input_bytes: usize,
-    env: &SweepEnvelope,
-    f: impl FnOnce() -> R,
-) -> R {
-    meter::reset_stack_segments();
-    #[cfg(feature = "limb-meter")]
-    meter::reset_limb_ops();
-    #[cfg(feature = "scan-meter")]
-    meter::reset_scan_bits();
-    HEAP.reset_peak_usage();
-    let baseline = HEAP.current_usage();
-    let r = f();
-    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
-    let segments = meter::stack_segments();
-    #[cfg(feature = "limb-meter")]
-    let limb_ops = meter::limb_ops();
-    #[cfg(feature = "scan-meter")]
-    let scan_bits = meter::scan_bits();
-    #[cfg(feature = "limb-meter")]
-    let limb_col = format!(" limb_ops={limb_ops}");
-    #[cfg(not(feature = "limb-meter"))]
-    let limb_col = "";
-    #[cfg(feature = "scan-meter")]
-    let scan_col = format!(" scan_bits={scan_bits}");
-    #[cfg(not(feature = "scan-meter"))]
-    let scan_col = "";
-    eprintln!(
-        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments}{limb_col}{scan_col}"
-    );
-    assert!(
-        peak_heap <= env.peak_heap,
-        "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B): {ISOLATION_NOTE}",
-        env.peak_heap,
-    );
-    assert!(
-        segments <= env.segments,
-        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.segments,
-    );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        limb_ops <= env.limb_ops,
-        "{name}: {limb_ops} limb operations exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.limb_ops,
-    );
-    #[cfg(feature = "scan-meter")]
-    assert!(
-        scan_bits <= env.scan_bits,
-        "{name}: {scan_bits} scanned bits exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.scan_bits,
-    );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        limb_ops >= env.limb_floor,
-        "{name}: limb counter reads {limb_ops}, below the {} improvement \
-         tripwire (measured x0.75): attribute the drop — an honest \
-         improvement re-pins the band; a dead meter is the bypass this \
-         column exists to catch",
-        env.limb_floor,
-    );
-    r
-}
-
-/// The empty version's two-bit skyline stream: the shallow operand of
-/// the family cmp scenarios.
-fn skyline_empty() -> meter::skyline::BitsBuf {
-    meter::skyline::encode(&Version::new())
-}
-
-/// The combined operand bytes of a sweep scenario.
-fn sweep_input_bytes(a: &meter::skyline::BitsBuf, b: &meter::skyline::BitsBuf) -> usize {
-    a.as_raw_slice().len() + b.as_raw_slice().len()
-}
-
-/// The sweep on the dense spine against the empty version stays within
-/// its envelope.
-///
-/// The deep side's 125k levels cost path *bits* (no grown segments, heap
-/// in the path stack), consumed iteratively against one depth-0 plateau.
+/// Rendering the boundary comb's skyline stays within its envelope:
+/// every tooth's wide printed base re-derives from 3-bit `±1` deltas
+/// against the running relative floor, each merge paid by the tooth's
+/// own rendered digits.
 #[test]
-fn skyline_cmp_dense_envelope() {
-    let a = skyline_of(&Shape::Dense.packed1(DENSE_DEPTH));
-    let b = skyline_empty();
-    let r = sweep_metered(
-        "skyline_cmp_dense",
-        sweep_input_bytes(&a, &b),
-        &sweep_env::SKYLINE_CMP_DENSE,
-        || meter::skyline::sweep::causal_cmp(meter::skyline::view(&a), meter::skyline::view(&b)),
-    );
-    assert_eq!(
-        r,
-        Some(Ordering::Greater),
-        "the dense spine strictly dominates the empty version"
+fn skyline_render_cliff_envelope() {
+    let v = version_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
+    let a = meter::skyline::encode(&v);
+    let expected = v.to_string();
+    let out = metered(
+        "skyline_render_cliff",
+        a.as_raw_slice().len(),
+        &text_env::SKYLINE_RENDER_CLIFF,
+        || meter::skyline::text::render(meter::skyline::view(&a)),
     );
+    assert_eq!(out, expected, "the kernel must render Display's bytes");
 }
 
-/// The sweep on two identical dense streams stays within its envelope.
+/// Rendering the wide-arming stream's skyline stays within its envelope.
 ///
-/// Every boundary is an aligned tie, both cursors advance in lockstep to
-/// full depth, and the verdict is Equal only after both streams are
-/// wholly consumed (no early exit anywhere).
+/// The wide swing and then the dense trail walk through the summary
+/// merges, and the touch cell pins the zero accumulator work doing it (the
+/// parse direction's dual).
 #[test]
-fn skyline_cmp_dense_self_envelope() {
-    let a = skyline_of(&Shape::Dense.packed1(DENSE_DEPTH));
-    let b = a.clone();
-    let r = sweep_metered(
-        "skyline_cmp_dense_self",
-        sweep_input_bytes(&a, &b),
-        &sweep_env::SKYLINE_CMP_DENSE_SELF,
-        || meter::skyline::sweep::causal_cmp(meter::skyline::view(&a), meter::skyline::view(&b)),
+fn skyline_render_wide_arming_envelope() {
+    let v =
+        version_of(&Shape::WideArming.packed2(WIDE_ARMING_RENDER_SCALE, WIDE_ARMING_RENDER_SCALE));
+    let a = meter::skyline::encode(&v);
+    let expected = v.to_string();
+    let out = metered(
+        "skyline_render_wide_arming",
+        a.as_raw_slice().len(),
+        &text_env::SKYLINE_RENDER_WIDE_ARMING,
+        || meter::skyline::text::render(meter::skyline::view(&a)),
     );
-    assert_eq!(r, Some(Ordering::Equal), "identical streams read equal");
+    assert_eq!(out, expected, "the kernel must render Display's bytes");
 }
 
-/// The sweep on bigroot against the empty version stays within its
-/// envelope: the difference accumulator absorbs the wide first height
-/// once (paid by its own code) and every later delta is small.
+/// Parsing the dense spine's text stays within its envelope: one frame
+/// per open node, single-digit bases through the delegated reader, and
+/// the per-leaf delta accumulator staying word-sized throughout.
 #[test]
-fn skyline_cmp_bigroot_envelope() {
-    let a = skyline_of(&Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
-    let b = skyline_empty();
-    let r = sweep_metered(
-        "skyline_cmp_bigroot",
-        sweep_input_bytes(&a, &b),
-        &sweep_env::SKYLINE_CMP_BIGROOT,
-        || meter::skyline::sweep::causal_cmp(meter::skyline::view(&a), meter::skyline::view(&b)),
+fn skyline_parse_dense_envelope() {
+    let v = version_of(&Shape::Dense.packed1(DENSE_DEPTH));
+    let s = v.to_string();
+    let expected = meter::skyline::encode(&v);
+    let out = metered(
+        "skyline_parse_dense",
+        s.len(),
+        &text_env::SKYLINE_PARSE_DENSE,
+        || meter::skyline::text::parse(&s).expect("canonical text parses"),
     );
     assert_eq!(
-        r,
-        Some(Ordering::Greater),
-        "bigroot strictly dominates the empty version"
+        out, expected,
+        "the kernel must build the transcoder's stream"
     );
 }
 
-/// The sweep on the boundary comb against the empty version stays within
-/// its envelope.
+/// Parsing bigroot's text stays within its envelope.
 ///
-/// Every 3-bit `±1` delta drives the running difference across the `2^k`
-/// carry boundary, and the accumulator keeps each crossing amortized O(1)
-/// (the flatness pin below is the cross-scale witness).
+/// The 12k-digit root base converts once through the backend's
+/// divide-and-conquer parser, joins and leaves the delta accumulator
+/// exactly twice, and every spine base is word-sized — no per-level copy
+/// of the wide value.
 #[test]
-fn skyline_cmp_cliff_envelope() {
-    let a = skyline_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
-    let b = skyline_empty();
-    let r = sweep_metered(
-        "skyline_cmp_cliff",
-        sweep_input_bytes(&a, &b),
-        &sweep_env::SKYLINE_CMP_CLIFF,
-        || meter::skyline::sweep::causal_cmp(meter::skyline::view(&a), meter::skyline::view(&b)),
+fn skyline_parse_bigroot_envelope() {
+    let v = version_of(&Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
+    let s = v.to_string();
+    let expected = meter::skyline::encode(&v);
+    let out = metered(
+        "skyline_parse_bigroot",
+        s.len(),
+        &text_env::SKYLINE_PARSE_BIGROOT,
+        || meter::skyline::text::parse(&s).expect("canonical text parses"),
     );
     assert_eq!(
-        r,
-        Some(Ordering::Greater),
-        "the comb strictly dominates the empty version"
+        out, expected,
+        "the kernel must build the transcoder's stream"
     );
 }
 
-/// The sweep on the wide-tooth comb against the empty version stays
-/// within its envelope.
-///
-/// Each `±2^w` delta is a genuinely wide operand paid by its own zigzag
-/// code, so limb work stays linear per input bit at every tooth width.
+/// Parsing hugeleaf's text stays within its envelope: one ~37k-digit
+/// run through the delegated conversion, one absolute payload out — the
+/// shape where any superlinear parse-side arithmetic shows undiluted.
 #[test]
-fn skyline_cmp_wide_tooth_envelope() {
-    let a =
-        skyline_of(&Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE));
-    let b = skyline_empty();
-    let r = sweep_metered(
-        "skyline_cmp_wide_tooth",
-        sweep_input_bytes(&a, &b),
-        &sweep_env::SKYLINE_CMP_WIDE_TOOTH,
-        || meter::skyline::sweep::causal_cmp(meter::skyline::view(&a), meter::skyline::view(&b)),
+fn skyline_parse_hugeleaf_envelope() {
+    let v = version_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
+    let s = v.to_string();
+    let expected = meter::skyline::encode(&v);
+    let out = metered(
+        "skyline_parse_hugeleaf",
+        s.len(),
+        &text_env::SKYLINE_PARSE_HUGELEAF,
+        || meter::skyline::text::parse(&s).expect("canonical text parses"),
     );
     assert_eq!(
-        r,
-        Some(Ordering::Greater),
-        "the wide-tooth comb strictly dominates the empty version"
-    );
-}
-
-// ─── skyline join/meet emission scenarios ───────────────────────────────────
-//
-// The emission sweep over the same adversarial families: pointwise
-// max/min re-delta-coded through the collapsing output builder, the
-// measured result held alive so every peak includes the emitted stream.
-// The columns carry the emission contract — zero grown segments (nothing
-// recurses), heap in the cursor paths, the builder's bit stacks, and the
-// output itself, limb work linear per input bit through the accumulator,
-// and scanned-plus-written bits linear in the streams. The absorb row is
-// the builder's collapse-heavy extreme: a flat dominating operand over
-// the dense spine collapses the whole output to one leaf, one truncation
-// per level around a held wide code — the shape whose cost a re-copying
-// collapse discipline would make quadratic in depth times code width.
-
-// The emission envelope table: pinned ceiling = measured ×1.25, rounded
-// up (aarch64-apple-darwin, dev profile, three identical runs), and only
-// ever tightened: where a remeasure rises while staying inside an
-// existing ceiling (spilled-magnitude heap cells and their backend growth
-// headroom), the older, tighter ceiling stands. The trailing comment on
-// each line states the mechanism that prices the row; the measurements of
-// record — and every re-pin's movement and attribution — live in the pin
-// commits (`git log -S` the constant). Re-pin by rerunning under
-// `--no-capture` with `--all-features` and reading the MEASURED lines.
-// The limb floor column is the measured value ×0.75, rounded down (the
-// file doc's improvement-tripwire convention).
-#[rustfmt::skip]
-mod emit_env {
-    use super::{sweep_envelope, SweepEnvelope};
-    //                                                                peak heap, segments, limb ops,  scan bits, limb floor
-    pub const SKYLINE_JOIN_DENSE: SweepEnvelope = sweep_envelope(130_277, 0, 0, 625_018, 0); // the peak is the emitted stream itself; word-valued payloads keep the limb column at zero
-    pub const SKYLINE_JOIN_ABSORB: SweepEnvelope = sweep_envelope(270_798, 0, 4_887, 1_250_013, 2_931); // the collapse-heavy extreme: one truncation per level around a held wide code, which absorb never moves
-    pub const SKYLINE_JOIN_BIGROOT: SweepEnvelope = sweep_envelope(85_060, 0, 1_565, 275_028, 939); // the wide first height absorbed once, paid by its own code
-    pub const SKYLINE_JOIN_CLIFF: SweepEnvelope = sweep_envelope(5_362, 0, 308, 35_848, 184); // every crossing re-emitted at amortized O(1) through the accumulator
-    pub const SKYLINE_JOIN_WIDE_TOOTH: SweepEnvelope = sweep_envelope(  128_312,        0,    74_477, 2_000_963, 44_685); // each wide delta re-coded into the output, paid by its own zigzag code
-    pub const SKYLINE_MEET_CLIFF: SweepEnvelope = sweep_envelope(4_422, 0, 88, 23_055, 52); // the absorb cascade collapses to the flat leaf while every delta still crosses the carry boundary in the accumulator
-    pub const SKYLINE_MEET_WIDE_TOOTH: SweepEnvelope = sweep_envelope(127_732, 0, 29_512, 1_005_613, 17_706); // wide deltas folded but never re-emitted: the collapse discipline at spilled operand widths
-}
-
-/// The one-tick version's skyline stream: the shallow operand of the
-/// family join/meet scenarios, mirroring the packed-form join rows.
-fn skyline_one_tick() -> meter::skyline::BitsBuf {
-    let one = Version::try_from(1u64).expect("a one-tick version is valid");
-    meter::skyline::encode(&one)
-}
-
-/// One family shape and the packed-form oracle's answer against the
-/// one-tick version, both as skyline streams built outside measurement,
-/// so every scenario asserts byte-identity after its sweep.
-fn skyline_oracle(
-    p: &meter::Packed,
-    join: bool,
-) -> (meter::skyline::BitsBuf, meter::skyline::BitsBuf) {
-    let v = version_of(p);
-    let one = Version::try_from(1u64).expect("a one-tick version is valid");
-    let out = if join { &v | &one } else { &v & &one };
-    (meter::skyline::encode(&v), meter::skyline::encode(&out))
-}
-
-/// Joining the dense spine's skyline with a one-tick stream stays within
-/// its envelope.
-///
-/// The 125k-level walk emits and collapses on path-bit stacks and one
-/// accumulator, with zero grown segments and the peak in the emitted
-/// stream itself.
-#[test]
-fn skyline_join_dense_envelope() {
-    let p = Shape::Dense.packed1(DENSE_DEPTH);
-    let (a, expected) = skyline_oracle(&p, true);
-    let b = skyline_one_tick();
-    let out = sweep_metered(
-        "skyline_join_dense",
-        sweep_input_bytes(&a, &b),
-        &emit_env::SKYLINE_JOIN_DENSE,
-        || meter::skyline::emit::join(meter::skyline::view(&a), meter::skyline::view(&b)),
+        out, expected,
+        "the kernel must build the transcoder's stream"
     );
-    assert_eq!(out, expected, "the emitted join must match the oracle");
 }
 
-/// Joining the dense spine's skyline with a dominating flat operand
-/// stays within its envelope.
+/// Parsing the boundary comb's text stays within its envelope.
 ///
-/// The whole output collapses to one leaf through 125k absorb steps
-/// around a held 125k-bit code, so this row is linear only because absorb
-/// never moves the held code.
+/// Every tooth's wide base enters and leaves the cliff-free accumulator
+/// paid by its own digit run, so the `2^k` carry boundary costs amortized
+/// O(1) digit touches per crossing.
 #[test]
-fn skyline_join_absorb_envelope() {
-    let p = Shape::Dense.packed1(DENSE_DEPTH);
-    let flat = version_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
-    let a = skyline_of(&p);
-    let b = meter::skyline::encode(&flat);
-    let expected = b.clone();
-    let out = sweep_metered(
-        "skyline_join_absorb",
-        sweep_input_bytes(&a, &b),
-        &emit_env::SKYLINE_JOIN_ABSORB,
-        || meter::skyline::emit::join(meter::skyline::view(&a), meter::skyline::view(&b)),
+fn skyline_parse_cliff_envelope() {
+    let v = version_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
+    let s = v.to_string();
+    let expected = meter::skyline::encode(&v);
+    let out = metered(
+        "skyline_parse_cliff",
+        s.len(),
+        &text_env::SKYLINE_PARSE_CLIFF,
+        || meter::skyline::text::parse(&s).expect("canonical text parses"),
     );
-    assert_eq!(out, expected, "a dominating flat operand is the whole join");
-}
-
-/// Joining bigroot's skyline with a one-tick stream stays within its
-/// envelope: the wide first height is absorbed once, paid by its own
-/// code, and every later delta is small.
-#[test]
-fn skyline_join_bigroot_envelope() {
-    let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
-    let (a, expected) = skyline_oracle(&p, true);
-    let b = skyline_one_tick();
-    let out = sweep_metered(
-        "skyline_join_bigroot",
-        sweep_input_bytes(&a, &b),
-        &emit_env::SKYLINE_JOIN_BIGROOT,
-        || meter::skyline::emit::join(meter::skyline::view(&a), meter::skyline::view(&b)),
+    assert_eq!(
+        out, expected,
+        "the kernel must build the transcoder's stream"
     );
-    assert_eq!(out, expected, "the emitted join must match the oracle");
 }
 
-/// Joining the boundary comb's skyline with a one-tick stream stays
-/// within its envelope: every 3-bit `±1` delta re-emits across the
-/// `2^k` carry boundary, and the accumulator keeps each crossing
-/// amortized O(1).
-#[test]
-fn skyline_join_cliff_envelope() {
-    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
-    let (a, expected) = skyline_oracle(&p, true);
-    let b = skyline_one_tick();
-    let out = sweep_metered(
-        "skyline_join_cliff",
-        sweep_input_bytes(&a, &b),
-        &emit_env::SKYLINE_JOIN_CLIFF,
-        || meter::skyline::emit::join(meter::skyline::view(&a), meter::skyline::view(&b)),
-    );
-    assert_eq!(out, expected, "the emitted join must match the oracle");
-}
+// ─── skyline cliff-freedom flatness ─────────────────────────────────────────
+//
+// The cross-scale witness that the validator's nonnegativity state is
+// cliff-free on the boundary comb: per-delta accumulator digit touches
+// and per-input-byte limb work both stay flat (×1.25) across a size
+// doubling of `k = n`. A plain big-integer running height roughly doubles
+// its per-unit cost per doubling here (the `meter/tier2` plain-sweep pin),
+// so this is the row that separates the two representations — provided
+// the height state actually runs on the metered accumulator. The touch
+// column carries a liveness floor of one digit touch per delta (the
+// counterpart of the heap meter's canaries): an unmetered height
+// representation registers zero touches, and a flatness ratio over zeros
+// holds vacuously, so the floor is what makes the flatness column a
+// witness rather than a tautology.
+#[cfg(feature = "limb-meter")]
+mod skyline_flatness {
+    use before::meter;
+    use before::meter::registry::Shape;
+    use suanpan::touch_meter;
 
-/// Joining the wide-tooth comb's skyline with a one-tick stream stays
-/// within its envelope: each `±2^w` delta is a genuinely wide operand
-/// re-coded into the output, paid by its own zigzag code.
-#[test]
-fn skyline_join_wide_tooth_envelope() {
-    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
-    let (a, expected) = skyline_oracle(&p, true);
-    let b = skyline_one_tick();
-    let out = sweep_metered(
-        "skyline_join_wide_tooth",
-        sweep_input_bytes(&a, &b),
-        &emit_env::SKYLINE_JOIN_WIDE_TOOTH,
-        || meter::skyline::emit::join(meter::skyline::view(&a), meter::skyline::view(&b)),
-    );
-    assert_eq!(out, expected, "the emitted join must match the oracle");
-}
+    /// Slack numerator over the small-scale cost (denominator
+    /// [`SLACK_DEN`]): the ×1.25 flatness convention.
+    const SLACK_NUM: u64 = 5;
 
-/// Meeting the boundary comb's skyline with a one-tick stream stays
-/// within its envelope.
-///
-/// The output collapses to the flat one-tick leaf through the absorb
-/// cascade while every comb delta still crosses the carry boundary in the
-/// accumulator.
-#[test]
-fn skyline_meet_cliff_envelope() {
-    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
-    let (a, expected) = skyline_oracle(&p, false);
-    let b = skyline_one_tick();
-    let out = sweep_metered(
-        "skyline_meet_cliff",
-        sweep_input_bytes(&a, &b),
-        &emit_env::SKYLINE_MEET_CLIFF,
-        || meter::skyline::emit::meet(meter::skyline::view(&a), meter::skyline::view(&b)),
-    );
-    assert_eq!(out, expected, "the emitted meet must match the oracle");
-}
+    /// Slack denominator for the flatness bound.
+    const SLACK_DEN: u64 = 4;
 
-/// Meeting the wide-tooth comb's skyline with a one-tick stream stays
-/// within its envelope.
-///
-/// Wide deltas are folded but never re-emitted (the flat side wins
-/// everywhere), so the collapse discipline runs at spilled operand
-/// widths.
-#[test]
-fn skyline_meet_wide_tooth_envelope() {
-    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
-    let (a, expected) = skyline_oracle(&p, false);
-    let b = skyline_one_tick();
-    let out = sweep_metered(
-        "skyline_meet_wide_tooth",
-        sweep_input_bytes(&a, &b),
-        &emit_env::SKYLINE_MEET_WIDE_TOOTH,
-        || meter::skyline::emit::meet(meter::skyline::view(&a), meter::skyline::view(&b)),
-    );
-    assert_eq!(out, expected, "the emitted meet must match the oracle");
-}
+    /// One comb validation run: the two per-unit denominators (deltas for
+    /// touches, skyline bytes for limb ops) and both counters.
+    struct Run {
+        deltas: u64,
+        bytes: u64,
+        touches: u64,
+        limb_ops: u64,
+    }
 
-// ─── grow-branch tick scenarios ─────────────────────────────────────────────
-//
-// The deep expansion shapes: pairs whose fill is the identity, so the
-// fused tick records the inflation route on its one walk and replays it
-// through the splice emit. These rows measure the whole public tick —
-// walk, route fold, and splice — on the shapes whose id side dominates
-// (their envelopes live in `query_env` with the other tick rows).
-
-/// The version that is `1` on the leftmost `2^-depth` interval and `0`
-/// everywhere else: `depth` nested nodes, all bases zero, the single
-/// 1-leaf at the bottom left.
-///
-/// This is what ticking a version that is zero over the owned region
-/// registers for a depth-`depth` unary id spine. Built as a text
-/// literal (the parser is iterative), so the expected tree shares no
-/// walk with the tick under measurement.
-fn left_spike(depth: usize) -> Version {
-    let mut text = "(0, ".repeat(depth - 1);
-    text.push_str("(0, 1, 0)");
-    text.push_str(&", 0)".repeat(depth - 1));
-    text.parse().expect("the spike literal is normal form")
-}
-
-/// Ticking the empty version under a 250k-deep unary id spine stays
-/// within its envelope.
-///
-/// The walk is one event leaf whose route fold is the iterative id
-/// scan (bit-stack frames, nothing recurses), and the emit codes the
-/// whole expansion chain as fresh one-bit deltas. The value witness is
-/// closed-form: the expansion chain to the owned tip is exactly the
-/// left spike literal.
-#[test]
-fn tick_expand_spine_envelope() {
-    let mut v = Version::new();
-    let party = party_of(&Shape::IdSpine.packed_flagged(ID_DEPTH, false));
-    // Byte sizes of buffers this test just allocated fit `usize`.
-    let input =
-        ((meter::skyline::encode(&v).len() / 8) + party.encoded_bits().div_ceil(8)) as usize;
-    query_metered(
-        "tick_expand_spine",
-        input,
-        &query_env::TICK_EXPAND_SPINE,
-        || v.tick(&party),
-    );
-    assert_eq!(
-        v,
-        left_spike(ID_DEPTH),
-        "the ticked version must be the derived closed form"
-    );
-}
-
-/// Ticking the alternating spine under a deep unary id spine stays
-/// within its envelope.
-///
-/// The regimes mix — the two-cursor fused walk down the shared spine,
-/// an id-only expansion fold where the id outruns the event — and the
-/// splice replays the recorded route. The value witness is closed-form:
-/// the unary id turns left into the spine's depth-2 zero leaf, so the
-/// forced route raises exactly the owned region from 0 to 1 — the
-/// pointwise max with the left spike, realized through the
-/// independently-tested join, byte-exact by canonical uniqueness.
-#[test]
-fn tick_expand_cross_envelope() {
-    let ev = Shape::AltSpine.packed1(DENSE_DEPTH);
-    let mut v = version_of(&ev);
-    let party = party_of(&Shape::IdSpine.packed_flagged(ID_DEPTH, false));
-    let expected = &v | &left_spike(ID_DEPTH);
-    // Byte sizes of buffers this test just allocated fit `usize`.
-    let input = ev.bytes.len() + party.encoded_bits().div_ceil(8) as usize;
-    query_metered(
-        "tick_expand_cross",
-        input,
-        &query_env::TICK_EXPAND_CROSS,
-        || v.tick(&party),
-    );
-    assert_eq!(
-        v, expected,
-        "the ticked version must be the derived closed form"
-    );
-}
-
-// ─── skyline text kernel scenarios ──────────────────────────────────────────
-//
-// The skyline-first text kernels (`meter::skyline::text`): rendering
-// derives every printed base in delta-sized relative coordinates and
-// sizes its output exactly before writing; parsing turns path-sum
-// movement into skyline payloads through the per-leaf delta accumulator
-// and the collapsing output builder. The columns carry the kernels'
-// contract — zero grown segments (nothing recurses at any depth), heap
-// in the open-node stacks, the digit arena, and the output itself, limb
-// work linear per I/O byte (radix conversion runs inside the backend;
-// the recorded ops are the delta algebra), and scan linear in the
-// skyline stream. The bigroot rows are the width separator: the 40k-bit
-// heights never materialize, so no summary or accumulator state carries
-// a copy of the wide magnitude per level.
-
-// The text envelope table: pinned ceiling = measured ×1.25, rounded up
-// (aarch64-apple-darwin, dev profile, identical repeated runs), and only
-// ever tightened. The trailing comment on each line states the mechanism
-// that prices the row; the measurements of record — and every re-pin's
-// movement and attribution — live in the pin commits (`git log -S` the
-// constant). Re-pin by rerunning under `--no-capture` with
-// `--all-features` and reading the MEASURED lines.
-// The limb floor column is the measured value ×0.75, rounded down (the
-// file doc's improvement-tripwire convention).
-#[rustfmt::skip]
-mod text_env {
-    use super::{sweep_envelope, SweepEnvelope};
-    //                                                                 peak heap, segments, limb ops,  scan bits, limb floor
-    pub const SKYLINE_RENDER_DENSE: SweepEnvelope    = sweep_envelope( 1_996_800,        0, 1_562_513,   468_758, 937_507); // word-sized finalize summaries per open node; the output sized exactly before one byte is written
-    pub const SKYLINE_RENDER_BIGROOT: SweepEnvelope  = sweep_envelope(   249_600,        0,   127_368,   137_512, 76_420); // leaf-delta-sized summaries: no per-level copy of the wide root value
-    pub const SKYLINE_RENDER_HUGELEAF: SweepEnvelope = sweep_envelope(   171_310,        0,     7_330,   312_503, 4_398); // one delegated decimal rendering plus the exact-sized output, no tree state
-    pub const SKYLINE_RENDER_CLIFF: SweepEnvelope    = sweep_envelope( 1_113_202,        0,   243_385,    17_923, 146_031); // each tooth's printed base re-derived from its 3-bit deltas, paid by its own rendered digits
-    pub const SKYLINE_PARSE_DENSE: SweepEnvelope = sweep_envelope(4_041_052, 0, 625_007, 468_758, 375_003); // parallel chunked open-node stacks; the parse pipeline ends at the builder — the built stream's canonicality rides the committed render↔parse inverse pair and transcoder differential — so the scan column is the build pass's own, and word-valued payloads keep narrow-value work out of the limb denomination
-    pub const SKYLINE_PARSE_BIGROOT: SweepEnvelope = sweep_envelope(377_944, 0, 51_574, 137_512, 30_944); // the wide root base converts once through the backend's divide-and-conquer parser; the scan column is the build pass's own
-    pub const SKYLINE_PARSE_HUGELEAF: SweepEnvelope  = sweep_envelope(   152_480,        0,     4_887,   312_503, 2_931); // one delegated conversion, one absolute payload out; no accumulator re-walks the built stream's wide payloads
-    pub const SKYLINE_PARSE_CLIFF: SweepEnvelope = sweep_envelope(344_152, 0, 56_475, 17_923, 33_885); // every tooth's base enters and leaves the cliff-free accumulator paid by its own digit run; the scan column is the build pass's own
-}
-
-/// Rendering the dense spine's skyline stays within its envelope.
-///
-/// 125k open-node levels and ~250k single-digit printed bases finalize
-/// through word-sized summaries, the output is sized exactly before one
-/// byte is written, and nothing recurses.
-#[test]
-fn skyline_render_dense_envelope() {
-    let v = version_of(&Shape::Dense.packed1(DENSE_DEPTH));
-    let a = meter::skyline::encode(&v);
-    let expected = v.to_string();
-    let out = sweep_metered(
-        "skyline_render_dense",
-        a.as_raw_slice().len(),
-        &text_env::SKYLINE_RENDER_DENSE,
-        || meter::skyline::text::render(meter::skyline::view(&a)),
-    );
-    assert_eq!(out, expected, "the kernel must render Display's bytes");
-}
-
-/// Rendering bigroot's skyline stays within its envelope — the width
-/// separator.
-///
-/// Every leaf height carries the 40k-bit root magnitude, but the finalize
-/// pass's summaries are leaf-delta-sized, so the deep spine's transient
-/// holds no per-level copy of the wide value and the one wide printed
-/// base is paid by its own rendered digits.
-#[test]
-fn skyline_render_bigroot_envelope() {
-    let v = version_of(&Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
-    let a = meter::skyline::encode(&v);
-    let expected = v.to_string();
-    let out = sweep_metered(
-        "skyline_render_bigroot",
-        a.as_raw_slice().len(),
-        &text_env::SKYLINE_RENDER_BIGROOT,
-        || meter::skyline::text::render(meter::skyline::view(&a)),
-    );
-    assert_eq!(out, expected, "the kernel must render Display's bytes");
-}
-
-/// Rendering hugeleaf's skyline stays within its envelope: one node, one
-/// 125k-bit magnitude, so the whole cost is the delegated decimal
-/// rendering plus the exact-sized output — no tree state at all.
-#[test]
-fn skyline_render_hugeleaf_envelope() {
-    let v = version_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
-    let a = meter::skyline::encode(&v);
-    let expected = v.to_string();
-    let out = sweep_metered(
-        "skyline_render_hugeleaf",
-        a.as_raw_slice().len(),
-        &text_env::SKYLINE_RENDER_HUGELEAF,
-        || meter::skyline::text::render(meter::skyline::view(&a)),
-    );
-    assert_eq!(out, expected, "the kernel must render Display's bytes");
-}
-
-/// Rendering the boundary comb's skyline stays within its envelope:
-/// every tooth's wide printed base re-derives from 3-bit `±1` deltas
-/// against the running relative floor, each merge paid by the tooth's
-/// own rendered digits.
-#[test]
-fn skyline_render_cliff_envelope() {
-    let v = version_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
-    let a = meter::skyline::encode(&v);
-    let expected = v.to_string();
-    let out = sweep_metered(
-        "skyline_render_cliff",
-        a.as_raw_slice().len(),
-        &text_env::SKYLINE_RENDER_CLIFF,
-        || meter::skyline::text::render(meter::skyline::view(&a)),
-    );
-    assert_eq!(out, expected, "the kernel must render Display's bytes");
-}
-
-/// Parsing the dense spine's text stays within its envelope: one frame
-/// per open node, single-digit bases through the delegated reader, and
-/// the per-leaf delta accumulator staying word-sized throughout.
-#[test]
-fn skyline_parse_dense_envelope() {
-    let v = version_of(&Shape::Dense.packed1(DENSE_DEPTH));
-    let s = v.to_string();
-    let expected = meter::skyline::encode(&v);
-    let out = sweep_metered(
-        "skyline_parse_dense",
-        s.len(),
-        &text_env::SKYLINE_PARSE_DENSE,
-        || meter::skyline::text::parse(&s).expect("canonical text parses"),
-    );
-    assert_eq!(
-        out, expected,
-        "the kernel must build the transcoder's stream"
-    );
-}
-
-/// Parsing bigroot's text stays within its envelope.
-///
-/// The 12k-digit root base converts once through the backend's
-/// divide-and-conquer parser, joins and leaves the delta accumulator
-/// exactly twice, and every spine base is word-sized — no per-level copy
-/// of the wide value.
-#[test]
-fn skyline_parse_bigroot_envelope() {
-    let v = version_of(&Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH));
-    let s = v.to_string();
-    let expected = meter::skyline::encode(&v);
-    let out = sweep_metered(
-        "skyline_parse_bigroot",
-        s.len(),
-        &text_env::SKYLINE_PARSE_BIGROOT,
-        || meter::skyline::text::parse(&s).expect("canonical text parses"),
-    );
-    assert_eq!(
-        out, expected,
-        "the kernel must build the transcoder's stream"
-    );
-}
-
-/// Parsing hugeleaf's text stays within its envelope: one ~37k-digit
-/// run through the delegated conversion, one absolute payload out — the
-/// shape where any superlinear parse-side arithmetic shows undiluted.
-#[test]
-fn skyline_parse_hugeleaf_envelope() {
-    let v = version_of(&Shape::Hugeleaf.packed1(HUGELEAF_MAGNITUDE_BITS));
-    let s = v.to_string();
-    let expected = meter::skyline::encode(&v);
-    let out = sweep_metered(
-        "skyline_parse_hugeleaf",
-        s.len(),
-        &text_env::SKYLINE_PARSE_HUGELEAF,
-        || meter::skyline::text::parse(&s).expect("canonical text parses"),
-    );
-    assert_eq!(
-        out, expected,
-        "the kernel must build the transcoder's stream"
-    );
-}
-
-/// Parsing the boundary comb's text stays within its envelope.
-///
-/// Every tooth's wide base enters and leaves the cliff-free accumulator
-/// paid by its own digit run, so the `2^k` carry boundary costs amortized
-/// O(1) digit touches per crossing.
-#[test]
-fn skyline_parse_cliff_envelope() {
-    let v = version_of(&Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE));
-    let s = v.to_string();
-    let expected = meter::skyline::encode(&v);
-    let out = sweep_metered(
-        "skyline_parse_cliff",
-        s.len(),
-        &text_env::SKYLINE_PARSE_CLIFF,
-        || meter::skyline::text::parse(&s).expect("canonical text parses"),
-    );
-    assert_eq!(
-        out, expected,
-        "the kernel must build the transcoder's stream"
-    );
-}
-
-// ─── skyline cliff-freedom flatness ─────────────────────────────────────────
-//
-// The cross-scale witness that the validator's nonnegativity state is
-// cliff-free on the boundary comb: per-delta accumulator digit touches
-// and per-input-byte limb work both stay flat (×1.25) across a size
-// doubling of `k = n`. A plain big-integer running height roughly doubles
-// its per-unit cost per doubling here (the `meter/tier2` plain-sweep pin),
-// so this is the row that separates the two representations — provided
-// the height state actually runs on the metered accumulator. The touch
-// column carries a liveness floor of one digit touch per delta (the
-// counterpart of the heap meter's canaries): an unmetered height
-// representation registers zero touches, and a flatness ratio over zeros
-// holds vacuously, so the floor is what makes the flatness column a
-// witness rather than a tautology.
-#[cfg(feature = "limb-meter")]
-mod skyline_flatness {
-    use before::meter;
-    use before::meter::registry::Shape;
-    use suanpan::touch_meter;
-
-    /// Slack numerator over the small-scale cost (denominator
-    /// [`SLACK_DEN`]): the ×1.25 flatness convention.
-    const SLACK_NUM: u64 = 5;
-
-    /// Slack denominator for the flatness bound.
-    const SLACK_DEN: u64 = 4;
-
-    /// One comb validation run: the two per-unit denominators (deltas for
-    /// touches, skyline bytes for limb ops) and both counters.
-    struct Run {
-        deltas: u64,
-        bytes: u64,
-        touches: u64,
-        limb_ops: u64,
-    }
-
-    /// Validate the `k = n = scale` boundary comb's skyline stream and
-    /// record both counters over the validation body alone.
-    ///
-    /// Enforces the touch-meter liveness floor before returning: every
-    /// delta code writes at least one accumulator digit, so a validator
-    /// whose height state runs on anything but the metered accumulator
-    /// (under which the flatness ratio holds vacuously at zero touches)
-    /// fails loudly here instead. The metered accumulator measures about
-    /// 1.6 touches per delta on this comb, so the one-touch floor is
-    /// comfortable.
-    fn comb_run(scale: usize) -> Run {
-        let packed = Shape::CliffComb.packed2(scale, scale);
-        let v = packed.version();
-        let enc = meter::skyline::encode(&v);
-        touch_meter::reset();
-        meter::reset_limb_ops();
-        meter::skyline::validate(meter::skyline::view(&enc)).expect("the comb stream is canonical");
-        let run = Run {
-            // 2n + 1 leaves: 2n delta codes follow the first leaf.
-            deltas: 2 * scale as u64,
-            bytes: enc.as_raw_slice().len() as u64,
-            touches: touch_meter::touches(),
-            limb_ops: meter::limb_ops(),
-        };
-        assert!(
-            run.touches >= run.deltas,
-            "skyline_comb scale {scale}: {} digit touches under the {}-delta floor: \
-             the validator's height state is not running on the metered accumulator",
-            run.touches,
-            run.deltas,
-        );
-        run
-    }
+    /// Validate the `k = n = scale` boundary comb's skyline stream and
+    /// record both counters over the validation body alone.
+    ///
+    /// Enforces the touch-meter liveness floor before returning: every
+    /// delta code writes at least one accumulator digit, so a validator
+    /// whose height state runs on anything but the metered accumulator
+    /// (under which the flatness ratio holds vacuously at zero touches)
+    /// fails loudly here instead. The metered accumulator measures about
+    /// 1.6 touches per delta on this comb, so the one-touch floor is
+    /// comfortable.
+    fn comb_run(scale: usize) -> Run {
+        let packed = Shape::CliffComb.packed2(scale, scale);
+        let v = packed.version();
+        let enc = meter::skyline::encode(&v);
+        touch_meter::reset();
+        meter::reset_limb_ops();
+        meter::skyline::validate(meter::skyline::view(&enc)).expect("the comb stream is canonical");
+        let run = Run {
+            // 2n + 1 leaves: 2n delta codes follow the first leaf.
+            deltas: 2 * scale as u64,
+            bytes: enc.as_raw_slice().len() as u64,
+            touches: touch_meter::touches(),
+            limb_ops: meter::limb_ops(),
+        };
+        assert!(
+            run.touches >= run.deltas,
+            "skyline_comb scale {scale}: {} digit touches under the {}-delta floor: \
+             the validator's height state is not running on the metered accumulator",
+            run.touches,
+            run.deltas,
+        );
+        run
+    }
 
     /// Assert one per-unit cost stays flat (×1.25) across the doubling.
     fn assert_flat(name: &str, unit: &str, small: (u64, u64), large: (u64, u64)) {
```

<!-- annotation -->
> **envelopes-a-2** (38), line 1595:
>
> Same sweep in the text-kernel section.

<!-- annotation -->
> **envelopes-a-6** (50), line 1533:
>
> The negative control ruling 50 names, committed in the binary: the ruled reversible library mutation (stubbing `skyline::validate_bits`) was refused by this session's permission system, so the same known-bad bodies are judged against the `SKYLINE_VALIDATE_DENSE` row here. The stub trips the touch tripwire first (touches 0 under floor 2); a validator reading half the stream trips exactly the whole-input scan floor (187506 under 375000). Both messages are in the step-4 commit.

<!-- annotation -->
> **envelopes-a-4** (4), line 1601:
>
> `skyline_render_records_zero_touches` folds into the `SKYLINE_RENDER_*` rows' touch cells (pinned 0/0 since step 3); its rationale, the conservation tripwire on the text seam, moves to the section that owns the rows. The test's wide-arming case had no envelope row and is not carried: reported in the lane report as a candidate row.

<!-- annotation -->
> **envelopes-a-11** (4), line 1570:
>
> The round-trip equality the dissolved decoder rows carried stays as a plain unit test, since `skyline::decode` remains in the tests' vocabulary.

<!-- annotation -->
> **meter-adequacy-11** (108), line 1533:
>
> The probe's `cfg(feature = "scan-meter")` gate is gone with the guard: the feature is always on.

<!-- annotation -->
> **fresh-eyes-F5** (50), line 1539:
>
> Both known-bad validators are judged against the row with its limb and touch bands opened, so the scan floor is the assert that fires for each (the stub had tripped the touch tripwire first); both messages must name the whole-input liveness floor.

<!-- annotation -->
> **fresh-eyes-taste** (105), line 1567:
>
> The doc no longer enumerates the shapes the loop already lists.

<!-- annotation -->
> **envelopes-a-4** (108), line 1615:
>
> The render side of the wide-arming dual, which the folded test had covered and no row did: `Shape::WideArming.packed2(512, 512)` rendered through the text kernel, every cell pinned from the parent library's reading (in the commit message), the touch cell at zero.

<!-- annotation -->
> **envelopes-a-4** (108), line 1704:
>
> The scenario beside the other render rows; the scale constant sits with the scenario sizes.

<!-- annotation -->
> **envelopes-a-4** (4), line 1727:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<!-- annotation -->
> **envelopes-a-4** (4), line 1750:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<!-- annotation -->
> **envelopes-a-4** (4), line 1770:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<!-- annotation -->
> **envelopes-a-4** (4), line 1792:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<a id="hunk-34"></a>
### crates/before/tests/meter.rs `@@ -2632,43 +2113,6 @@ mod skyline_flatness {`

```diff
@@ -2632,43 +2113,6 @@ mod skyline_flatness {
         );
     }
 
-    /// Rendering records exactly zero accumulator digit touches: the
-    /// renderer's relative-coordinate summaries carry no running
-    /// accumulator, and this pin is the conservation tripwire on the
-    /// text seam.
-    ///
-    /// The parse direction carries the touch floor and flatness pins
-    /// above; the render direction pins the measured zero, so render
-    /// adopting an accumulator — or parse-side accumulator work leaking
-    /// into the render path — moves a pinned constant instead of
-    /// arriving silently. Re-pin only with the derivation that prices
-    /// the new accumulator work.
-    #[test]
-    fn skyline_render_records_zero_touches() {
-        for (packed, name) in [
-            (Shape::Dense.packed1(4_096), "dense"),
-            (Shape::CliffComb.packed2(512, 512), "cliff"),
-            (Shape::Bigroot.packed2(8_000, 2_000), "bigroot"),
-            // The parse direction's dual: the render walks the same
-            // wide-swing-then-dense-trail stream through its summary
-            // merges, and records zero accumulator work doing it.
-            (Shape::WideArming.packed2(512, 512), "wide_arming"),
-        ] {
-            let v = packed.version();
-            let enc = meter::skyline::encode(&v);
-            touch_meter::reset();
-            let out = meter::skyline::text::render(meter::skyline::view(&enc));
-            assert!(!out.is_empty(), "the render does real work");
-            assert_eq!(
-                touch_meter::touches(),
-                0,
-                "skyline_render_{name}: the renderer touched the accumulator; its \
-                 delta-sized summaries are priced by the limb and heap columns, so new \
-                 accumulator work here needs its own pin, not a silent arrival"
-            );
-        }
-    }
-
     /// Tooth width (bits) one notch under the rank freeze threshold's
     /// 256-bit digit bound: the band's flat side.
     const FREEZE_BAND_UNDER_BITS: usize = 192;
```

<!-- annotation -->
> **envelopes-a-4** (4), line 2116:
>
> Deletion: `skyline_render_records_zero_touches` stood here; the `SKYLINE_RENDER_*` rows' touch cells pin the zero it asserted, and the text-kernel section carries its rationale. The band module's other tests are untouched.

<a id="hunk-35"></a>
### crates/before/tests/meter.rs `@@ -6103,8 +5547,8 @@ fn id_pair_input_bytes(a: &meter::Packed, b: &meter::Packed) -> usize {`

```diff
@@ -6103,8 +5547,8 @@ fn id_pair_input_bytes(a: &meter::Packed, b: &meter::Packed) -> usize {
 }
 
 /// Joining the diverted id-spine pair stays within its envelope (the
-/// two-tree walk runs to full lockstep depth; its iterative frames must
-/// grow no stack segments) and produces the construction-known bytes.
+/// two-tree walk runs to full lockstep depth) and produces the
+/// construction-known bytes.
 ///
 /// The output pin: the divert arms are the two children of the node at
 /// depth `d − 1`, so their union collapses that node to a terminal — the
```

<!-- annotation -->
> **envelopes-a-2** (38), line 5550:
>
> The id-spine docs (`id_join`, `id_covers`, `id_without`, `id_fork`) each promised "must grow no stack segments"; the promise is now the file doc's, so the docs keep only what the row prices.

<a id="hunk-36"></a>
### crates/before/tests/meter.rs `@@ -6129,8 +5573,7 @@ fn id_join_envelope() {`

```diff
@@ -6129,8 +5573,7 @@ fn id_join_envelope() {
 }
 
 /// `covers` over the diverted id-spine pair stays within its envelope (the
-/// two-tree walk runs to full lockstep depth; its iterative frames must
-/// grow no stack segments).
+/// two-tree walk runs to full lockstep depth).
 #[test]
 fn id_covers_envelope() {
     let pa = Shape::IdSpine.packed_flagged(ID_DEPTH, false);
```

<!-- annotation -->
> **envelopes-a-2** (38), line 5576:
>
> `id_covers`'s doc promised "its iterative frames must grow no stack segments"; the promise is the file doc's now, so the doc keeps only the walk it prices.

<a id="hunk-37"></a>
### crates/before/tests/meter.rs `@@ -6243,9 +5686,8 @@ fn packed_bit(bytes: &[u8], i: usize) -> bool {`

```diff
@@ -6243,9 +5686,8 @@ fn packed_bit(bytes: &[u8], i: usize) -> bool {
     bytes[i / 8] & (0x80 >> (i % 8)) != 0
 }
 
-/// `without` subtracting an id spine from the seed stays within its envelope
-/// (the sweep is iterative, so the subtrahend's depth alone must grow no
-/// stack segments).
+/// `without` subtracting an id spine from the seed stays within its
+/// envelope: the complement sweep runs the subtrahend's full depth.
 #[test]
 fn id_without_envelope() {
     let pb = Shape::IdSpine.packed_flagged(ID_DEPTH, true);
```

<!-- annotation -->
> **envelopes-a-2** (38), line 5690:
>
> `id_without`'s doc promised "must grow no stack segments"; restated to what the row prices.

<a id="hunk-38"></a>
### crates/before/tests/meter.rs `@@ -6394,8 +5836,7 @@ mod id_walk_scan_cost {`

```diff
@@ -6394,8 +5836,7 @@ mod id_walk_scan_cost {
 
 // ─── fork envelope (the split kernel's committed cost record) ───────────────
 
-/// The fork envelope: measured ×1.25 (dev profile, the envelope
-/// suite's convention).
+/// The fork envelope.
 ///
 /// The split kernel builds both halves by raw bit-slice writes and walks
 /// the spine by raw indexing — deliberately outside the scan primitives —
```

<!-- annotation -->
> **envelopes-a-4** (4), line 5839:
>
> The fork table's doc no longer restates the pin convention; the file doc holds it once.

<a id="hunk-39"></a>
### crates/before/tests/meter.rs `@@ -6405,17 +5846,16 @@ mod id_walk_scan_cost {`

```diff
@@ -6405,17 +5846,16 @@ mod id_walk_scan_cost {
 /// column is the one that prices the halves' materialization.
 #[rustfmt::skip]
 mod fork_env {
-    use super::{sweep_envelope, SweepEnvelope};
-    //                                                    peak heap, segments, limb ops, scan bits, limb floor
-    pub const ID_FORK: SweepEnvelope = sweep_envelope(      156_253,        0,        0,         3, 0); // the heap column prices both halves' materialization (~2x the packed input); the scan ceiling pins the raw split path's near-zero reading
+    use super::{band, envelope, Envelope};
+    pub const ID_FORK: Envelope = envelope(156_253, band(0, 0), band(0, 0), band(3, 1)); // the heap column prices both halves' materialization (~2x the packed input); the scan ceiling pins the raw split path's near-zero reading
 }
 
 /// Forking the deep id spine stays within its envelope, and the halves
 /// rejoin into the original party byte for byte.
 ///
 /// Fork is the one id operation with no committed cost record: its halves
-/// materialize (the heap column prices them), its spine walk is iterative
-/// (zero segments), and its writes are raw (the scan pin above). The
+/// materialize (the heap column prices them), its spine walk is iterative,
+/// and its writes are raw (the scan pin above). The
 /// rejoin closes the semantic leg: fork then join is the identity.
 #[test]
 fn id_fork_envelope() {
```

<!-- annotation -->
> **envelopes-b-8** (4), line 5848:
>
> `ID_FORK` in the unified form, every column pinned (its touch cell measured 0, its scan cell keeps the raw-path pin of 3 over a tripwire of 1); `id_fork`'s doc drops "(zero segments)".

<a id="hunk-40"></a>
### crates/before/tests/meter.rs `@@ -6423,7 +5863,7 @@ fn id_fork_envelope() {`

```diff
@@ -6423,7 +5863,7 @@ fn id_fork_envelope() {
     let input = pa.bytes.len();
     let original = pa.bytes.clone();
     let mut a = party_of(&pa);
-    let child = sweep_metered("id_fork", input, &fork_env::ID_FORK, || a.fork());
+    let child = metered("id_fork", input, &fork_env::ID_FORK, || a.fork());
     a.join(child).expect("a fork's halves are disjoint");
     assert_eq!(
         a.encode(),
```

<!-- annotation -->
> **envelopes-a-4** (4), line 5866:
>
> Call site renamed from `sweep_metered` to `metered`; no reading moved.

<a id="hunk-41"></a>
### crates/before/tests/meter.rs `@@ -6724,12 +6164,10 @@ mod accum_streams {`

```diff
@@ -6724,12 +6164,10 @@ mod accum_streams {
 //
 // The query kernels over skyline streams: rank on the anchored-segment
 // height split, min_ticks on the range-minimum anchor web and its epoch
-// ledger, and
-// projection against a packed id. Streams are transcoded outside
-// measurement. These rows carry all five columns — heap, segments, limbs,
-// scanned bits, and accumulator touches — because the kernels' arithmetic
-// lives in digit touches (the limb column alone would read a vacuous
-// near-zero), while their stream work lives in the scan column. The cliff
+// ledger, and projection against a packed id. Streams are transcoded
+// outside measurement. The kernels' arithmetic lives in digit touches (the
+// limb column alone reads a vacuous near-zero) and their stream work in
+// the scan column. The cliff
 // and wide-tooth rank rows are load-bearing live-path pins: wide deltas
 // ride the live component without freezing — the comb's terminal borrow
 // and every 192-bit tooth are each paid by their own codes — and the
```

<!-- annotation -->
> **envelopes-b-8** (4), line 6169:
>
> The query section no longer says these rows "carry all five columns" as the reason for a separate harness: every table carries every column now, so the sentence states only where the kernels' work lives.

<a id="hunk-42"></a>
### crates/before/tests/meter.rs `@@ -6740,126 +6178,45 @@ mod accum_streams {`

```diff
@@ -6740,126 +6178,45 @@ mod accum_streams {
 // mandatory and dominates its input, so the pinned ceilings price
 // input + output bytes (the MEASURED line prints both).
 
-/// One query scenario's pinned ceilings: [`Envelope`]'s three columns
-/// plus scanned bits and accumulator touches, asserted when their
-/// features are lit.
-struct QueryEnvelope {
-    /// Peak heap delta over the scenario body, in bytes.
-    peak_heap: usize,
-    /// Stack segments grown during the scenario body.
-    segments: u64,
-    /// Big-integer limb operations counted during the scenario body.
-    #[cfg(feature = "limb-meter")]
-    limb_ops: u64,
-    /// Packed-stream bits scanned during the scenario body.
-    #[cfg(feature = "scan-meter")]
-    scan_bits: u64,
-    /// Accumulator digit touches counted during the scenario body.
-    #[cfg(feature = "limb-meter")]
-    touches: u64,
-    /// Improvement tripwire under the limb column: measured ×0.75, per
-    /// the file doc's tripwire convention.
-    #[cfg(feature = "limb-meter")]
-    limb_floor: u64,
-    /// Improvement tripwire under the touch column: measured ×0.75, per
-    /// the file doc's tripwire convention.
-    ///
-    /// A touch reading below it is a >25% drop from the pinned reading —
-    /// attribute it, re-pinning an honest improvement or curing a dead
-    /// meter, without which every touch ceiling above would hold
-    /// vacuously (zero where the measured count is zero, under which the
-    /// bound asserts nothing).
-    #[cfg(feature = "limb-meter")]
-    touch_floor: u64,
-}
-
-/// Build a [`QueryEnvelope`] from the five pinned columns and the limb
-/// and touch floors.
-///
-/// The limb, scan, and touch columns are carried only when their features
-/// compile the counters in; the leading underscores keep the parameters
-/// warning-free in the other configurations.
-const fn query_envelope(
-    peak_heap: usize,
-    segments: u64,
-    _limb_ops: u64,
-    _scan_bits: u64,
-    _touches: u64,
-    _limb_floor: u64,
-    _touch_floor: u64,
-) -> QueryEnvelope {
-    QueryEnvelope {
-        peak_heap,
-        segments,
-        #[cfg(feature = "limb-meter")]
-        limb_ops: _limb_ops,
-        #[cfg(feature = "scan-meter")]
-        scan_bits: _scan_bits,
-        #[cfg(feature = "limb-meter")]
-        touches: _touches,
-        #[cfg(feature = "limb-meter")]
-        limb_floor: _limb_floor,
-        #[cfg(feature = "limb-meter")]
-        touch_floor: _touch_floor,
-    }
-}
-
-// The query envelope table: pinned ceiling = measured ×1.25, rounded up
-// (aarch64-apple-darwin, dev profile, three identical runs), and only
-// ever tightened: where a remeasure rises while staying inside an
-// existing ceiling (the bigroot heap and touch cells, whose frozen
-// component lives on the accumulator), the older, tighter ceiling
-// stands. The trailing comment on each line states the mechanism that
-// prices the row; the measurements of record — and every re-pin's
-// movement and attribution — live in the pin commits (`git log -S` the
-// constant). Re-pin by rerunning under `--no-capture` with
-// `--all-features` and reading the MEASURED lines.
-// The limb floor column is the measured value ×0.75, rounded down (the
-// file doc's improvement-tripwire convention).
+// Pins per the file doc's convention; each row's trailing comment states
+// the mechanism that prices it.
 #[rustfmt::skip]
 mod query_env {
-    use super::{query_envelope, QueryEnvelope};
-    //                                                                        peak heap, segments,  limb ops, scan bits,   touches, limb floor, touch floor
-    pub const SKYLINE_RANK_DENSE: QueryEnvelope = query_envelope(30_720, 0, 4, 937_515, 7, 2, 3); // the depth control: path bits and near-zero arithmetic; the max_depth pre-scan records each payload skip once, and word-valued payloads keep the work columns near zero
-    pub const SKYLINE_RANK_BIGROOT: QueryEnvelope = query_envelope(67_145, 0, 2_739, 275_023, 8_993, 1_643, 5_395); // the wide-magnitude control: the first leaf's magnitude seeds the frozen component and is read once, in the closing shifted add
-    pub const SKYLINE_RANK_HARMONIC: QueryEnvelope = query_envelope(52_500, 0, 2_562, 491_530, 248_285, 1_536, 148_971); // the separating family: each level's one-leaf delta lands at its own weight; the segment feed opens only at the first freeze
-    pub const SKYLINE_RANK_CLIFF: QueryEnvelope = query_envelope(2_855, 0, 172, 35_845, 6_688, 102, 4_012); // the live component absorbs the oscillation at O(1) digits per fold; the terminal borrow rides it into one wide add, no freeze
-    pub const SKYLINE_RANK_WIDE_TOOTH: QueryEnvelope      = query_envelope(     3_635,        0,    29_552, 2_000_960,    24_585, 17_755, 14_751); // the no-freeze pin: every fold paid by its tooth's own code; certificate skips replace zero-run walks, and the pre-scan records each payload skip once on this payload-dominated comb
+    use super::{band, envelope, Envelope};
+    pub const SKYLINE_RANK_CLIFF: Envelope           = envelope(  2_855,        band(172, 102),     band(6_688, 4_012),       band(35_845, 21_507)); // the live component absorbs the oscillation at O(1) digits per fold; the terminal borrow rides it into one wide add, no freeze
+    pub const SKYLINE_RANK_WIDE_TOOTH: Envelope      = envelope(  3_635,  band(29_552, 17_755),   band(24_585, 14_751), band(2_000_960, 1_200_576)); // the no-freeze pin: every fold paid by its tooth's own code; certificate skips replace zero-run walks, and the pre-scan records each payload skip once on this payload-dominated comb
     // The practical-regime gauge: `Version::rank`
     // on one concurrent-pair operand — word-scale heights over organic
     // forks, no freeze, no arming. The row pins the benign path's
     // constants so the adversarial machinery's price on common inputs is
     // a committed number, not a vibe.
-    pub const RANK_CONCURRENT: QueryEnvelope = query_envelope(0, 0, 4, 61_448, 11_099, 2, 6_659); // word-scale heights: zero heap, near-zero limb work, one walk's scan and touches
-    pub const TICKS_DENSE: QueryEnvelope = query_envelope(58_815, 0, 8, 468_809, 156_270, 4, 93_762); // the tick row's cost plus the count's gamma codes
-    pub const TICKS_NESTED_WIDE: QueryEnvelope = query_envelope(14_107, 0, 323, 150_072, 31_125, 193, 18_675); // the fill branch pays its documented second walk: scan ~2x the tick row's one walk
-    pub const TICKS_MIRROR_WIDE: QueryEnvelope = query_envelope(39_506, 0, 723, 220_048, 72_582, 433, 43_548); // second-walk fill branch, as the nested-wide row; the pre-scan records minima only, so the per-site collapse re-read and raise-mirror folds stay out of the scan and touch columns
-    pub const SKYLINE_MIN_TICKS_DENSE: QueryEnvelope = query_envelope(30_720, 0, 5, 468_758, 312_508, 3, 187_504); // every delta folds into two accumulators — the live height and the web's gap — so touches run ~2x the rank row's with no minima circulation
-    pub const SKYLINE_MIN_TICKS_CLIFF: QueryEnvelope = query_envelope(3_530, 0, 180, 17_923, 12_000, 108, 7_200); // the comb's wide F-relative pending offsets are epoch-ledger counts, and the wide first height enters the exact total once, through the counting term
-    pub const SKYLINE_MIN_TICKS_ASCEND: QueryEnvelope = query_envelope(553_660, 0, 33, 12_823, 20_044, 19, 12_026); // the boundary-stacking row: the anchor web's per-boundary word compaction's measured basis — with compaction deleted the same body reads well over both the heap and touch ceilings
-    pub const SKYLINE_PROJECT_COMB_SCATTER: QueryEnvelope = query_envelope(   525_700,        0,   115_265, 2_652_165,    44_924, 69_159, 26_954); // output-dominated: the pinned ceilings price input + output bytes; id tags are single records
-    pub const FOLD_VERSION_SCATTER: QueryEnvelope = query_envelope(323, 0, 0, 330_913, 61_429, 0, 36_857); // the balanced reduction: near-linear in the population's packed bytes where a left fold re-scans its whole accumulator per input; the at-rest form is a length-carrying container of the wire bytes, cloned by refcount in the fold's lone-group settle and adoption arms, and the counter stack's entries carry the operand-form tag (~8 B per level)
-    pub const FOLD_PARTY_SCATTER: QueryEnvelope          = query_envelope(       780,        0,         0,   322_068,         0, 0, 0); // pure stream scanning: join_all answers its up-front tests through a per-call id index, the id walk does no arithmetic, and one refcount control block per frozen stream lives in the fold's groups
-    // Tick rows, on the five-meter harness: the tick walk's cost
-    // currency is accumulator digit touches (with scanned bits beside
-    // it), which the four-column table never watched. Ceilings ×1.25
-    // and floors ×0.75 over the measurements of record in the pin
-    // commits.
-    pub const TICK_DENSE: QueryEnvelope = query_envelope(58_815, 0, 0, 468_765, 156_265, 0, 93_759); // the fused tick: copy-on-first-divergence defers the output buffer past the collapse scan, so the scan path and the builder never coexist at peak
-    pub const TICK_NESTED_WIDE: QueryEnvelope = query_envelope(14_108, 0, 239, 80_028, 30_808, 143, 18_484); // the explicit-stack walk: suspended ancestors ride metered frame bits, zero grown segments (the zero pin is the ratchet); the anchor web reads the wide first payload O(1) times
-    pub const TICK_MIRROR_WIDE: QueryEnvelope = query_envelope(32_467, 0, 398, 160_003, 71_955, 238, 43_173); // the frame ledger stores no link for the shared wide minimum (heap parity with one queue word per site); the pre-scan records minima only, so the per-site collapse re-read and raise-mirror folds stay out of the scan and touch columns
+    pub const RANK_CONCURRENT: Envelope              = envelope(      0,            band(4, 2),    band(11_099, 6_659),       band(61_448, 36_868)); // word-scale heights: zero heap, near-zero limb work, one walk's scan and touches
+    pub const TICKS_DENSE: Envelope                  = envelope( 58_815,            band(8, 4),  band(156_270, 93_762),     band(468_809, 281_285)); // the tick row's cost plus the count's gamma codes
+    pub const TICKS_NESTED_WIDE: Envelope            = envelope( 14_107,        band(323, 193),   band(31_125, 18_675),      band(150_072, 90_042)); // the fill branch pays its documented second walk: scan ~2x the tick row's one walk
+    pub const TICKS_MIRROR_WIDE: Envelope            = envelope( 39_506,        band(723, 433),   band(72_582, 43_548),     band(220_048, 132_028)); // second-walk fill branch, as the nested-wide row; the pre-scan records minima only, so the per-site collapse re-read and raise-mirror folds stay out of the scan and touch columns
+    pub const SKYLINE_MIN_TICKS_DENSE: Envelope      = envelope( 30_720,            band(5, 3), band(312_508, 187_504),     band(468_758, 281_254)); // every delta folds into two accumulators — the live height and the web's gap — so touches run ~2x the rank row's with no minima circulation
+    pub const SKYLINE_MIN_TICKS_CLIFF: Envelope      = envelope(  3_530,        band(180, 108),    band(12_000, 7_200),       band(17_923, 10_753)); // the comb's wide F-relative pending offsets are epoch-ledger counts, and the wide first height enters the exact total once, through the counting term
+    pub const SKYLINE_MIN_TICKS_ASCEND: Envelope     = envelope(553_660,          band(33, 19),   band(20_044, 12_026),        band(12_823, 7_693)); // the boundary-stacking row: the anchor web's per-boundary word compaction's measured basis — with compaction deleted the same body reads well over both the heap and touch ceilings
+    pub const SKYLINE_PROJECT_COMB_SCATTER: Envelope = envelope(525_700, band(115_265, 69_159),   band(44_924, 26_954), band(2_652_165, 1_591_299)); // output-dominated: the pinned ceilings price input + output bytes; id tags are single records
+    pub const FOLD_VERSION_SCATTER: Envelope         = envelope(    323,            band(0, 0),   band(61_429, 36_857),     band(330_913, 198_547)); // the balanced reduction: near-linear in the population's packed bytes where a left fold re-scans its whole accumulator per input; the at-rest form is a length-carrying container of the wire bytes, cloned by refcount in the fold's lone-group settle and adoption arms, and the counter stack's entries carry the operand-form tag (~8 B per level)
+    pub const FOLD_PARTY_SCATTER: Envelope           = envelope(    780,            band(0, 0),             band(0, 0),     band(322_068, 193_240)); // pure stream scanning: join_all answers its up-front tests through a per-call id index, the id walk does no arithmetic, and one refcount control block per frozen stream lives in the fold's groups
+    // The tick rows: the tick walk's cost currency is accumulator digit
+    // touches, with scanned bits beside it.
+    pub const TICK_DENSE: Envelope                   = envelope( 58_815,            band(0, 0),  band(156_265, 93_759),     band(468_765, 281_259)); // the fused tick: copy-on-first-divergence defers the output buffer past the collapse scan, so the scan path and the builder never coexist at peak
+    pub const TICK_NESTED_WIDE: Envelope             = envelope( 14_108,        band(239, 143),   band(30_808, 18_484),       band(80_028, 48_016)); // the explicit-stack walk: suspended ancestors ride metered frame bits; the anchor web reads the wide first payload O(1) times
+    pub const TICK_MIRROR_WIDE: Envelope             = envelope( 32_467,        band(398, 238),   band(71_955, 43_173),      band(160_003, 96_001)); // the frame ledger stores no link for the shared wide minimum (heap parity with one queue word per site); the pre-scan records minima only, so the per-site collapse re-read and raise-mirror folds stay out of the scan and touch columns
     // The expansion rows: grow-branch deep
     // ticks measuring the whole public tick — walk, route fold, and
     // splice — in one fused pass.
-    pub const TICK_OWNERSHIP_HOLE: QueryEnvelope = query_envelope(3_647, 0, 0, 37_585, 7_563, 0, 4_537); // the ownership-gated block scan: unowned staircase runs fold as one net-and-minimum summary each; the touch ceiling sits below the leaf-by-leaf mechanism's reading, so the skip must engage for the pin to hold, and the scan column holds every skipped bit still read
-    pub const TICK_OWNERSHIP_COMB: QueryEnvelope = query_envelope(59_575, 0, 0, 498_774, 156_275, 0, 93_765); // readings identical to the ungated per-leaf walk's on this family (single-leaf regions everywhere, so the block gate never opens and may cost nothing when closed)
-    pub const TICK_COLLAPSE_HOLE: QueryEnvelope = query_envelope(2_748, 0, 0, 14_368, 8_125, 0, 4_875); // the descend-arm consuming max scan rides the block summary over each deep collapse range, its only crossing; rerouting either lead's ranges to the per-leaf fold reads touches over the ceiling, and the scan column holds every folded bit still read
-    pub const TICK_COPY_HOLE: QueryEnvelope = query_envelope(1_733, 0, 18, 53_302, 15_615, 10, 9_369); // the pre-scan copies each untouched range as one net movement and one watermark emission; rerouting either lead's ranges to per-leaf virtual emissions reads touches over the ceiling, and the scan column holds every folded bit still read
-    pub const TICK_RAISE_HOLE: QueryEnvelope = query_envelope(2_660, 0, 0, 13_543, 8_030, 0, 4_818); // the ascend-arm consuming max scan rides the block summary over each deep raised range, its only crossing; rerouting either lead's ranges to the per-leaf fold reads touches over the ceiling, and the scan column holds every folded bit still read
-    pub const TICK_SITE_HOLE: QueryEnvelope = query_envelope(2_768, 0, 0, 27_962, 10_779, 0, 6_467); // the pre-scan's collapse skip and the walk's consuming max scan each cross every deep range once as one block fold, and the collapse skip's fold accumulates the net movement alone; a block fold that also streams the range's unread minimum reads touches over the ceiling, and the scan column holds every folded bit still read
-    pub const MASKED_CMP_HOLE: QueryEnvelope = query_envelope(480, 0, 0, 7_535, 18, 0, 10); // the block skip consumes the spine's unowned continuation whole: the touch reading is a function of the mask depth alone; a per-boundary walk reads ~one touch per spine boundary, orders over the ceiling — the depth band beside this row holds the reading flat across a spine-depth doubling
-    pub const TICK_EXPAND_SPINE: QueryEnvelope = query_envelope(435_435, 0, 5, 2_187_519, 0, 3, 0); // an empty version's tick folds one word-scale payload: near-zero accumulator work; the emit codes the whole expansion chain as fresh one-bit deltas
-    pub const TICK_EXPAND_CROSS: QueryEnvelope = query_envelope(611_210, 0, 5, 3_593_782, 156_260, 3, 93_756); // the mixed regimes: the fused walk down the shared spine plus the id-only expansion fold, spliced in one pass
+    pub const TICK_OWNERSHIP_HOLE: Envelope          = envelope(  3_647,            band(0, 0),     band(7_563, 4_537),       band(37_585, 22_551)); // the ownership-gated block scan: unowned staircase runs fold as one net-and-minimum summary each; the touch ceiling sits below the leaf-by-leaf mechanism's reading, so the skip must engage for the pin to hold, and the scan column holds every skipped bit still read
+    pub const TICK_OWNERSHIP_COMB: Envelope          = envelope( 59_575,            band(0, 0),  band(156_275, 93_765),     band(498_774, 299_264)); // readings identical to the ungated per-leaf walk's on this family (single-leaf regions everywhere, so the block gate never opens and may cost nothing when closed)
+    pub const TICK_COLLAPSE_HOLE: Envelope           = envelope(  2_748,            band(0, 0),     band(8_125, 4_875),        band(14_368, 8_620)); // the descend-arm consuming max scan rides the block summary over each deep collapse range, its only crossing; rerouting either lead's ranges to the per-leaf fold reads touches over the ceiling, and the scan column holds every folded bit still read
+    pub const TICK_COPY_HOLE: Envelope               = envelope(  1_733,          band(18, 10),    band(15_615, 9_369),       band(53_302, 31_980)); // the pre-scan copies each untouched range as one net movement and one watermark emission; rerouting either lead's ranges to per-leaf virtual emissions reads touches over the ceiling, and the scan column holds every folded bit still read
+    pub const TICK_RAISE_HOLE: Envelope              = envelope(  2_660,            band(0, 0),     band(8_030, 4_818),        band(13_543, 8_125)); // the ascend-arm consuming max scan rides the block summary over each deep raised range, its only crossing; rerouting either lead's ranges to the per-leaf fold reads touches over the ceiling, and the scan column holds every folded bit still read
+    pub const TICK_SITE_HOLE: Envelope               = envelope(  2_768,            band(0, 0),    band(10_779, 6_467),       band(27_962, 16_776)); // the pre-scan's collapse skip and the walk's consuming max scan each cross every deep range once as one block fold, and the collapse skip's fold accumulates the net movement alone; a block fold that also streams the range's unread minimum reads touches over the ceiling, and the scan column holds every folded bit still read
+    pub const MASKED_CMP_HOLE: Envelope              = envelope(    480,            band(0, 0),           band(18, 10),         band(7_535, 4_521)); // the block skip consumes the spine's unowned continuation whole: the touch reading is a function of the mask depth alone; a per-boundary walk reads ~one touch per spine boundary, orders over the ceiling — the depth band beside this row holds the reading flat across a spine-depth doubling
+    pub const TICK_EXPAND_SPINE: Envelope            = envelope(435_435,            band(5, 3),             band(0, 0), band(2_187_519, 1_312_511)); // an empty version's tick folds one word-scale payload: near-zero accumulator work; the emit codes the whole expansion chain as fresh one-bit deltas
+    pub const TICK_EXPAND_CROSS: Envelope            = envelope(611_210,            band(5, 3),  band(156_260, 93_756), band(3_593_782, 2_156_268)); // the mixed regimes: the fused walk down the shared spine plus the id-only expansion fold, spliced in one pass
     // The version-pair rows: the public
     // two-operand queries on the pair families (the corpus pairing
     // `w = v + one seed tick` collapses the second operand onto a
```

<!-- annotation -->
> **envelopes-b-8** (4), line 6184:
>
> `QueryEnvelope`, `query_envelope`, the table's own preamble, and `query_metered` are deleted; the table cites the file doc, its rows keep their numbers in the unified form (scan floors added in step 4), and the two inner comments that restated the convention or the tables' history are shortened to what the rows are.

<a id="hunk-43"></a>
### crates/before/tests/meter.rs `@@ -6872,329 +6229,519 @@ mod query_env {`

```diff
@@ -6872,329 +6229,519 @@ mod query_env {
     // accumulator instead of skipping the meet leg, which is what buys
     // its heap, limb, and scan columns down to the distance row's
     // neighborhood.
-    pub const DISTANCE_JUMP_PAIR: QueryEnvelope = query_envelope(5_750, 0, 48_714, 2_694_095, 208_749, 29_228, 125_249); // the fused co-sweep with cluster-delegated settle products and certificate skips; this pair freezes early, so the segment feed's deposits are the pre-freeze prefix alone, and the max_depth pre-scan records each payload skip once, twice per pair walk
-    pub const LAG_JUMP_PAIR: QueryEnvelope = query_envelope(5_750, 0, 45_420, 2_694_095, 173_492, 27_252, 104_094); // the one-sided functional over the same fused co-sweep as the distance row
-    pub const DISTANCE_CONCURRENT: QueryEnvelope = query_envelope(0, 0, 4, 117_753, 32_429, 2, 19_457); // orientation-switch density on word-scale heights: the pair never freezes, so no segment feed deposits
-    pub const LAG_CONCURRENT: QueryEnvelope = query_envelope(0, 0, 4, 117_753, 33_278, 2, 19_966); // the one-sided functional over the same switch-dense overlay
+    pub const DISTANCE_JUMP_PAIR: Envelope           = envelope(  5_750,  band(48_714, 29_228), band(208_749, 125_249), band(2_694_095, 1_616_457)); // the fused co-sweep with cluster-delegated settle products and certificate skips; this pair freezes early, so the segment feed's deposits are the pre-freeze prefix alone, and the max_depth pre-scan records each payload skip once, twice per pair walk
+    pub const LAG_JUMP_PAIR: Envelope                = envelope(  5_750,  band(45_420, 27_252), band(173_492, 104_094), band(2_694_095, 1_616_457)); // the one-sided functional over the same fused co-sweep as the distance row
+    pub const DISTANCE_CONCURRENT: Envelope          = envelope(      0,            band(4, 2),   band(32_429, 19_457),      band(117_753, 70_651)); // orientation-switch density on word-scale heights: the pair never freezes, so no segment feed deposits
+    pub const LAG_CONCURRENT: Envelope               = envelope(      0,            band(4, 2),   band(33_278, 19_966),      band(117_753, 70_651)); // the one-sided functional over the same switch-dense overlay
     // The masked-comparison rows:
     // the fused projected comparisons on the correlated mask-drift
     // families, priced input-only on shapes whose *materialization* is
-    // product-growth — the laziness the view exists for. Ceilings x1.25
-    // and floors x0.75 over the measurements of record in the pin
-    // commits.
-    pub const MASKED_CMP_DRIFT_TRIPLE: QueryEnvelope = query_envelope(1_570, 0, 59, 20_488, 5_240, 35, 3_144); // one pass over the overlay, ~2 touches per stored delta
-    pub const MASKED_CMP_DRIFT_QUAD: QueryEnvelope        = query_envelope(     2_720,        0,    39_722, 1_342_092,    83_946, 23_833, 50_367); // the sparse comb's wide climb/drop codes dominate the input; scan ~8 bits per input byte
+    // product-growth — the laziness the view exists for.
+    pub const MASKED_CMP_DRIFT_TRIPLE: Envelope      = envelope(  1_570,          band(59, 35),     band(5_240, 3_144),       band(20_488, 12_292)); // one pass over the overlay, ~2 touches per stored delta
+    pub const MASKED_CMP_DRIFT_QUAD: Envelope        = envelope(  2_720,  band(39_722, 23_833),   band(83_946, 50_367),   band(1_342_092, 805_255)); // the sparse comb's wide climb/drop codes dominate the input; scan ~8 bits per input byte
 }
 
-/// Run one query scenario body under all five meters and assert its
+/// The rank kernel on the boundary comb's skyline stays within its
 /// envelope.
 ///
-/// [`sweep_metered`]'s harness plus the accumulator touch column; prints
-/// the measured numbers so re-pinning never requires editing the harness.
-fn query_metered<R>(
-    name: &str,
-    input_bytes: usize,
-    env: &QueryEnvelope,
-    f: impl FnOnce() -> R,
-) -> R {
-    meter::reset_stack_segments();
-    #[cfg(feature = "limb-meter")]
-    meter::reset_limb_ops();
-    #[cfg(feature = "limb-meter")]
-    suanpan::touch_meter::reset();
-    #[cfg(feature = "scan-meter")]
-    meter::reset_scan_bits();
-    HEAP.reset_peak_usage();
-    let baseline = HEAP.current_usage();
-    let r = f();
-    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
-    let segments = meter::stack_segments();
-    #[cfg(feature = "limb-meter")]
-    let limb_ops = meter::limb_ops();
-    #[cfg(feature = "limb-meter")]
-    let touches = suanpan::touch_meter::touches();
-    #[cfg(feature = "scan-meter")]
-    let scan_bits = meter::scan_bits();
-    #[cfg(feature = "limb-meter")]
-    let limb_col = format!(" limb_ops={limb_ops} touches={touches}");
-    #[cfg(not(feature = "limb-meter"))]
-    let limb_col = "";
-    #[cfg(feature = "scan-meter")]
-    let scan_col = format!(" scan_bits={scan_bits}");
-    #[cfg(not(feature = "scan-meter"))]
-    let scan_col = "";
-    eprintln!(
-        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments}{limb_col}{scan_col}"
-    );
-    assert!(
-        peak_heap <= env.peak_heap,
-        "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B): {ISOLATION_NOTE}",
-        env.peak_heap,
+/// The heights are `2^k`-scale behind 3-bit deltas, the live component
+/// absorbs the oscillation at O(1) digits per fold, and the terminal
+/// borrow — as wide as its own code — rides the live component into the
+/// last leaf's single wide add, no freeze anywhere.
+#[test]
+fn skyline_rank_cliff_envelope() {
+    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
+    let v = version_of(&p);
+    let enc = skyline_of(&p);
+    let r = metered(
+        "skyline_rank_cliff",
+        enc.as_raw_slice().len(),
+        &query_env::SKYLINE_RANK_CLIFF,
+        || meter::skyline::query::rank(meter::skyline::view(&enc)),
     );
-    assert!(
-        segments <= env.segments,
-        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.segments,
+    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
+}
+
+/// The rank kernel on the wide-tooth comb's skyline stays within its
+/// envelope — the no-freeze pin.
+///
+/// Bounded 192-bit oscillation keeps the live component exactly as wide
+/// as each tooth's own code, so every fold and every per-leaf add is paid
+/// by that code and the frozen component never churns (the
+/// `skyline_flatness` freeze-band row pins the same shape above the
+/// freeze allowance).
+#[test]
+fn skyline_rank_wide_tooth_envelope() {
+    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
+    let v = version_of(&p);
+    let enc = skyline_of(&p);
+    let r = metered(
+        "skyline_rank_wide_tooth",
+        enc.as_raw_slice().len(),
+        &query_env::SKYLINE_RANK_WIDE_TOOTH,
+        || meter::skyline::query::rank(meter::skyline::view(&enc)),
     );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        limb_ops <= env.limb_ops,
-        "{name}: {limb_ops} limb operations exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.limb_ops,
+    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
+}
+
+/// The min_ticks kernel on the dense spine stays within its envelope:
+/// one narrow offset min-merge per node, heights on the accumulator,
+/// across 125k levels.
+#[test]
+fn skyline_min_ticks_dense_envelope() {
+    let p = Shape::Dense.packed1(DENSE_DEPTH);
+    let v = version_of(&p);
+    let enc = skyline_of(&p);
+    let r = metered(
+        "skyline_min_ticks_dense",
+        enc.as_raw_slice().len(),
+        &query_env::SKYLINE_MIN_TICKS_DENSE,
+        || meter::skyline::query::min_ticks(meter::skyline::view(&enc)),
     );
-    #[cfg(feature = "scan-meter")]
-    assert!(
-        scan_bits <= env.scan_bits,
-        "{name}: {scan_bits} scanned bits exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.scan_bits,
+    assert_eq!(
+        r.to_string(),
+        v.min_ticks().to_string(),
+        "the kernel must match the packed fold"
     );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        touches <= env.touches,
-        "{name}: {touches} accumulator digit touches exceed the pinned envelope {}: {ISOLATION_NOTE}",
-        env.touches,
+}
+
+/// The min_ticks kernel on the boundary comb stays within its envelope.
+///
+/// The `2^k`-scale first height rides the frozen component and enters
+/// the exact total once, through the counting term — never per leaf —
+/// so the comb's teeth cost narrow offsets only.
+#[test]
+fn skyline_min_ticks_cliff_envelope() {
+    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
+    let v = version_of(&p);
+    let enc = skyline_of(&p);
+    let r = metered(
+        "skyline_min_ticks_cliff",
+        enc.as_raw_slice().len(),
+        &query_env::SKYLINE_MIN_TICKS_CLIFF,
+        || meter::skyline::query::min_ticks(meter::skyline::view(&enc)),
     );
-    #[cfg(feature = "limb-meter")]
     assert!(
-        limb_ops >= env.limb_floor,
-        "{name}: limb counter reads {limb_ops}, below the {} improvement \
-         tripwire (measured x0.75): attribute the drop — an honest \
-         improvement re-pins the band; a dead meter is the bypass this \
-         column exists to catch",
-        env.limb_floor,
+        r.to_string().len() > 20,
+        "the comb's floor exceeds any machine word: the wide arm is live"
     );
-    #[cfg(feature = "limb-meter")]
-    assert!(
-        touches >= env.touch_floor,
-        "{name}: touch counter reads {touches}, below the {} improvement \
-         tripwire (measured x0.75): attribute the drop — an honest \
-         improvement re-pins the band; a dead meter is the bypass this \
-         column exists to catch",
-        env.touch_floor,
+    assert_eq!(
+        r.to_string(),
+        v.min_ticks().to_string(),
+        "the kernel must match the packed fold"
     );
-    r
 }
 
-/// The rank kernel on the dense spine's skyline stays within its
-/// envelope (the depth control: 125k levels of path bits, near-zero
-/// arithmetic).
+/// The min_ticks kernel on the ascending cliff stays within its
+/// envelope — the boundary-stacking case, and the committed basis for
+/// the anchor web's per-boundary word compaction.
+///
+/// The ascending spine arms every open range one above its parent's
+/// minimum, so the web holds `ASCEND_STACK_DEPTH − 1` nonzero unit
+/// boundary differences simultaneously at the terminal cliff — the one
+/// committed min_ticks shape where per-boundary transient storage is
+/// the envelope. The heap and touch ceilings are what the compacting
+/// instantiation buys: each word-scale difference is stored inline
+/// instead of as an accumulator entry, and the terminal cliff's
+/// undercut consumes each one by an O(1) word fold instead of an
+/// accumulator hop. With compaction deleted the same body reads over
+/// both the heap and touch ceilings \[demonstrated under the live
+/// swap, same harness\], so this row is the measured basis
+/// `MinWeb::compacting` cites.
 #[test]
-fn skyline_rank_dense_envelope() {
-    let p = Shape::Dense.packed1(DENSE_DEPTH);
+fn skyline_min_ticks_ascend_envelope() {
+    let p = Shape::AscendCliff.packed2(ASCEND_STACK_DEPTH, ASCEND_STACK_MAGNITUDE_BITS);
     let v = version_of(&p);
     let enc = skyline_of(&p);
-    let r = query_metered(
-        "skyline_rank_dense",
+    let r = metered(
+        "skyline_min_ticks_ascend",
         enc.as_raw_slice().len(),
-        &query_env::SKYLINE_RANK_DENSE,
-        || meter::skyline::query::rank(meter::skyline::view(&enc)),
+        &query_env::SKYLINE_MIN_TICKS_ASCEND,
+        || meter::skyline::query::min_ticks(meter::skyline::view(&enc)),
+    );
+    // The family's closed form: k leaves at 2^b + i over spine minima
+    // all zero (the terminal cliff), so min_ticks = k·2^b + k(k+1)/2.
+    let k = ASCEND_STACK_DEPTH;
+    let expected = dashu_int::UBig::from(k as u64)
+        * (dashu_int::UBig::ONE << ASCEND_STACK_MAGNITUDE_BITS)
+        + dashu_int::UBig::from((k * (k + 1) / 2) as u64);
+    assert_eq!(
+        r.to_string(),
+        expected.to_string(),
+        "min_ticks disagrees with the ascending cliff's closed form"
+    );
+    assert_eq!(
+        r.to_string(),
+        v.min_ticks().to_string(),
+        "the kernel must match the packed fold"
     );
-    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
 }
 
-/// The rank kernel on the bigroot skyline stays within its envelope (the
-/// wide-magnitude control).
+/// The projection kernel on the comb × scattered-party cross stays
+/// within its envelope — the output-dominated case.
 ///
-/// The first leaf's magnitude seeds the frozen component and is read
-/// exactly once, in the closing shifted add against the whole interval.
+/// Every kept tooth boundary forces a fresh `2^k`-scale magnitude into
+/// the output, so the mandatory output dominates the linear input and the
+/// pinned ceilings price input + output bytes (the denomination the
+/// board's criterion records for exactly this cross).
 #[test]
-fn skyline_rank_bigroot_envelope() {
-    let p = Shape::Bigroot.packed2(BIGROOT_MAGNITUDE_BITS, BIGROOT_DEPTH);
+fn skyline_project_comb_scatter_envelope() {
+    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
     let v = version_of(&p);
+    let party = before::Party::decode(&Shape::ScatteredId.packed1(CLIFF_SCALE / 2).bytes[..])
+        .expect("scattered id is strict normal form");
     let enc = skyline_of(&p);
-    let r = query_metered(
-        "skyline_rank_bigroot",
-        enc.as_raw_slice().len(),
-        &query_env::SKYLINE_RANK_BIGROOT,
-        || meter::skyline::query::rank(meter::skyline::view(&enc)),
+    let io_bytes_in =
+        enc.as_raw_slice().len() + Shape::ScatteredId.packed1(CLIFF_SCALE / 2).bytes.len();
+    let out = metered(
+        "skyline_project_comb_scatter",
+        io_bytes_in,
+        &query_env::SKYLINE_PROJECT_COMB_SCATTER,
+        || meter::skyline::query::project(meter::skyline::view(&enc), &party),
     );
-    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
+    eprintln!(
+        "MEASURED skyline_project_comb_scatter: output_bytes={}",
+        out.as_raw_slice().len()
+    );
+    let expected = meter::skyline::encode(&(&v / &party).to_version());
+    assert_eq!(out, expected, "the kernel must match the packed quotient");
+}
+
+// ─── tick scenarios ─────────────────────────────────────────────────────────
+//
+// The public tick and multi-tick on the tick-designated families: the
+// fused walk, route fold, and splice in one pass. The tick walk's cost
+// currency is accumulator digit touches, with scanned bits beside it.
+
+/// Ticking the dense spine stays within its envelope: the fused tick's
+/// one walk and splice, with the output buffer deferred past the
+/// collapse scan.
+#[test]
+fn tick_dense_envelope() {
+    let p = Shape::Dense.packed1(DENSE_DEPTH);
+    let mut v = version_of(&p);
+    let seed = Party::seed();
+    metered("tick_dense", p.bytes.len(), &query_env::TICK_DENSE, || {
+        v.tick(&seed)
+    });
+    drop(v);
+}
+
+/// Ticking the wide right-full chain (a bigroot magnitude over the
+/// nested-full id) stays within its envelope: the anchor-web walk
+/// touches the wide first payload O(1) times, never once per shortcut
+/// level.
+#[test]
+fn tick_nested_wide_envelope() {
+    let ev = Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
+    let id = Shape::NestedFullId.packed1(TICK_CROSS_SCALE);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered(
+        "tick_nested_wide",
+        input,
+        &query_env::TICK_NESTED_WIDE,
+        || v.tick(&p),
+    );
+    drop(v);
+}
+
+/// Ticking the wide memo chain (a wide-tail spine under the
+/// nested-left-full id) stays within its envelope: the pre-scan's
+/// frame ledger stores no link for the shared wide minimum, so
+/// nothing is materialized per site.
+#[test]
+fn tick_mirror_wide_envelope() {
+    let ev = Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
+    let id = Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered(
+        "tick_mirror_wide",
+        input,
+        &query_env::TICK_MIRROR_WIDE,
+        || v.tick(&p),
+    );
+    drop(v);
+}
+
+/// Ticking the descending staircase under a party owning one deep
+/// diverted fragment (the ownership-hole family) stays within an
+/// envelope the leaf-by-leaf walk exceeds.
+///
+/// The fill walk's unowned regions are whole staircase runs, and the
+/// block scan must fold each into O(1) accumulator work instead of
+/// per-leaf freight. The touch ceiling is the skip's liveness signal
+/// — it sits below the per-leaf mechanism's reading, so the fast path
+/// must demonstrably engage; the scan column pins that every skipped
+/// bit is still read.
+#[test]
+fn tick_ownership_hole_envelope() {
+    let ev = Shape::Staircase.packed1(HOLE_STAIR_DEPTH);
+    let id = Shape::IdSpine.packed_flagged(HOLE_ID_DEPTH, true);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered(
+        "tick_ownership_hole",
+        input,
+        &query_env::TICK_OWNERSHIP_HOLE,
+        || v.tick(&p),
+    );
+    drop(v);
+}
+
+/// Ticking the alternating spine under the scattered id (the
+/// alternating-ownership comb: owned fragments and absent gaps
+/// interleaved at every level, so every unowned region is a single
+/// leaf) stays within its envelope.
+///
+/// The comb is the region gate's worst case — the block scan can
+/// never engage — and the pin holds the gated walk to the per-leaf
+/// walk's own readings: a gate that costs anything when closed moves
+/// this envelope.
+#[test]
+fn tick_ownership_comb_envelope() {
+    let ev = Shape::AltSpine.packed1(DENSE_DEPTH);
+    let id = Shape::ScatteredId.packed1(COMB_FRAGMENTS);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered(
+        "tick_ownership_comb",
+        input,
+        &query_env::TICK_OWNERSHIP_COMB,
+        || v.tick(&p),
+    );
+    drop(v);
+}
+
+/// Ticking the collapse-hole pair (deep descending collapse ranges under
+/// left-full sites with absent siblings) stays within an envelope the
+/// per-leaf consuming max scan exceeds.
+///
+/// Each unit's fully-owned range is crossed exactly once, by the walk's
+/// consuming max scan at its descend arm, and the crossing must ride the
+/// block summary: the touch ceiling sits below what per-leaf register
+/// freight over the same ranges reads, and the scan column holds every
+/// folded bit still read.
+#[test]
+fn tick_collapse_hole_envelope() {
+    let (ev, id) = Shape::CollapseHole.packed_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered(
+        "tick_collapse_hole",
+        input,
+        &query_env::TICK_COLLAPSE_HOLE,
+        || v.tick(&p),
+    );
+    drop(v);
+}
+
+/// Ticking the copy-hole pair (deep descending absent-child ranges inside
+/// one covering pre-scan) stays within an envelope the per-leaf sub-scan
+/// mechanism exceeds.
+///
+/// Each unit's untouched range is copied once by the pre-scan, and the
+/// copy must ride the block summary — one net movement and one watermark
+/// emission per range, never a virtual emission per leaf: the touch
+/// ceiling sits below the per-leaf mechanism's reading, and the scan
+/// column holds every folded bit still read.
+#[test]
+fn tick_copy_hole_envelope() {
+    let (ev, id) = Shape::CopyHole.packed_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered("tick_copy_hole", input, &query_env::TICK_COPY_HOLE, || {
+        v.tick(&p)
+    });
+    drop(v);
+}
+
+/// Ticking the site-hole pair (deep descending collapse ranges under
+/// interior left-full sites inside one covering pre-scan) stays within an
+/// envelope the extremum-streaming block fold exceeds.
+///
+/// Each unit's collapse range is crossed exactly twice — once by the
+/// pre-scan's collapse skip, once by the walk's consuming max scan at the
+/// site's own consume — and the pre-scan's crossing owes the web only the
+/// range's net height movement: the touch ceiling sits below what a block
+/// fold that also streams the range's unread minimum reads over the same
+/// ranges, and the scan column holds every folded bit still read.
+#[test]
+fn tick_site_hole_envelope() {
+    let (ev, id) = Shape::SiteHole.packed_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered("tick_site_hole", input, &query_env::TICK_SITE_HOLE, || {
+        v.tick(&p)
+    });
+    drop(v);
 }
 
-/// The rank kernel on the harmonic spine stays within its envelope — the
-/// rank fold's separating family.
+/// Ticking the raise-hole pair (deep descending raised ranges under
+/// right-full sites) stays within an envelope the per-leaf consuming max
+/// scan exceeds.
 ///
-/// The fold is linear here because each level's one-leaf delta lands in
-/// the accumulator at its own weight instead of re-shifting an
-/// accumulated numerator.
+/// Each unit's fully-owned right range is crossed exactly once, by the
+/// walk's consuming max scan at its ascend arm, and the crossing must
+/// ride the block summary: the touch ceiling sits below the per-leaf
+/// mechanism's reading, and the scan column holds every folded bit still
+/// read.
 #[test]
-fn skyline_rank_harmonic_envelope() {
-    let p = Shape::Harmonic.packed1(RANK_HARMONIC_DEPTH);
-    let v = version_of(&p);
-    let enc = skyline_of(&p);
-    let r = query_metered(
-        "skyline_rank_harmonic",
-        enc.as_raw_slice().len(),
-        &query_env::SKYLINE_RANK_HARMONIC,
-        || meter::skyline::query::rank(meter::skyline::view(&enc)),
+fn tick_raise_hole_envelope() {
+    let (ev, id) = Shape::RaiseHole.packed_pair(SCAN_HOLE_UNITS, SCAN_HOLE_STEPS);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered(
+        "tick_raise_hole",
+        input,
+        &query_env::TICK_RAISE_HOLE,
+        || v.tick(&p),
     );
-    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
+    drop(v);
 }
 
-/// The rank kernel on the boundary comb's skyline stays within its
-/// envelope.
-///
-/// The heights are `2^k`-scale behind 3-bit deltas, the live component
-/// absorbs the oscillation at O(1) digits per fold, and the terminal
-/// borrow — as wide as its own code — rides the live component into the
-/// last leaf's single wide add, no freeze anywhere.
+/// The fused multi-tick on the dense spine stays within its envelope:
+/// registering [`TICKS_POINT_LO`] events costs the single tick's walk and splice plus
+/// only the count's gamma-width boundary codes.
 #[test]
-fn skyline_rank_cliff_envelope() {
-    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
-    let v = version_of(&p);
-    let enc = skyline_of(&p);
-    let r = query_metered(
-        "skyline_rank_cliff",
-        enc.as_raw_slice().len(),
-        &query_env::SKYLINE_RANK_CLIFF,
-        || meter::skyline::query::rank(meter::skyline::view(&enc)),
+fn ticks_dense_envelope() {
+    let p = Shape::Dense.packed1(DENSE_DEPTH);
+    let mut v = version_of(&p);
+    let seed = Party::seed();
+    metered(
+        "ticks_dense",
+        p.bytes.len(),
+        &query_env::TICKS_DENSE,
+        || v.ticks(&seed, TICKS_POINT_LO),
     );
-    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
+    drop(v);
 }
 
-/// The rank kernel on the wide-tooth comb's skyline stays within its
-/// envelope — the no-freeze pin.
-///
-/// Bounded 192-bit oscillation keeps the live component exactly as wide
-/// as each tooth's own code, so every fold and every per-leaf add is paid
-/// by that code and the frozen component never churns (the
-/// `skyline_flatness` freeze-band row pins the same shape above the
-/// freeze allowance).
+/// The fused multi-tick on the wide right-full chain stays within its
+/// envelope: the `+n` splice compounds at the same site the single
+/// tick's does, and the wide first payload is still touched O(1) times.
 #[test]
-fn skyline_rank_wide_tooth_envelope() {
-    let p = Shape::WideToothComb.packed3(CLIFF_SCALE, WIDE_TOOTH_WIDTH_BITS, CLIFF_SCALE);
-    let v = version_of(&p);
-    let enc = skyline_of(&p);
-    let r = query_metered(
-        "skyline_rank_wide_tooth",
-        enc.as_raw_slice().len(),
-        &query_env::SKYLINE_RANK_WIDE_TOOTH,
-        || meter::skyline::query::rank(meter::skyline::view(&enc)),
+fn ticks_nested_wide_envelope() {
+    let ev = Shape::Bigroot.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
+    let id = Shape::NestedFullId.packed1(TICK_CROSS_SCALE);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered(
+        "ticks_nested_wide",
+        input,
+        &query_env::TICKS_NESTED_WIDE,
+        || v.ticks(&p, TICKS_POINT_LO),
     );
-    assert_eq!(r, v.rank(), "the kernel must match the packed rank");
+    drop(v);
 }
 
-/// The min_ticks kernel on the dense spine stays within its envelope:
-/// one narrow offset min-merge per node, heights on the accumulator,
-/// zero grown segments at 125k levels.
+/// The fused multi-tick on the wide memo chain stays within its
+/// envelope: the pre-scan's frame ledger behaves exactly as the single
+/// tick's, count notwithstanding.
 #[test]
-fn skyline_min_ticks_dense_envelope() {
-    let p = Shape::Dense.packed1(DENSE_DEPTH);
-    let v = version_of(&p);
-    let enc = skyline_of(&p);
-    let r = query_metered(
-        "skyline_min_ticks_dense",
-        enc.as_raw_slice().len(),
-        &query_env::SKYLINE_MIN_TICKS_DENSE,
-        || meter::skyline::query::min_ticks(meter::skyline::view(&enc)),
-    );
-    assert_eq!(
-        r.to_string(),
-        v.min_ticks().to_string(),
-        "the kernel must match the packed fold"
+fn ticks_mirror_wide_envelope() {
+    let ev = Shape::WideTail.packed2(TICK_CROSS_SCALE, TICK_CROSS_SCALE);
+    let id = Shape::NestedLeftFullId.packed1(TICK_CROSS_SCALE);
+    let mut v = version_of(&ev);
+    let p = party_of(&id);
+    let input = ev.bytes.len() + id.bytes.len();
+    metered(
+        "ticks_mirror_wide",
+        input,
+        &query_env::TICKS_MIRROR_WIDE,
+        || v.ticks(&p, TICKS_POINT_LO),
     );
+    drop(v);
 }
 
-/// The min_ticks kernel on the boundary comb stays within its envelope.
+// ─── grow-branch tick scenarios ─────────────────────────────────────────────
+//
+// The deep expansion shapes: pairs whose fill is the identity, so the
+// fused tick records the inflation route on its one walk and replays it
+// through the splice emit. These rows measure the whole public tick —
+// walk, route fold, and splice — on the shapes whose id side dominates.
+
+/// The version that is `1` on the leftmost `2^-depth` interval and `0`
+/// everywhere else: `depth` nested nodes, all bases zero, the single
+/// 1-leaf at the bottom left.
 ///
-/// The `2^k`-scale first height rides the frozen component and enters
-/// the exact total once, through the counting term — never per leaf —
-/// so the comb's teeth cost narrow offsets only.
-#[test]
-fn skyline_min_ticks_cliff_envelope() {
-    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
-    let v = version_of(&p);
-    let enc = skyline_of(&p);
-    let r = query_metered(
-        "skyline_min_ticks_cliff",
-        enc.as_raw_slice().len(),
-        &query_env::SKYLINE_MIN_TICKS_CLIFF,
-        || meter::skyline::query::min_ticks(meter::skyline::view(&enc)),
-    );
-    assert!(
-        r.to_string().len() > 20,
-        "the comb's floor exceeds any machine word: the wide arm is live"
-    );
-    assert_eq!(
-        r.to_string(),
-        v.min_ticks().to_string(),
-        "the kernel must match the packed fold"
-    );
+/// This is what ticking a version that is zero over the owned region
+/// registers for a depth-`depth` unary id spine. Built as a text
+/// literal (the parser is iterative), so the expected tree shares no
+/// walk with the tick under measurement.
+fn left_spike(depth: usize) -> Version {
+    let mut text = "(0, ".repeat(depth - 1);
+    text.push_str("(0, 1, 0)");
+    text.push_str(&", 0)".repeat(depth - 1));
+    text.parse().expect("the spike literal is normal form")
 }
 
-/// The min_ticks kernel on the ascending cliff stays within its
-/// envelope — the boundary-stacking case, and the committed basis for
-/// the anchor web's per-boundary word compaction.
+/// Ticking the empty version under a 250k-deep unary id spine stays
+/// within its envelope.
 ///
-/// The ascending spine arms every open range one above its parent's
-/// minimum, so the web holds `ASCEND_STACK_DEPTH − 1` nonzero unit
-/// boundary differences simultaneously at the terminal cliff — the one
-/// committed min_ticks shape where per-boundary transient storage is
-/// the envelope. The heap and touch ceilings are what the compacting
-/// instantiation buys: each word-scale difference is stored inline
-/// instead of as an accumulator entry, and the terminal cliff's
-/// undercut consumes each one by an O(1) word fold instead of an
-/// accumulator hop. With compaction deleted the same body reads over
-/// both the heap and touch ceilings \[demonstrated under the live
-/// swap, same harness\], so this row is the measured basis
-/// `MinWeb::compacting` cites.
+/// The walk is one event leaf whose route fold is the iterative id
+/// scan (bit-stack frames, nothing recurses), and the emit codes the
+/// whole expansion chain as fresh one-bit deltas. The value witness is
+/// closed-form: the expansion chain to the owned tip is exactly the
+/// left spike literal.
 #[test]
-fn skyline_min_ticks_ascend_envelope() {
-    let p = Shape::AscendCliff.packed2(ASCEND_STACK_DEPTH, ASCEND_STACK_MAGNITUDE_BITS);
-    let v = version_of(&p);
-    let enc = skyline_of(&p);
-    let r = query_metered(
-        "skyline_min_ticks_ascend",
-        enc.as_raw_slice().len(),
-        &query_env::SKYLINE_MIN_TICKS_ASCEND,
-        || meter::skyline::query::min_ticks(meter::skyline::view(&enc)),
-    );
-    // The family's closed form: k leaves at 2^b + i over spine minima
-    // all zero (the terminal cliff), so min_ticks = k·2^b + k(k+1)/2.
-    let k = ASCEND_STACK_DEPTH;
-    let expected = dashu_int::UBig::from(k as u64)
-        * (dashu_int::UBig::ONE << ASCEND_STACK_MAGNITUDE_BITS)
-        + dashu_int::UBig::from((k * (k + 1) / 2) as u64);
-    assert_eq!(
-        r.to_string(),
-        expected.to_string(),
-        "min_ticks disagrees with the ascending cliff's closed form"
+fn tick_expand_spine_envelope() {
+    let mut v = Version::new();
+    let party = party_of(&Shape::IdSpine.packed_flagged(ID_DEPTH, false));
+    // Byte sizes of buffers this test just allocated fit `usize`.
+    let input =
+        ((meter::skyline::encode(&v).len() / 8) + party.encoded_bits().div_ceil(8)) as usize;
+    metered(
+        "tick_expand_spine",
+        input,
+        &query_env::TICK_EXPAND_SPINE,
+        || v.tick(&party),
     );
     assert_eq!(
-        r.to_string(),
-        v.min_ticks().to_string(),
-        "the kernel must match the packed fold"
+        v,
+        left_spike(ID_DEPTH),
+        "the ticked version must be the derived closed form"
     );
 }
 
-/// The projection kernel on the comb × scattered-party cross stays
-/// within its envelope — the output-dominated case.
+/// Ticking the alternating spine under a deep unary id spine stays
+/// within its envelope.
 ///
-/// Every kept tooth boundary forces a fresh `2^k`-scale magnitude into
-/// the output, so the mandatory output dominates the linear input and the
-/// pinned ceilings price input + output bytes (the denomination the
-/// board's criterion records for exactly this cross).
+/// The regimes mix — the two-cursor fused walk down the shared spine,
+/// an id-only expansion fold where the id outruns the event — and the
+/// splice replays the recorded route. The value witness is closed-form:
+/// the unary id turns left into the spine's depth-2 zero leaf, so the
+/// forced route raises exactly the owned region from 0 to 1 — the
+/// pointwise max with the left spike, realized through the
+/// independently-tested join, byte-exact by canonical uniqueness.
 #[test]
-fn skyline_project_comb_scatter_envelope() {
-    let p = Shape::CliffComb.packed2(CLIFF_SCALE, CLIFF_SCALE);
-    let v = version_of(&p);
-    let party = before::Party::decode(&Shape::ScatteredId.packed1(CLIFF_SCALE / 2).bytes[..])
-        .expect("scattered id is strict normal form");
-    let enc = skyline_of(&p);
-    let io_bytes_in =
-        enc.as_raw_slice().len() + Shape::ScatteredId.packed1(CLIFF_SCALE / 2).bytes.len();
-    let out = query_metered(
-        "skyline_project_comb_scatter",
-        io_bytes_in,
-        &query_env::SKYLINE_PROJECT_COMB_SCATTER,
-        || meter::skyline::query::project(meter::skyline::view(&enc), &party),
+fn tick_expand_cross_envelope() {
+    let ev = Shape::AltSpine.packed1(DENSE_DEPTH);
+    let mut v = version_of(&ev);
+    let party = party_of(&Shape::IdSpine.packed_flagged(ID_DEPTH, false));
+    let expected = &v | &left_spike(ID_DEPTH);
+    // Byte sizes of buffers this test just allocated fit `usize`.
+    let input = ev.bytes.len() + party.encoded_bits().div_ceil(8) as usize;
+    metered(
+        "tick_expand_cross",
+        input,
+        &query_env::TICK_EXPAND_CROSS,
+        || v.tick(&party),
     );
-    eprintln!(
-        "MEASURED skyline_project_comb_scatter: output_bytes={}",
-        out.as_raw_slice().len()
+    assert_eq!(
+        v, expected,
+        "the ticked version must be the derived closed form"
     );
-    let expected = meter::skyline::encode(&(&v / &party).to_version());
-    assert_eq!(out, expected, "the kernel must match the packed quotient");
 }
 
 // ─── version-pair query scenarios ───────────────────────────────────────────
```

<!-- annotation -->
> **envelopes-a-2** (38), line 6289:
>
> `skyline_min_ticks_dense`'s "zero grown segments at 125k levels" restated as the depth alone.

<!-- annotation -->
> **envelopes-a-4** (4), line 6410:
>
> The tick and multi-tick scenarios and the grow-branch section now sit beside `query_env`, the table their rows live in. The ticks flatness bands stay in the dense-spine section: they are the suites lane's and use no table.

<!-- annotation -->
> **envelopes-a-4** (4), line 6416:
>
> Restated from the row's comment while moving it; "linear in nodes today" is gone (ruling 45).

<!-- annotation -->
> **fresh-eyes-taste** (105), line 6613:
>
> The moved tick doc names the constant instead of restating its value.

<a id="hunk-44"></a>
### crates/before/tests/meter.rs `@@ -7226,7 +6773,7 @@ fn version_distance_jump_pair_envelope() {`

```diff
@@ -7226,7 +6773,7 @@ fn version_distance_jump_pair_envelope() {
     let a = pa.version();
     let b = pb.version();
     let input_bytes = a.encode().len() + b.encode().len();
-    let (r, a, b) = query_metered(
+    let (r, a, b) = metered(
         "version_distance_jump_pair",
         input_bytes,
         &query_env::DISTANCE_JUMP_PAIR,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 6777:
>
> Call site renamed from `query_metered` to `metered`; no reading moved.

<a id="hunk-45"></a>
### crates/before/tests/meter.rs `@@ -7256,7 +6803,7 @@ fn version_lag_jump_pair_envelope() {`

```diff
@@ -7256,7 +6803,7 @@ fn version_lag_jump_pair_envelope() {
     let a = pa.version();
     let b = pb.version();
     let input_bytes = a.encode().len() + b.encode().len();
-    query_metered(
+    metered(
         "version_lag_jump_pair",
         input_bytes,
         &query_env::LAG_JUMP_PAIR,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 6807:
>
> Call site renamed from `query_metered` to `metered`; no reading moved.

<a id="hunk-46"></a>
### crates/before/tests/meter.rs `@@ -7281,7 +6828,7 @@ fn version_lag_jump_pair_envelope() {`

```diff
@@ -7281,7 +6828,7 @@ fn version_lag_jump_pair_envelope() {
 fn version_rank_concurrent_envelope() {
     let (v, _) = Shape::ConcurrentPair.version_pair(CONCURRENT_PAIR_LEAVES);
     let input_bytes = v.encode().len();
-    query_metered(
+    metered(
         "version_rank_concurrent",
         input_bytes,
         &query_env::RANK_CONCURRENT,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 6832:
>
> Call site renamed from `query_metered` to `metered`; no reading moved.

<a id="hunk-47"></a>
### crates/before/tests/meter.rs `@@ -7306,7 +6853,7 @@ fn version_rank_concurrent_envelope() {`

```diff
@@ -7306,7 +6853,7 @@ fn version_rank_concurrent_envelope() {
 fn version_distance_concurrent_envelope() {
     let (v, w) = Shape::ConcurrentPair.version_pair(CONCURRENT_PAIR_LEAVES);
     let input_bytes = v.encode().len() + w.encode().len();
-    let (r, _, _) = query_metered(
+    let (r, _, _) = metered(
         "version_distance_concurrent",
         input_bytes,
         &query_env::DISTANCE_CONCURRENT,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 6857:
>
> Call site renamed from `query_metered` to `metered`; no reading moved.

<a id="hunk-48"></a>
### crates/before/tests/meter.rs `@@ -7331,7 +6878,7 @@ fn version_distance_concurrent_envelope() {`

```diff
@@ -7331,7 +6878,7 @@ fn version_distance_concurrent_envelope() {
 fn version_lag_concurrent_envelope() {
     let (v, w) = Shape::ConcurrentPair.version_pair(CONCURRENT_PAIR_LEAVES);
     let input_bytes = v.encode().len() + w.encode().len();
-    query_metered(
+    metered(
         "version_lag_concurrent",
         input_bytes,
         &query_env::LAG_CONCURRENT,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 6882:
>
> Call site renamed from `query_metered` to `metered`; no reading moved.

<a id="hunk-49"></a>
### crates/before/tests/meter.rs `@@ -7370,7 +6917,7 @@ fn own_version_cmp_mask_drift_envelope() {`

```diff
@@ -7370,7 +6917,7 @@ fn own_version_cmp_mask_drift_envelope() {
     let p = Party::decode(&mask.bytes[..]).expect("the mask is strict normal form");
     let w = plateau.version();
     let input_bytes = v.encode().len() + mask.bytes.len() + w.encode().len();
-    let (ord, v, p, w) = query_metered(
+    let (ord, v, p, w) = metered(
         "own_version_cmp_mask_drift",
         input_bytes,
         &query_env::MASKED_CMP_DRIFT_TRIPLE,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 6921:
>
> Call site renamed from `query_metered` to `metered`; no reading moved.

<a id="hunk-50"></a>
### crates/before/tests/meter.rs `@@ -7408,7 +6955,7 @@ fn own_version_pair_cmp_mask_drift_envelope() {`

```diff
@@ -7408,7 +6955,7 @@ fn own_version_pair_cmp_mask_drift_envelope() {
     let p2 = Party::decode(&odd_mask.bytes[..]).expect("the mask is strict normal form");
     let input_bytes =
         v1.encode().len() + even_mask.bytes.len() + v2.encode().len() + odd_mask.bytes.len();
-    let (ord, v1, p1, v2, p2) = query_metered(
+    let (ord, v1, p1, v2, p2) = metered(
         "own_version_pair_cmp_mask_drift",
         input_bytes,
         &query_env::MASKED_CMP_DRIFT_QUAD,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 6959:
>
> Call site renamed from `query_metered` to `metered`; no reading moved.

<a id="hunk-51"></a>
### crates/before/tests/meter.rs `@@ -7448,7 +6995,7 @@ fn masked_cmp_hole_envelope() {`

```diff
@@ -7448,7 +6995,7 @@ fn masked_cmp_hole_envelope() {
     let p = Party::decode(&mask.bytes[..]).expect("the mask is strict normal form");
     let w = plateau.version();
     let input_bytes = v.encode().len() + mask.bytes.len() + w.encode().len();
-    let (ord, v, p, w) = query_metered(
+    let (ord, v, p, w) = metered(
         "masked_cmp_hole",
         input_bytes,
         &query_env::MASKED_CMP_HOLE,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 6999:
>
> Call site renamed from `query_metered` to `metered`; no reading moved, and the row's 480 B heap pin is untouched (ruling 24).

<a id="hunk-52"></a>
### crates/before/tests/meter.rs `@@ -7661,7 +7208,7 @@ fn fold_version_scatter_envelope() {`

```diff
@@ -7661,7 +7208,7 @@ fn fold_version_scatter_envelope() {
     let reference = versions.iter().fold(Version::new(), |acc, v| acc | v);
     let rest = versions.split_off(1);
     let receiver = versions.pop().expect("the population is nonempty");
-    let out = query_metered(
+    let out = metered(
         "fold_version_scatter",
         input_bytes,
         &query_env::FOLD_VERSION_SCATTER,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 7212:
>
> Call site renamed from `query_metered` to `metered`; no reading moved.

<a id="hunk-53"></a>
### crates/before/tests/meter.rs `@@ -7683,7 +7230,7 @@ fn fold_party_scatter_envelope() {`

```diff
@@ -7683,7 +7230,7 @@ fn fold_party_scatter_envelope() {
     let input_bytes: usize = parties.iter().map(|p| p.encode().len()).sum();
     let rest = parties.split_off(1);
     let mut acc = parties.remove(0);
-    let acc = query_metered(
+    let acc = metered(
         "fold_party_scatter",
         input_bytes,
         &query_env::FOLD_PARTY_SCATTER,
```

<!-- annotation -->
> **envelopes-a-4** (4), line 7234:
>
> Call site renamed from `query_metered` to `metered`; no reading moved.

<a id="hunk-54"></a>
### crates/before/tests/meter.rs `@@ -7727,7 +7274,6 @@ fn fold_party_scatter_envelope() {`

```diff
@@ -7727,7 +7274,6 @@ fn fold_party_scatter_envelope() {
 // with arity (the demonstration readings live in the pin commit); the
 // per-door `*_log_factor_is_alive` pins (the asymptotics suite) keep
 // the model's log factor itself honest.
-#[cfg(all(feature = "limb-meter", feature = "scan-meter"))]
 mod fold_stagger {
     use before::meter::registry::Shape;
     use before::{meter, Version};
```

<!-- annotation -->
> **fresh-eyes-D2** (108), line 7277:
>
> Deletion: the twelfth always-true `cfg(all(...))` gate, on this band module.

<a id="hunk-55"></a>
### justfile `@@ -97,13 +97,22 @@ default:`

```diff
@@ -97,13 +97,22 @@ default:
 
 # ── inner loop ───────────────────────────────────────────────────────────────
 
-# Type-check every host target: libs, tests, benches, examples.
+# Type-check every host target: libs, tests, benches, examples. A target
+# with `required-features` (the before envelope suite) is skipped here and
+# type-checked by `just test` and the gate's all-features legs instead.
 check:
     cargo check --workspace --all-targets
 
-# Run the test suites; pass a filter to narrow (`just test mirror`).
+# Run the test suites; pass a filter to narrow (`just test mirror`). The
+# envelope suite (crates/before/tests/meter.rs) builds only with its limb
+# and scan meters (`required-features` on its test target), so the inner
+# loop lights them for that package. One `--workspace` build unifies
+# features, so this compiles the before lib with both meters for every
+# dependent and runs every before suite gated on them (the envelope
+# suite, the fold and coincident-span suites, the limb-meter unit tests);
+# the other packages' own features stay default.
 test *args:
-    cargo nextest run --workspace {{ args }}
+    cargo nextest run --workspace --features before/limb-meter,before/scan-meter {{ args }}
 
 # Every feature is lit here and nowhere else in the gate: the meter suites
 # and the conformance module build only under `--all-features`.
```

<!-- annotation -->
> **meter-adequacy-11** (108), line 111:
>
> The inner loop passes the meter features for the `before` package (the `package/feature` form, since the recipe is `--workspace`), so `just test` runs the suite the manifest would otherwise skip; the comment says why.

<!-- annotation -->
> **fresh-eyes-W1** (108), line 106:
>
> The recipe comment states what the invocation builds: feature unification lights both meters on the before lib for every dependent, so every before suite gated on them joins `just test`.

<!-- annotation -->
> **fresh-eyes-W2** (108), line 98:
>
> `just check` skips a required-features target silently; the comment says where the meter suite is type-checked instead.

