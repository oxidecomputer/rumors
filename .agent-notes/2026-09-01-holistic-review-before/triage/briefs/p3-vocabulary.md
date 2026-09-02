<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P3 lane: vocabulary and register

## Goal

The crate's prose uses one vocabulary, and that vocabulary is the plain
one: no self-invented term stands for an entry point, a boundary, a
parameter, or a property; no moralized word stands for "measured" or
"real"; code comments use spaced double-hyphens, never em-dashes; values
are constructed, created, or built, never minted; `std` paths, edition
2024, and no dated benchmark record in the tree. The invariant is the
doctrine's: established terms of art only, and prose that reads as if
written today against today's code. Rulings 52 to 57 fix each sweep's
shape; this brief carries them with the census greps that hold the
result at zero.

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `10cdd255` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `10cdd255`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the class document in
  `.agent-notes/2026-09-01-holistic-review-before/`; the entry's full
  record (evidence, construction, witness outcome) is under `### <id>:`
  there (`#### <id>:` in `simplification.md`; nits in `documentation.md`,
  `simplification.md`, and `verification.md` are one-line table rows) and
  in `evidence/`. Line anchors are at the reviewed commit `9e5784fb`, and
  the tree outside `.agent-notes/` is byte-identical at your base, so they
  hold; re-anchor from the quoted evidence, never from a line number
  alone, once your own commits move the file.
- **The rulings govern.** `triage/rulings.md` records Finch's decisions.
  Where a quoted Resolution offers alternatives, the ruling named beside
  it picks one; where the ruling amends the Resolution, the amendment is
  stated under the quote and wins. Where a quoted Resolution and the lane
  goal come apart, the goal wins, and the discrepancy is reported.
- **Typed references, never strings (ruling 43).** Wherever this lane
- **Prose (ruling 105).** Every paragraph you touch passes the three tests in `PROSE.md` (altitude, concision, legibility); the reviewer applies its checks; the diff is net shorter in prose unless your report says what the additions buy.
  touches `meter/registry.rs`, a family roster, `TRIPWIRE_ROSTER`, the
  surface rosters, or any test that names another test, file, or line:
  reasons, pins, and enforcement homes are expressed as references the
  compiler resolves (function items, registered law names, `Shape` and
  `Op` values), never as strings naming a test function, a file, or a
  line number. A lane that sees a cleaner idiomatic shape for a roster is
  authorized to adopt it and reports the reshaping in its diff. Finch's
  words: "please make these instruments impossible to drift in the
  future. I *really don't like* the pattern of hard-coded strings and
  Rust source locations embedded in tests; the way these family rosters
  ended up is not really to my taste, but I haven't had time to make it
  more idiomatic and obviously correct. If you see a good way to clean it
  up, please do."
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin the brief does not
  name as moving; any change to a public signature or public rustdoc
  contract the resolution does not name (the meter surface under
  `any(test, feature = "meter")` is instrument surface by ruling 8 and
  not public API, but a change there lands with the `rumors` test update
  in the same commit); anything that contradicts a ruling; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop
  on one entry does not block the others.
- **Negative controls.** Every repaired or added instrument lands with a
  committed demonstration that a known-bad artifact fails it. The
  constructions in `evidence/witness.md` and each entry's Construction
  line are those artifacts; convert each into a committed test
  (`should_panic`, an asserted `Err`, a judge test over synthetic samples,
  or a reversible mutation whose observed failure the commit message
  records verbatim). Ruling 20 is the one place this brief set says
  otherwise, and it says so at the entry.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement; launch no daemons or services. One full
  `just gate` per agent, before the final commit, run in the background
  redirected to a log under `<scratchpad>/<lane>/` and polled with short
  foreground checks (the foreground command cap is ten minutes; a
  foreground gate run cannot finish). Keep every working file and evidence
  log under that directory, never loose at the scratchpad top level.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and the ruling. Every re-pin is measured
  at the parent and its movement named in the commit. Commit every
  proptest seed file that appears. Prose speaks in the present tense: no
  reference to code that no longer exists, no dated rationale at a
  declaration site. Comments use spaced double-hyphens, never em-dashes;
  every test has a doc comment stating its invariant. Commit with a
  descriptive message before finishing.
- **Never delete anything outside your worktree.** If the disk fills
  (ENOSPC), stop and report; it is the coordinator's problem, not a
  reason to trim caches you do not own.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why, and every deviation from a stated
  resolution, explicitly. Your report is data: the coordinator verifies
  each entry's Acceptance against the tree at the reported sha before the
  ledger records it. Report what you could not do rather than working
  around it.

## Ordering

This lane rewrites prose in nearly every file of the crate. It runs
after every P1 and P2 lane that rewrites the same files has landed, or
rebases onto them before its gate run; it never merges. Concretely: after
`p1-harness`, `p1-board`, `p1-suites`, `p1-fuzz`, `p2-rows`, and
`p2-surface` (each rewrites docs this sweep would also touch), and
after `p4-ghosts` and `p4-structure` if they are running (they edit the
same rustdoc). If the coordinator launches this lane earlier, it lands
the mechanical commits (ruling 54's sweep, ruling 57's `std` and
edition commits, the `Reign::new` rename) first, since a later rebase
of a purely mechanical sweep is cheap to redo, and holds the
vocabulary commits for the rebase.

Within the lane: ruling 54's `tools/` check first (or adopt the rumors
lane's if it has landed); then the em-dash sweep; then rulings 52, 53,
and 55 as three commits, each with its grep at zero in the commit
message; then ruling 57's three commits; ruling 56's one sentence
anywhere.

## Word list for ruling 53 (for Finch's review)

Replacement by sense, to be applied per site and reported where no
sense fits:

- "honest reading", "honest measurement", "honest count": measured
  reading, measured count.
- "honest implementation", "honest kernel", "honest fold": the shipped
  implementation, the unmodified kernel, the production fold.
- "honest baseline" (tools self-tests): baseline fixture.
- "genuine super-linearity", "genuine regression": a real
  super-linearity, a real regression (or the class named outright: a
  quadratic term).
- "genuine work", "genuine input": actual work, a real input.
- Keep as-is: `assert_honest_text` (an identifier), benchjudge's "honest"
  algorithm-class name and every string that must match it.

## Hazards and stops

- A rename that changes a public identifier other than `Reign::mint`
  (which is `pub(crate)`-side; confirm before renaming, and stop if it
  is public API) is a stop.
- A grep in an entry's Acceptance that cannot reach zero because a
  surviving technical sense needs the word (ruling 53's anchored names;
  ruling 55's watermark sense) is reported with the sites, not forced.
- The edition migration may surface new lints or rustfix changes; a
  behavior change under 2024 (for example `expr_2021` or `gen` keyword
  hits) is landed as its own commit and named; anything that moves a
  pin is a stop.

## Members

### codec-bits-5 (low, documentation): ruling 52

Door, seam, gate: three unanchored words for one boundary; "denomination" and "currency" each carry two senses

Resolution: Define "door" and "seam" once by contrast in `codec.rs`'s module doc (a door admits untrusted bytes or text into a stored value and owns their validation: decode, parse, literal; a seam is a hand-off between two in-crate representations at which an invariant is established: freeze, `built_view`, `into_base`); retire "gate" in the boundary sense (`just gate` already owns the word). Write "width" or "`u64`" for the integer-width sense of "denomination" and "unit" for stack.rs's "currency". Acceptance: `codec.rs` defines the two terms; "gate" as a boundary noun does not appear in the partition; "denominat" in `codec/` refers to a unit of measure or does not appear.

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### fresh-eyes-4 (low, documentation): ruling 52

'mint' and the 'door' metaphor in public rustdoc

Resolution: At party.rs:872-873 write "Creates identity exactly as the `u8` literal does: a constructor for tests and fresh universes ([Safety rules](crate#safety-rules))"; at party.rs:850 drop "Like every literal door,"; at lib.rs:49 write "never create themselves", then `just readme`. The private-prose uses of `door` are a separate question for the owner (below). Acceptance: `grep -rnw 'mint\|mints\|door' crates/before/src/party.rs crates/before/src/lib.rs` finds no line beginning `///` or `//!`.

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### oracle-laws-19 (low, documentation): ruling 52

"door" is an undefined coinage used 41 times in laws.rs

Resolution: Replace with the plain term at each site ("entry point", or the method or trait name: "the `Sum` and `FromIterator` impls", "the n-ary method"), or, if the word is kept, define it once in the crate-level docs and link that definition from laws.rs's module doc. The partition-local edit is mechanical; the crate-wide decision is Finch's (see open questions). Acceptance: `grep -c door crates/before/src/laws.rs` returns 0, or one definition site exists and laws.rs links it.

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### prose-hygiene-11 (low, documentation): ruling 52

Unanchored coinages: door (279), knob (107), seam (276), luck-proof (1)

Resolution: per term, either define once at first use in the owning module (registry.rs for knob and both senses of seam; codec.rs or the lib.rs implementation essay for door) or replace with the plain term where the count is small (luck-proof: name the touch list's property; seam outside the shape names: boundary). Acceptance: each surviving term has an italic definition site the module doc links, or the greps return only the shape-family identifiers.

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### span-causally-22 (low, documentation): ruling 52

"Structural genres" in `Span::decode`'s public `# Errors` is undefined at the user's altitude

Resolution: "On an input defective several ways at once, a component's own [`Truncated`](Decode::Truncated) or [`TrailingBits`](Decode::TrailingBits) is reported before the pair's [`NotCanonical`](Decode::NotCanonical), exactly as decoding the two components separately would." Acceptance: the public doc names the variants; "genre" appears in wire.rs only in private comments or not at all.

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### testing-diff-gen-24 (low, documentation): ruling 52

"door" is a crate-wide coinage never defined; "seam", "keystone", and "vehicle" point at things not named

Resolution: Define "door" once where the pins are organized (asymptotics.rs module doc: a *door* is one public method an operation is reachable through; the pins bind each separately because a door's wiring can drop a factor the shared core keeps) and cite that definition from registry.rs and laws/tests.rs, or replace with "public method"/"entry point". Name `replay_matches_across_references` where "keystone" is used; "the hole this pin closes is a drift" for the seam sentence; "the assertion macro" for "vehicle". Acceptance: every use of "door" resolves to one definition or is gone; "keystone" is followed by the test name on first use in each file.

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### version-core-17 (low, documentation): ruling 52

Register and vocabulary rules that postdate the prose: `mints`, moralized and dated adjectives, unanchored `door`, em-dashes in `//` comments

Resolution: (1) "returns the endpoints owned"; (2) "reachable only past the backend's capacity", "the production coordinate is 2⁶⁴ − 64 bits", "whose capacity is astronomically higher", "the backend-arm path", "the exact-size length stays exact", "half a GiB of groups"; (3) define `door` once (lib.rs or AGENTS.md: a public method or trait impl through which a value enters or leaves the crate) or replace per site with "public method" / "codec entry" / "fold"; (4) on `//` lines only, replace ` — ` with `: ` or `; ` where it introduces an apposition and with a parenthetical or a new sentence where it brackets one. Book all four as one crate-wide pass. Acceptance: `grep -n -i 'mint\|honest\|historical\|truthful'` over the partition returns nothing; a single definition site for `door` exists or the term is gone; `grep` for `—` on lines matching `^\s*//[^/!]` over the partition returns nothing.

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### crate-root-21 (nit, documentation): ruling 52

"door" is crate-wide jargon that no site defines

Where: `crates/before/src/fold.rs:89-89`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Define once where the entries live (codec.rs's or lib.rs's private docs): "A *door* is a public entry through which a value enters or leaves the proce ...

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### meter-core-1 (nit, documentation): ruling 52

Module-doc and comment prose: unanchored coinage, duplicated sentence, history in prose, "mint", em-dashes in `//` comments

Where: `crates/before/src/meter.rs:18-27`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Point line 26 at the registry's own phrasing ("the roster entries the compiler cannot force ...

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### rank-13 (nit, documentation): ruling 52

Register and vocabulary: "honest", "loud", "rent", "sliver", "door", "two-ways pin", "rank-class"

Where: `crates/before/src/version/rank.rs:549-552`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Replace each modifier with the mechanism it abbreviates: rank.rs:550 "and the gap clause holds for every u64 exponent on a 64-bit target" ...

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### skyline-coding-1 (nit, documentation): ruling 52

"currency" collides with the board's defined term; "arm" carries three senses; "door" is undefined

Where: `crates/before/src/version/skyline.rs:55-55`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): skyline.rs:55 and walk.rs:203: "the sign-magnitude type" / "as `Signed` values". shape.rs:110: "setting the pending rise" ...

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### span-causally-17 (nit, documentation): ruling 52

"door" is used as jargon for constructors and entry points without a definition

Where: `crates/before/src/span/tests.rs:149-151`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): in this partition, the plain noun ("constructor", "entry point", "`span_all`"). Crate-wide ...

Ruled (52): retire the term everywhere, public rustdoc and maintainer prose alike; replace each use with the plain noun (entry point, constructor, method, boundary, parameter, class) or the property the coinage named. No definition site survives; where the Resolution offers "define once", that alternative is struck. The census greps in the prose-hygiene entries hold zero at the end of the lane.

### board-families-floors-judge-15 (low, simplification): ruling 53

Vocabulary: "honest" as an undefined soundness criterion (29 sites, two rendered), "mint" (7), "backstop", "trued to", "door", "seams", "genre"

Resolution: Define the criterion once in the floors module doc ("a floor is sound when every conforming implementation reads at least it; a not-applicable declaration is sound when the contract forces no metered work") and use "sound"/"forced"/"mandatory" or the mechanism at the sites, the two rendered strings first; "mint" -> "build"/"derive"; "backstop" -> "the one leg that bounds"; "trued to" -> "matches"; "door" -> "constructor"; "seams" -> "the two kernels"; "genre" is established in-repo and may stay by ruling. Acceptance: no `honest` inside a rendered `WHY_`/`NA_` string; `grep -nwiE 'mint|mints|minted' family.rs` is empty; the owner's ruling on the crate-wide sweep is recorded.

Ruled (53): replace each moralized use of "honest" or "genuine" with the plain word the sentence means (measured, real, unmodified, baseline). The two anchored technical names, `assert_honest_text` and benchjudge's algorithm-class name, stay. Use the replacement list in this brief's "Word list" section; a site the list does not cover is reported, not improvised.

### prose-hygiene-10 (low, documentation): ruling 53

Moralized code: "honest" (234) and "genuine" (65) in place of the property that holds

Resolution: sweep both words: "honest floor" to "the floor" (its derivation is stated beside it), "honest improvement" to "an attributed improvement", "honest reading/tree" to "unmutated" or "within the model", "genuinely quadratic" to "quadratic"; rename the `honest` binding to `captures`. Acceptance: the two greps return nothing outside quoted test names.

Ruled (53): replace each moralized use of "honest" or "genuine" with the plain word the sentence means (measured, real, unmodified, baseline). The two anchored technical names, `assert_honest_text` and benchjudge's algorithm-class name, stay. Use the replacement list in this brief's "Word list" section; a site the list does not cover is reported, not improvised.

### board-frame-18 (nit, documentation): ruling 53

Vocabulary tells across the frame's prose: "mint", an unanchored "honest" in four senses, register transplants, "genre", "today", a past-tense justification, a duplicated sentence

Where: `crates/before/src/meter/board/coverage.rs:435`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Schedule a crate-wide prose pass. In it: "construct"/"build" for mint; keep "output honesty" anchored to `assert_honest_text` and replace the other se ...

Ruled (53): replace each moralized use of "honest" or "genuine" with the plain word the sentence means (measured, real, unmodified, baseline). The two anchored technical names, `assert_honest_text` and benchjudge's algorithm-class name, stay. Use the replacement list in this brief's "Word list" section; a site the list does not cover is reported, not improvised.

### skyline-query-12 (nit, documentation): ruling 53

Moralized and significance wording: "honest", "real", "is the point"

Where: `crates/before/src/version/skyline/query/integral.rs:356-359`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Name the property at each site; drop "is the point" and state the claim; restate integral.rs:272 as the work bound

Ruled (53): replace each moralized use of "honest" or "genuine" with the plain word the sentence means (measured, real, unmodified, baseline). The two anchored technical names, `assert_honest_text` and benchjudge's algorithm-class name, stay. Use the replacement list in this brief's "Word list" section; a site the list does not cover is reported, not improvised.

### tools-8 (nit, documentation): ruling 53

"honest" as an algorithm class in benchjudge and as the baseline-fixture label in every self-test; "mint" for constructing a value at four sites (benchjudge:447, 859; memwatch:70, 94)

Where: `tools/benchjudge:126-127`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "the divide-and-conquer class", "the baseline fixture", "a roster cannot declare a class", "forge a synthetic record"; `grep -n '\bmint' tools/*` empty and `grep -ci honest tools/benchjudge` 0

Ruled (53): replace each moralized use of "honest" or "genuine" with the plain word the sentence means (measured, real, unmodified, baseline). The two anchored technical names, `assert_honest_text` and benchjudge's algorithm-class name, stay. Use the replacement list in this brief's "Word list" section; a site the list does not cover is reported, not improvised.

### prose-hygiene-12 (low, simplification): ruling 54

Em-dashes in // and # comments and in message string literals

Resolution: mechanical sweep: in `//` and `#` comments replace ` — ` with `: `, `; ` or ` -- ` by sentence sense; in string literals that reach a terminal replace with colons. A `tools/` linter leg rejecting U+2014 outside `///`, `//!` and Markdown would keep it closed. Acceptance: the partition reports zero plain-comment, code-line and non-Markdown hits.

Ruled (54, with ruling 7): the workspace U+2014 check in `tools/` lands first and is wired into `just gate`, reads red on the existing lines, and one mechanical sweep commit takes it to zero across `crates/before` and `tools/`. Rustdoc (`///`, `//!`) and Markdown are outside the check; only `//` comments, Python docstrings, and printed diagnostics are swept. If the rumors triage's lane has already landed the check at your base, run against it and add nothing.

### tools-29 (nit, documentation): ruling 54

Em-dashes in `#` comments, docstrings, and nine printed diagnostics across the tools (owner-gated: the pending em-dash ruling)

Where: `tools/mutantcheck:195-197`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Colons or semicolons throughout tools/; the self-test needles are substring-safe except citecheck:844 and workflowlint:545

Ruled (54, with ruling 7): the workspace U+2014 check in `tools/` lands first and is wired into `just gate`, reads red on the existing lines, and one mechanical sweep commit takes it to zero across `crates/before` and `tools/`. Rustdoc (`///`, `//!`) and Markdown are outside the check; only `//` comments, Python docstrings, and printed diagnostics are swept. If the rumors triage's lane has already landed the check at your base, run against it and add nothing.

### prose-hygiene-5 (medium, documentation): ruling 55

"mint" for constructing a value at 82 sites, including the identifier Reign::mint

Resolution: rename `Reign::mint` to `Reign::new` (private; no API movement) and replace every prose, comment and message use with construct, create, build, or the specific operation (fork, split, emit, allocate); rerun `just readme` so README.md:53 follows lib.rs:49. Acceptance: the grep above returns nothing across the in-scope surfaces.

Ruled (55): rename `Reign::mint` to `Reign::new` and sweep the prose to construct, create, build, or the specific operation. `watermark.rs`'s latent-register sense of "mint" is not swept: report the sites and the sense so Finch can rule on it separately.

### skyline-fill-grow-21 (low, documentation): ruling 55

"mint"/"minted" at eight sites, in three senses

Resolution: memo.rs:4 "defined in"; prescan.rs:380 "nothing is allocated per resolve"; tests.rs:1268 "producing content"; tests.rs:1298 "the orbit expands each id site at most once". For the watermark sense (fill.rs:817; tests.rs:753, 894, 957), either link the term to its definition at first use or rename it with watermark.rs (owner's call, outside this partition). Acceptance: `grep -in mint` over the partition returns only sites that link to the watermark definition, or nothing.

Ruled (55): rename `Reign::mint` to `Reign::new` and sweep the prose to construct, create, build, or the specific operation. `watermark.rs`'s latent-register sense of "mint" is not swept: report the sites and the sense so Finch can rule on it separately.

### crate-root-38 (medium, documentation): ruling 56

A plateau is documented as a "maximal constant run" but the walk yields canonical leaves, which can be adjacent and equal

Resolution: Define the item by the coding: "A *plateau* is one leaf of the version's canonical coding: a dyadic interval on which the step function is constant. Canonical form merges equal sibling leaves, so adjacent plateaus differ in height except across a subtree boundary, where `rise: None` marks a level step." Apply the same correction to the `Region` sentences (14-15, 115-116), the module bullets (11-12), and, outside this partition, skyline.rs:4-5, overlay.rs:99, and party.rs:473. Keep "equal iff plateau sequences equal" (it holds for the leaf sequence). Acceptance: no sentence in shape.rs calls an item a maximal constant run; a doctest parses `(0, (0, 0, 1), (0, 1, 0))` and asserts four plateaus with the third's `rise == None`. Construction: `let v: Version = "(0, (0, 0, 1), (0, 1, 0))".parse().unwrap(); let p: Vec<_> = v.shape().collect();` The tree is canonical (no equal sibling leaves; no liftable minimum), the step function is 0, 1, 1, 0 over quarters (three maximal runs), and the walk yields four `Plateau { depth: 2 }` items, the third with `rise: None`. For parties, `((0, 1), (1, 0))` yields four regions with the middle two both `owned: true`.

Ruled (56): model. Finch's words: "I think this overloading is fine. Two adjacent same-height plateaux are the same as one." One sentence on `shape::Plateau` records it; the three other sites are not reworded.

### benches-examples-25 (medium, claims): ruling 57

results/benchmarks is a 2026-06-02 record of benches that no longer exist, with a wrong mechanism for its largest quoted win and a broken table row

Resolution: excise crates/before/results/benchmarks and scripts/plot_benchmarks.py, or regenerate under the live suite (drop the party partial_cmp panels, add `version/hole`, escape the pipe in the merge title, re-run `cargo bench -p before` on a quiet machine, date the README by commit rather than calendar). Drop the mechanism sentence at README:43-46 either way (see benches-examples-17). Acceptance: every group/function the plot script names exists in benches/*.rs; the speedup table matches the live bench roster; the README's mechanism claims match sweep.rs; no table row splits on a pipe. Construction: `cargo bench -p before --bench party -- --sample-size 10 --measurement-time 1` then `python3 crates/before/scripts/plot_benchmarks.py`: the party partial_cmp panels render empty (`series()` returns None for absent groups) and the version join speedup inverts relative to the committed table.

Ruled (57): delete `results/benchmarks`; do not regenerate.

### oracle-laws-7 (nit, simplification): ruling 57

Qualified `crate::` paths where an import would do, `std`/`core` mixing, the legacy `DefaultHasher` path, and a mid-clause comment wrap

Where: `crates/before/src/oracle/version.rs:211-212`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Imports; one `iter::once` spelling; `std::hash::DefaultHasher`

Ruled (57): `std` over `core` crate-wide; the five edition-2021 member crates migrate to 2024 in one commit with `manifestlint` holding the edition; `results/benchmarks` is deleted (git keeps it), nothing regenerated.

### span-causally-10 (nit, simplification): ruling 57

Inline qualified paths where imports exist, and a `core::` beside `std::` imports

Where: `crates/before/src/span/algebra.rs:356-369`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Import `once` and `balanced_reduce`

Ruled (57): `std` over `core` crate-wide; the five edition-2021 member crates migrate to 2024 in one commit with `manifestlint` holding the edition; `results/benchmarks` is deleted (git keeps it), nothing regenerated.

### suanpan-1 (nit, simplification): ruling 57

Edition 2021 under a 2024 workspace root

Where: `crates/suanpan/Cargo.toml:4-4`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Migrate the editions together, or state the reason at the root

Ruled (57): migrate all five edition-2021 crates in one commit; `manifestlint` holds the workspace edition thereafter (a committed fixture with a 2021 member fails it).

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### board-frame-11 (low, documentation): roster: approved (ruling 104)

Escaped-bracket citation tags (`\[derived\]`, `\[the ... in the test suite\]`) are an undefined convention that names no test

Resolution: Delete the `\[derived\]` tags; replace each test citation with the test's function name in backticks (the chunked-schoolbook, schoolbook, delegating-parser, and sub-scaling tests in board/tests.rs). Acceptance: `grep -rn -F '\[' crates/before/src/meter/board/` returns nothing; every test cited in ceilings.rs and cell.rs prose is a function `grep -n 'fn <name>'` finds.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-26 (low, documentation): roster: approved (ruling 104)

"mint" for constructing a value, in the Quickstart and crate-wide

Resolution: lib.rs:49 "// New participants fork off a live clock; nothing creates a second seed."; serde_impls/tests.rs:111 "every rejection genre the raw decodes produce"; sweep the remaining sites (party.rs:15 "which create a second holder", :872 "Creates identity exactly as ..."; rename `Reign::mint` if the owner wants the rule to reach identifiers). Acceptance: `grep -rni '\bmint' crates/before/src` is empty, or lists only identifiers the owner exempts.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### deps-11 (low, documentation): roster: approved (ruling 104)

the fuzz workspace manifest and README carry opaque roster IDs (PROG-5 / COV-7) and restate build commands the justfile supersedes

Resolution: drop the IDs and name the property ("the decode round-trip invariant: an accepted value re-encodes stably and decodes back to itself"); replace the command listings in Cargo.toml:3-9 and README:50-64 with a pointer to the two recipes; let the README's prerequisites name the pinned nightly via the justfile rather than `rustup toolchain install nightly`; drop the justfile:51-52 cross-reference once the manifest no longer carries the number. Acceptance: grep of `(PROG|COV)-[0-9]+` over the tree is empty; the fuzz workspace's prose names no cargo-fuzz command line.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### envelopes-a-18 (low, documentation): roster: approved (ruling 104)

Unanchored coinages and significance refrains at maintainer altitude

Resolution: "genre" outside 37-46 to "kind" or "family"; "freight" to "per-leaf register work"; "daylight" to "digit clearance" (matching `SEAM_CLEARANCE`); "funded width" to "the width its own code paid for" or "priced by" per overlay.rs; "honest improvement" to "an improvement", "honest stand-in" to "the stand-in"; delete the "never decoration" and "Semantics first:" sentences or fold their fact into the preceding clause. Acceptance: `grep -c 'honest\|freight\|daylight\|never decoration\|Semantics first\|truing'` over lines 1-5305 is 0; "genre" appears only in the file doc's contrast definition; "funded" is gone or defined once beside "priced by".

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### envelopes-b-3 (low, documentation): roster: approved (ruling 104)

Coined labels and register transplants: `GREEN PIN`, `mandate`, `mint`, two senses of `genre`, moralized and economic vocabulary

Resolution: delete the `GREEN PIN:` prefixes (each sentence already states the invariant); replace `genre` outside the file doc's floor/tripwire contrast with `class` or define the second sense once; `mandate` to `lower bound`; `consume-minted` to `consume-time`, `mints` to `produces`; drop "not a vibe", "is the point", "kills"; "honest improvement" to "improvement" where the dead-meter contrast is already stated. Acceptance: `grep -c 'GREEN PIN' crates/before/tests/meter.rs` reads 0; `grep -n 'mint' crates/before/tests/meter.rs` is empty; `genre` appears only in its defined sense.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-pipeline-2 (low, simplification): roster: approved (ruling 104)

Vocabulary and register sweep: "mint" for constructing values, "honest" and "real" for properties, em-dashes in line comments

Resolution: "derived" (lib.rs:19); "split in the guest" / "guest-split" (plan.rs:19, 168; ops.rs:105, 800); "builds" (ops.rs:185); "O(1) hole construction" (ops.rs:2448); "produce" (ops.rs:2507); "no size axis" (ops.rs:2168); "rejects a uniform draw" (sample/tests.rs:99); "the shipping decoder(s)" for "real" and rename `real` to `decoded` in sample/tests.rs:73, 88; swap the five line-comment em-dashes for ` -- ` or a colon. Acceptance: `grep -n -i -w 'mint\|minted\|mints\|honest' crates/before-fuelscape/src` returns nothing; `grep -n -E '^\s*//[^/!].*—'` over the partition returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-render-1 (low, documentation): roster: approved (ruling 104)

Vocabulary: "guest-minted", "two-ways seam", "Honesty rule"/"honest", "backstop"

Resolution: "guest-minted" to "produced in the guest" (and the other mint forms in plan.rs, ops.rs, lib.rs to construct/produce/derive); "the two-ways seam" to a sentence that defines the check ("the grid is recomputed from the samples and must equal the stored one"), and "two-ways pin" likewise where it appears; "Honesty rule" to "Presentation rule", "honest linear density" to "linear density", "most honest clicks" to "most ordinary clicks"; "blur is the backstop" to "blur ends any drag the browser never releases". Acceptance: no "mint" in before-fuelscape prose; "two-ways" appears only in a sentence that defines it; fuelscape.js contains neither "honest" nor "backstop".

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzz-guests-pins-2 (low, documentation): roster: approved (ruling 104)

Opaque roster IDs and a ghost test name in the fuzz manifest and README

Resolution: Delete the three parenthetical tags (the surrounding sentences already name the invariant in plain words) and cite `clock::tests::decode_never_panics`. Acceptance: `grep -rnE 'PROG-[0-9]|COV-[0-9]|h34_' crates/before` is empty and the cited test name resolves to a `fn`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzz-guests-pins-21 (low, documentation): roster: approved (ruling 104)

"mint" for constructing values at five sites

Resolution: 454-456: "The text door constructs the clock's party from the literal; ... so no such party ever meets a live handle." 881, 899, 1592, 1657: "the operands are read in place and the endpoints are freshly allocated (owned)". Acceptance: `grep -in '\bmint' crates/before/fuzzfit/guest/src/lib.rs` returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-strategies-15 (low, documentation): roster: approved (ruling 104)

Vocabulary: a moralized bound, unanchored coinages, and four words carrying two meanings

Resolution: 34-37: "and bounds a composed case's total denominated work to `max_ops` times a constant fixed by the tick, fork, and fold caps"; "decoration-wide" to "too wide to catch a regression"; 1880: "a clock assembled from two universes"; 87: "the enforcement suite's case count"; 48: "this instrument's scope is the region..."; keep "rung" in ops.rs (before's own term) and say "snapshot" or "step" for the ladder in strategies.rs; rename the escalation arm's "cadence battery" or fold it into a named function; cite `FamilyId` variants by identifier at 172-174 or per `Family` variant. Acceptance: each listed term names an identifier, is defined once by contrast where introduced, or is replaced by its mechanism; no word has two referents in the fuzzfit harness. (The same "honest" qualifier recurs in sanity.rs:54, enforce.rs:201, and bands.rs:52, 116-119, 170, 187-190, outside this partition; noted for those reviewers.)

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### gate-legs-11 (low, documentation): roster: approved (ruling 104)

The fuzz workspace's prose carries opaque roster tags, floating-nightly instructions, a ghost test name, and a hand-duplicated duration

Resolution: Delete the tags; fix the test name to `clock::tests::decode_never_panics`; replace the manual command lists with `just fuzz-build` / `just fuzz` (keeping the seed-corpus explanation); drop the duration duplication from the justfile comment or make the manifest defer to the recipe. Acceptance: no `PROG-`/`COV-` tag remains in the in-scope tree, every test name in the fuzz README resolves against `cargo nextest list -p before --all-features`, and the fuzz prose names the dated toolchain or the recipe rather than `+nightly`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### meter-registry-tier2-1 (low, documentation): roster: approved (ruling 104)

Vocabulary tells in the registry prose: "mint" for construction, "earns", "sentinel", "honest", "luck", "mandate", "tombstone"

Resolution: "build"/"built" for mint; "has a column"/"gets no column" for earns; "probe" for sentinel; "minimal-work witness" and "never a requirement" at 883/887; name the mechanism at 920 ("the shape that retires every live-anchored follower"); "otherwise unchecked" at 572 and "the adjacent-slot coalescing that index order would allow" at 636; a straight apostrophe at tests.rs:163. Acceptance: `grep -nwiE 'mint|minted|earns?|sentinel|honest|luck|mandate|tombstone' crates/before/src/meter/registry.rs` returns nothing, and `grep -n "’" crates/before/src/meter/tier2/tests.rs` is empty.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### module-graph-6 (low, documentation): roster: approved (ruling 104)

The fuzz workspace's manifest and README carry opaque roster IDs and a stale gate claim

Resolution: Rewrite both headers in the surfacecheck manifest's form: detached so workspace-wide cargo invocations never compile it; the gate reaches it through `just fuzz-build` (and `just fuzz` at `all` cadence); drop the ID tags and keep the invariant in words (line 48 already spells it out). Acceptance: no roster ID remains in the tree; the header names the recipes that reach it.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### oracle-laws-22 (low, documentation): roster: approved (ruling 104)

Prose tells: the banned "minted", "THE LAW", moralized "real"/"honest", significance adverbs, the undefined "under mass" and "faces", and temporal "still works"/"survive here"

Resolution: "no event minted" -> "without marking an event" (production's phrasing at clock.rs:494) at 2956, 3236, 3473; "THE LAW of the rank wire form" -> "The rank wire form's defining law"; "is real *within*" -> "is strict within"; "the definitionally honest loop" -> "the literal loop"; drop "genuinely" at laws.rs:1036, 2354, laws/tests.rs:74, oracle/tests.rs:544; "under mass" -> "on most samples" or define it once in testing/generators.rs where it originates; "faces" -> "the party and clock instances"; "still works" -> "works"; "survive here for" -> "exist for"; "not real public API" (oracle.rs:40) -> "a test and bench reference, not supported API"; rewrite laws/tests.rs:67-76 as mechanism ("This feed order makes the closing drain hand back a coalesced group; the conservation laws hold on it, and a fold that dropped that group would fail them while passing the acceptance laws' `Err` clauses"). Acceptance: `grep -n -i 'minted\|THE LAW\|honest\|genuinely\|under mass\|survive here\|still works\|red the day\|police' crates/before/src/laws.rs crates/before/src/laws/tests.rs crates/before/src/oracle crates/before/src/oracle.rs` returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### party-5 (low, documentation): roster: approved (ruling 104)

Register and vocabulary sweep items: `mint`, em-dashes in `//` comments, `honest`, `tripwire`, a `d_` prefix, a dependent first sentence

Resolution: party.rs:15 "create a second holder", party.rs:872 "Creates identity exactly as", sum_split.rs:166 "the inline pair reads better than a coined name" (party-6 removes the allow anyway), tests.rs:1042 "add more than its one tree level"; ` -- ` at the 24 `//` sites; "the original" / "the defect-free transcription" / "the sublinear arm" for `honest`; "The deterministic companion to" (430) and "witnesses and floors" (584) for `tripwire`; rename `d_fork_join_roundtrip` to `fork_join_roundtrip_matches_oracle`; tests.rs:993 "`as_bytes` equals `encode` for both halves produced by `fork`". Acceptance: `grep -rni '\bmint' crates/before/src/party.rs crates/before/src/party/ crates/before/src/idbits.rs` empty; `grep -n '^\s*//[^/!].*—'` over the partition empty; every `tripwire` in tests.rs names a committed known-bad artifact.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### prose-hygiene-6 (low, documentation): roster: approved (ruling 104)

Prose still describes owner rationales and re-pin annotations as dated after the dated-notes excision

Resolution: in board.rs:171 and ceilings.rs:234 write "with the owner ratification stated at the declaring constant"; in calibrate.rs:373-374 and 384-385 (the source of bands.rs:325-326 and 924-925) and in bands.rs:7-8 write "commit with the movement and its attribution in the commit message", then run `just fuzzfit-calibrate` so the generated copies follow. Acceptance: `grep -rn -w dated crates/before/src/meter/board crates/before/fuzzfit` returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### prose-hygiene-8 (low, documentation): roster: approved (ruling 104)

Historical narration at declaration sites ("retired", "the old", "before this binding existed")

Resolution: name each known-bad kernel by mechanism ("the per-digit schoolbook charge", "the frozen-width-per-tooth kernel", "quadratic re-walks") and describe each current check without its predecessor; delete "the table was twice found wrong in review before this binding existed" (the bullet already states the binding's purpose). Acceptance: `grep -rn -E '\bretired\b|\bthe old\b|\bit replaces\b|before this binding'` over the listed files returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### prose-hygiene-9 (low, documentation): roster: approved (ruling 104)

"today"/"currently"/"none at this tip" as relative-time naming of the present implementation

Resolution: delete "today" in the six floors.rs strings, currency.rs:139, strategies.rs:117, compact.rs:42 and AGENTS.md:34; in the three empty-roster docs keep only the sentence stating what an entry means and let the empty literal speak. Acceptance: `grep -rn -i -E '\btoday\b|currently empty|none at this tip'` over the listed files returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-query-16 (low, documentation): roster: approved (ruling 104)

`Arming` (a ledger entry) collides with the watermark web's "arming" of a range, with no contrast drawn

Resolution: Either one sentence of contrast at `Arming`'s definition ("an arming here is a ledger entry; the watermark web's arming of a pending range is unrelated") and the mirror sentence in web.rs's module doc, or rename the struct to `Promotion` (its own doc's noun; the field is already `promotions: Vec<Arming>`), leaving the registry family names untouched. Acceptance: each module's first use of "arming" is unambiguous to a reader of that module alone.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-query-23 (low, simplification): roster: approved (ruling 104)

`Reign::mint` and "mint" prose for constructing a value

Resolution: Rename to `Reign::new` (or `Reign::at_leaf`); reword query.rs:56 "defines and derives", web.rs:64 "a reign record's creation", 66 "between creation and death", 176 "since the record was created", 306 "created or moved". Acceptance: the grep over the partition returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### span-causally-27 (low, documentation): roster: approved (ruling 104)

Banned vocabulary: "mint"/"minting" for constructing a hole

Resolution: line 30: "(constructors add at most one hole; ...)"; line 37: "rather than introducing a corner case here." Acceptance: `grep -rni '\bmint' crates/before/src/causally crates/before/src/span` returns nothing (party.rs is outside this partition).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### surface-roster-15 (low, documentation): roster: approved (ruling 104)

`d1_seeds_stay_committed` carries an opaque prefix defined nowhere

Resolution: rename to `fold_seeds_stay_committed` and update surface_coverage.rs:77 and 115. Acceptance: `grep -rn 'd1_' crates/before/src` returns nothing; the coverage tests pass.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-diff-gen-5 (low, documentation): roster: approved (ruling 104)

Prose sweep: em-dashes in line comments and one assert message, "mint", "honest", temporal and anticipatory phrasing

Resolution: Mechanical sweep: colons or semicolons for the listed em-dashes (`///` and `//!` doc comments keep theirs); "coin" or "add" for "mint"; "states" for "states honestly" and "the measured constant" for "the honest constant"; "a cure that removes the merge flips this pin" for the anticipatory sentence; drop "Currently"; drop the "three independent spellings" count. Acceptance: `grep -nE '^\s*//[^/!].*—'` over the partition returns nothing; no `mint`, `honest`, or `Currently` in the partition.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-oracles-29 (low, documentation): roster: approved (ruling 104)

Validation index prose: a ghost reference to replaced bodies, "mint", "honest", a self-description hedge, and a hand count

Resolution: 42-45: state the property positively ("A body that chose its own population would make coverage a product nobody enumerated; here the population belongs to the driver and the operation to the descriptor, so the two meet by construction."). 5-6: "This module is documentation only: it holds no code." 75: "guarded by a deliberate layering of instruments". 142: "never set a threshold". 178: "a legitimate input did less work than the floor's premise". 182: "whichever direction the code supports". Acceptance: none of "replaced", "mint", "honest", "in the spirit of", or "four instruments" remains in the file.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### tests-other-12 (low, documentation): roster: approved (ruling 104)

"pincer" and "jaw" are an unanchored metaphor used as jargon across four roster pins

Resolution: Replace each use with the mechanism ("invisible to both totality checks: the rustdoc-JSON census omits hidden items and the roster scan reads only named files"), or define the term once where the two checks are described and cite that site. Acceptance: `grep -rn 'pincer\|\bjaw' crates/before` returns nothing, or every hit follows one definition site.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### tests-other-23 (low, simplification): roster: approved (ruling 104)

"mint" for constructing a value, including two test names

Resolution: Rename to `same_party_ticks_on_divergent_clones_produce_equal_versions` and `from_parts_over_an_earlier_version_reproduces_its_successor`; "produces"/"yields"/"re-derives" at lines 6, 22, 79, 93; "build its operands" at amp_board_smoke.rs:354. Acceptance: `grep -rn -i mint crates/before/tests` is empty.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### api-audit-19 (nit, documentation): roster: approved (ruling 104)

Vocabulary tells in public rustdoc: "mint" for constructing values, "honest" for exact

Where: `crates/before/src/lib.rs:49-49`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "create"/"build" for mint; "exact"/"the price of exactness" for honest, at the public sites first

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-ops-render-5 (nit, documentation): roster: approved (ruling 104)

Em-dashes in plain `//` comments at twenty sites

Where: `crates/before/src/meter/board/ops.rs:327-327`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Replace with colons, semicolons, or spaced `--` in those twenty lines

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-ops-render-8 (nit, documentation): roster: approved (ruling 104)

Moralized and overloaded vocabulary: unanchored "honest", two meanings of "diagonal", three referents for "seam", and "mints"

Where: `crates/before/src/meter/board/ops.rs:701-702`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Replace unanchored "honest" with the mechanism ("the input-byte denominator, which the output-honesty assertion makes the smaller of the two" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### clock-24 (nit, documentation): roster: approved (ruling 104)

Unanchored coinages and significance adverbs in maintainer prose

Where: `crates/before/src/clock/tests.rs:612`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "keystone invariant" → "the invariant `Eq`/`Hash` rest on"; "text mirror" → "the text round-trip"; drop "genuinely", "really", and the " ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### clock-27 (nit, documentation): roster: approved (ruling 104)

Banned "mints" in an orbit test doc

Where: `crates/before/src/clock/tests.rs:1257-1259`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "iterated re-partitioning of an idle region adds no bytes, with no transient and no ratchet"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-base-text-tree-25 (nit, documentation): roster: approved (ruling 104)

Register words: "honest", "genre", "keystone", "real", "major finding", a caps `WITNESS` label, and a "Test-only" mislabel

Where: `crates/before/src/codec/tests.rs:1225-1238`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "so the recorded cost counts exactly the window pairs the scan compared"; "class" or "kind" for "genre" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-bits-3 (nit, documentation): roster: approved (ruling 104)

Register tells: "honest", "genuinely", and "real" where the anchored term is "live"

Where: `crates/before/src/codec/bits.rs:50-50`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "Two instruments pin the ladder"; "a consumer with wide arithmetic"; "live stream" / "live input" / "allocated memory" at the "real" sites

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-bits-4 (nit, documentation): roster: approved (ruling 104)

Em-dashes in `//` comments, and a doc line broken mid-clause

Where: `crates/before/src/codec/bits.rs:60-60`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Replace the em-dashes in `//` comments with ` -- `; reflow dsi.rs:295-298

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-render-25 (nit, documentation): roster: approved (ruling 104)

nit: literal `\u2014` escapes inside JavaScript comments, and em-dashes in `//` comments across the partition

Where: `crates/before/docs/fuelscape.js:1055`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): replace `\u2014` with an em-dash or a colon at 1055, 1538, 1540; sweep the listed `//` lines to colons, semicolons, or spaced double-hyphens

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzz-guests-pins-17 (nit, documentation): roster: approved (ruling 104)

Moralized qualifiers ("honest", "real") at ten sites

Where: `crates/before/fuzzfit/guest/src/lib.rs:84-87`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Batch with the crate-wide prose pass; substitute the property at each site

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzz-guests-pins-7 (nit, documentation): roster: approved (ruling 104)

Em-dashes inside `//` comments at six sites

Where: `crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:296-296`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Batch into a crate-wide sweep rather than fixing these six alone; per site, a colon, semicolon, or parenthetical

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-1 (nit, documentation): roster: approved (ruling 104)

"honest" as a moral adjective and "sentry" as an unanchored coinage

Where: `crates/before/fuzzfit/harness/src/bands.rs:66-70`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Replace each "honest" with the mechanism it names ("pre-drift divergence", "in-band work", "the tail of legitimately cheap draws" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### paper-fidelity-13 (nit, simplification): roster: approved (ruling 104)

"mint" used for constructing values across the crate

Where: `crates/before/src/lib.rs:49`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One pass replacing "mint" at the 56 sites

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### prose-hygiene-15 (nit, documentation): roster: approved (ruling 104)

Hand-maintained counts in doc comments

Where: `crates/before/fuzzfit/harness/src/bands.rs:76-77`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): bands.rs: state the structure (one key per kernel plus one per

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### prose-hygiene-16 (nit, documentation): roster: approved (ruling 104)

Residual dialect tells: load-bearing, earns, backstop, surface-as-verb, flavour, story, dial

Where: `crates/before/src/meter/board/floors.rs:136-136`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): replace per the style tables in one sweep: load-bearing to "the

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### rank-15 (nit, documentation): roster: approved (ruling 104)

Em-dashes in // comments (19 lines); none in assert or expect messages

Where: `crates/before/src/version/rank.rs:621`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Replace with colons, semicolons, or parentheses at the listed lines, as part of a crate-wide pass

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### recursion-9 (nit, simplification): roster: approved (ruling 104)

"mint" for constructing values and coining terms is a crate-wide idiom

Where: `crates/before/src/meter.rs:18-19`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Sweep the 57 sites with the vocabulary pass

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-5 (nit, documentation): roster: approved (ruling 104)

em-dashes in 24 line comments and one assert message

Where: `crates/before/src/version/skyline.rs:239-239`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): replace each with a colon, semicolon, or restructured sentence; the assert message becomes "the adequacy witness went green: the kernel no longer demo ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-7 (nit, documentation): roster: approved (ruling 104)

"mints" for constructing an error value

Where: `crates/before/src/version/skyline/admit.rs:256-257`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "and returns [`Decode::NotCanonical`] for a [`Refuted`] verdict only after they pass"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-10 (nit, documentation): roster: approved (ruling 104)

Em-dashes in `//` comments (62 sites in the partition)

Where: `crates/before/src/version/skyline/fill.rs:456-459`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): If the owner rules for the double-hyphen in this crate, one mechanical pass over `//` (not `///`/`//!`) lines replacing ` — ` with ` -- ` or a colon ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-14 (nit, documentation): roster: approved (ruling 104)

Register texture that names no mechanism

Where: `crates/before/src/version/skyline/fill.rs:895-897`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): 897 drop "genuinely"; 1018 "their size"; memo.rs:88 "charge the heap meter for"; fuse.rs:36 "the route fold's cost"; grow.rs:36 drop "simply" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-22 (nit, documentation): roster: approved (ruling 104)

"forest parent" and "site forest" are used as terms without a definition

Where: `crates/before/src/version/skyline/fill/memo.rs:21-25`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One sentence before the bullets at memo.rs:17: the sites a scan records nest (a site can sit inside another's sibling range) ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-query-1 (nit, documentation): roster: approved (ruling 104)

Unanchored crate-dialect terms "seam" and "genre" in this partition's prose

Where: `crates/before/src/version/skyline/query.rs:121-123`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Rule once at crate scope. If the words stay, define each once by contrast at one home (the crate's vocabulary section or the first use) and link to it ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-query-4 (nit, documentation): roster: approved (ruling 104)

Em-dashes in `//` comments and in one assert message

Where: `crates/before/src/version/skyline/query.rs:388-390`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Rule once at crate scope; if the doctrine applies, sweep `//` comments to ` -- ` or restructure with a colon ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-sweep-place-masked-16 (nit, documentation): roster: approved (ruling 104)

Em-dashes in `//` comments at 25 sites

Where: `crates/before/src/version/skyline/place.rs:258-260`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): rewrite the 25 sites with colons, semicolons, parentheses, or spaced double-hyphens; the grep above enumerates them

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-sweep-place-masked-29 (nit, documentation): roster: approved (ruling 104)

Texture coinages used as jargon: currency, face, genre, "block consume", "real", point "tripwire"

Where: `crates/before/src/version/skyline/signed.rs:1-3`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): currency to "representation" or "exchange form" (signed.rs already anchors "exchange pair/shape" to `Signed`); face to "form" or "entry point" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-watermark-1 (nit, documentation): roster: approved (ruling 104)

Vocabulary sweep: 'mint' for constructing a value, moralized 'honest', shouted 'MOVES', 'genuinely'

Where: `crates/before/src/version/skyline/watermark.rs:44-1144`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): 66 "defines"; 129, 553, 628 "constructs"; 303 "creating it"; 350 "the fresh-latent move"; 361 "a fresh latent finds them `m`-exact" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-watermark-10 (nit, documentation): roster: approved (ruling 104)

Em-dashes in // line comments at eight sites

Where: `crates/before/src/version/skyline/watermark.rs:491-811`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): replace each with a colon, semicolon, or sentence break. Seven of the eight sit above line 790 ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-watermark-25 (nit, documentation): roster: approved (ruling 104)

Vocabulary collisions: 're-arm' for lease and 'fill phase' beside the fill walk

Where: `crates/before/src/version/skyline/pool_traffic.rs:4-18`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): watermark.rs:82 "return to a pool and are leased again cleared"; pool_traffic.rs:5 "and leases from it (`MinWeb::lease`)" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### span-causally-2 (nit, documentation): roster: approved (ruling 104)

Em-dashes inside `//` comments at fifteen partition sites

Where: `crates/before/src/span.rs:176-179`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): recast each with a colon, semicolon, or parentheses. The crate-wide sweep (374 lines) is a separate prose-pass decision; see the open questions

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-13 (nit, documentation): roster: approved (ruling 104)

Em-dashes in `//` comments, a TOML comment, and panic/error strings

Where: `crates/suanpan/src/accumulator.rs:603-603`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): a colon or semicolon at each site (e.g. 603 "keeps both free: no rebuild of a"; tests.rs:291 "no longer holds the #[test] fn `{witness}` ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-19 (nit, documentation): roster: approved (ruling 104)

Vocabulary tells: moralized code, a significance adverb, a mechanism-less "silently", three spellings of one term, colon-fronted labels and antitheses

Where: `crates/suanpan/src/accumulator.rs:992-992`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): drop "honest" at 992 ("one spelling of the value, not a normal form" already says it) and "genuinely" at tests.rs:321 ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-3 (nit, documentation): roster: approved (ruling 104)

Opaque `\[derived\]` tags in the crate page

Where: `crates/suanpan/src/lib.rs:72-72`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): delete both; if the intent is to mark which paragraphs the table's "derived above" (line 200) points at, say it in words

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-tests-17 (nit, documentation): roster: approved (ruling 104)

moralized and register-transplant vocabulary: "honest" (5), "real fold", "minted", and "tripwire" for criteria with no committed known-bad

Where: `crates/suanpan/src/accumulator/tests/metered.rs:337-337`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): metered.rs:337 "the difference is operand − receiver"; :678-679 drop the comment (the `#[allow]` says why) or "a named alias would only add a name to  ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### surface-roster-25 (nit, documentation): roster: approved (ruling 104)

Metaphors promoted to jargon without an anchor: "jaw"/"pincer", "honest reading", "earns", "the seal", "keystone", "a real roster row"

Where: `crates/before/surfacecheck/src/main.rs:19-20`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): rewrite as mechanism: "the census that pins each impl `FAMILY_SURFACE` disposes"; "two public spellings are two rows of surface" (drop "honest") ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-oracles-12 (nit, documentation): roster: approved (ruling 104)

"keystone" is used six times before any definition and as a first sentence that says nothing; "honest" and "genuine" moralize the correct reference

Where: `crates/before/src/testing/semantic_oracle/tests.rs:157-157`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Open the keystone doc with the invariant ("After one op trace, every ordered pair of final clocks has the same comparison descriptor under all three r ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### tests-other-8 (nit, documentation): roster: approved (ruling 104)

Register transplants and dash register across the partition

Where: `crates/before/tests/bench_judge_roster.rs:7-8`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "launder" -> "misclassify as expected"; `honest` -> `untampered`/`intact`; drop "genuine(ly)/real(ly)" or state the mechanism ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

