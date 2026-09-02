<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T49, T50, T51, T56, and T57 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P3 lane: vocabulary and register

## Goal

The crate's prose uses established terms only: no coinage stands for a
boundary or a setting (T49); "honest" names the authenticated-honest-peer
trust premise and nothing else, the lie family becomes the mechanism it
stands for, and the fixtures are renamed to match (T50); "V2" survives only
where the wire dialect number is the subject (T56); the nine
`missing_const_for_thread_local` comments name no platform the tree does not
build for, and AGENTS.md records the illumos hand run (T57); public rustdoc
opens in one mood per item kind (T51). Each sweep's grep reaches zero in its
own commit; T49 and T50 name no standing check. Effort: medium.

## Ground rules

These apply to every P3 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `<base sha>` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `<base sha>`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the topic document in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; the entry's full record
  (evidence, construction, demonstration) is under `### <id>:` there and in
  `evidence/`. Line anchors are at the reviewed commit `9e5784fb`; re-anchor
  from the quoted evidence, never from the line numbers.
- **Goal beside mechanism.** Where a quoted resolution and the goal above
  come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the resolution does not name;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop on
  one entry does not block the others.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it. The constructions in
  `evidence/witness.md` and each entry's Construction line are those
  artifacts; convert each into a committed test (`should_panic`, an asserted
  `Err`, a `--self-test` case, or a reversible mutation whose observed
  failure the commit message records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under `<scratchpad>/p3-vocabulary/`,
  polled with short foreground checks (the foreground command cap is ten
  minutes). Keep every working file under that directory. `just all` and
  `just ci` are run once each at the end, not per commit.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and ruling. Commit every proptest seed
  file that appears. Prose speaks in the present tense: no reference to code
  that no longer exists, no dated rationale at a declaration site. Comments
  use spaced double-hyphens, never em-dashes; every test has a doc comment
  stating its invariant. Never delete anything outside your worktree; if the
  disk fills, stop and report.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why. Your report is data: the coordinator
  verifies each entry's Acceptance against the tree at the reported sha
  before the ledger records it. Report what you could not do rather than
  working around it.
- **Prose.** `PROSE.md` in this directory binds this lane (ruling T141): every
  paragraph you touch passes its three tests (altitude, concision,
  legibility), and the diff is net shorter in prose unless your report says
  what the added sentences buy.
- **Machine.** The illumos box (`ox-east-1`, per the `building-on-illumos` skill) is
  where a lane builds, tests, and gates. The wrapper syncs the Mac
  worktree to `~/src/<worktree basename>` on the box and runs one command
  there with its own target directory, so lanes do not collide; cargo
  runs `--locked` there; nothing is edited or committed on the box. A
  clean gate on the box is the gate of record for a commit; the Mac runs
  no gate (Finch's ruling). The box gate is `on-illumos.sh <worktree> 'just gate'`. One leg is
  expected red there and counts as clean when it is the only failure:
  `fuzz`, because libFuzzer has no illumos port (`FuzzerPlatform.h`
  refuses the target); a lane quotes that line and runs no fuzz build
  elsewhere (Finch's ruling: fuzzing is CI's). clippy's
  `missing_const_for_thread_local` misfires on illumos, where
  `thread_local!` expands through the OS-keyed path; `before` carries an
  illumos-scoped crate-level allow, so a lane based before that landed
  either rebases or passes `RUSTFLAGS="-A clippy::missing_const_for_thread_local"`
  in the remote command for that one run. Two legs that
  pin toolchain-derived numbers may fire on the box if its toolchains
  differ from the pinned ones; a lane reports such a leg with both numbers
  rather than re-pinning anything. `tools/memwatch` is deleted by the
  `p1-memwatch` lane, which runs first and unstacked; `p1-gate` rebases
  onto it. Benchmarks whose committed baselines are the Mac's run on the
  Mac, once, on a quiet machine. Clock guard, checked before every box
  run: rsync preserves mtimes and cargo's rebuild detection is
  mtime-based, so a box clock ahead of the Mac by more than a couple of
  seconds means a green build of stale code; on skew, a lane either runs
  with a fresh target directory on the box (a cold build, no stale
  artifact to trust) or waits, and says which. Stepping the box's clock is
  admin work on a shared machine and is Finch's, never a lane's. The
  builder cap counts Mac builders only; on the box (96 cores, 1 TiB) there
  is no lane cap, only the load: hold a launch while the one-minute load
  average sits above about 150 on 192 threads, and keep wall-time
  measurements under `pset-run` to one at a time, announced in the merge
  queue first.
- **Out of scope.** The formal tier (`lean`, `eventdag`, `muxprobe`, everything under
  `formal/`) and `before`'s bench judge (`bench-judge`,
  `bench-judge-tripwire`) are never run or edited by a rumors lane; a
  recipe that composes them (`all`) is exercised by its other legs
  individually. A rustdoc on a Rust-side literal derived from the Lean
  artifact is Rust prose and may be edited where a ruling names it.

## Mechanism, in order

`<paths>` is `src tests benches examples justfile Cargo.toml AGENTS.md README.md design .github` (`README.md` is derived: edit `src/lib.rs`, then `just readme`; `crates/` and `tools/` are the `before` triage's).

1. **T49, one commit per word family**, each use rewritten as the mechanism
   it names ("boundary", "setting", the trait method, the `before` item linked
   by name); the `Knob` fixture in `src/conformance/backend/tests.rs` takes a
   plain noun. Oracle: `git grep -n -i -P '\b(seams?|knobs?)\b' -- <paths>` empty (61 and 79 sites at `6c90bd7d`); each row's own words gone.
2. **T50, one prose commit.** Renames: `HONEST_LEN` to `FULL_LEN`, `Dishonest`
   to `Skewed`, `is_honest_error` and kin to `is_injected_cut` and kin,
   `GreetingLie` and `tell` to a misdeclaration name (reported). Oracle: `git grep -n -i -P '\b(dis)?honest(y|ly)?\b' -- <paths> | grep -v -i -P 'honest(-| )(peer|peers|member|members)|authenticated-honest'`
   empty, and `git grep -n -i -P '\b(lie|lies|lied|lying|deceiv\w*|malicious\w*)\b' -- <paths>`
   printing only positional "lies" (237 and 77 sites at `6c90bd7d`).
3. **T57, one commit, after step 2.** The nine comments become one text: the
   allow exists because a target the crate supports flags the item under
   `-D warnings`; no platform named, no "honest". One AGENTS.md line: the illumos
   build is a hand run on ox-east-1. Oracle: `git grep -n -i illumos -- src tests` empty; the allows stay.
4. **T56, one commit.** Behavioral prose drops "V2"; prose whose subject is
   the dialect number, identifiers, and wire constants keep it. Oracle: the
   rows' greps, plus every surviving prose `V2` (`git grep -n -P '\bV2\b' -- src tests benches examples | grep -P ':\s*(///|//!|//)'`)
   listed in the commit message with the subject that keeps it.
5. **T51, last, after `p3-lints`'s new rustdoc.** One mood per item kind; the
   three sentences appearing in both moods unify. Extend `tools/doclint` only if
   a cheap rule exists (a first-word deny list is a tripwire, not a check: say which); otherwise the sweep is one-time, and say why.

## Members

Nit rows are quoted from their table row (`| <id> |`); heading entries carry Acceptance.

### T49 (owner decision 4)

- **prose-hygiene-10** (low). Resolution: In public rustdoc name the boundary ("**Bulk methods**: the backend's own `leaves` and `assemble` overrides"; "at the point named in its field docs"; "the mechanism and the point it is counted at"). In private comments and tests, "boundary", "interface", or the method name is a one-word substitution. Acceptance: `grep -rniE '\bseam(s)?\b' src` returns nothing in `///` or `//!` lines; private uses at the owner's discretion. (T49: no private use survives either.)
- **mirror-common-34** (low). Resolution: stats.rs: "at the point named in its field docs" / "taken exactly at that boundary, between the codec and the transport stream" / "Counted at the same codec boundary as"; erased.rs and streaming.rs: "the height-erased boundary"; framing.rs:42 "at the chunk boundaries"; framing/tests.rs:18 "every partial-read boundary". Acceptance: `grep -rn -i seam` over the nine files is empty.
- **streaming-tests-8** (medium). Resolution: Name the thing at each site and keep the Lean names: "the payload-independence bridge (announced-skeleton reconstruction)" for B5; "the mux impossibility theorem `wc_impossibility`" for T3; drop "adjudication repair F4" and say "for an impossibility, realizability flows from Rust to the model"; "Parent-placement probe" for "finding #7"; drop the `Bridge N` ordinals (each file's first sentence already names its bridge); "the fact the locality theorems rest on" for "charter locality"; "the model's" for "adjudicated"; "parameters" for "knobs"; "A malformed reply" for "A genuine malformed reply"; "held"/"absent" for "genuinely held"/"genuinely absent". Sweep the out-of-partition sites in the same pass so production and tests keep one vocabulary. Acceptance: `grep -rnE '\b(B5|T3|F4)\b|Bridge [0-9]|finding #[0-9]|adjudicat|charter|knob|genuine' src/tree/mirror/streaming` is empty; every formal citation is a Lean theorem or definition name. (T128 keeps `B5` where materialized-20 and prose-hygiene-4 agree; report the conflict at those sites.)
- **testing-infra-6** (low). Resolution: tests.rs:87: "a buggy or nonconforming peer". memnet.rs:10: "names are plain strings, so nothing here can be mistaken for an IP address". transport.rs:21-24: "The endpoint the report attributes operations to; which is which is the test's choice." Define "inversion" once (a released batch of two or more) and drop the "genuine" qualifiers; "waits, yielding, for a further arrival" for "genuinely waits"; "degenerates to pass-through with no failing assertion" for "silently degenerates"; "divergence on both sides" for "honest divergence"; "correct" for "sound"; rewrite tests.rs:543-548 as "the same rejection the mirror suites pin in process and over their wires, observed here through `Rumors::gossip`; the poisoned store is built by a local `Tree::join`, which no session check guards". Acceptance: the grep above returns only "honest peer" at transport.rs:64 and 616; "proxy endpoint" is gone from transport.rs.
- **conformance-22** (nit). Resolution: Name the method; "seam" is a crate-wide ruling.
- **fresh-eyes-11** (nit). Resolution: "boundary" at the five public sites.
- **link-13** (nit). Resolution: Reword per site; owner vocabulary, so batch for a prose pass.
- **materialized-23** (nit). Resolution: Reword; delete the banner and the narration.
- **prose-hygiene-11** (nit). Resolution: Reword per site; `just readme`.
- **remote-capture-atlas-7** (nit). Resolution: "the renderer" or "the traversal". (`capture.rs` is rewritten by `p1-renderer`; re-anchor.)
- **remote-proxy-17** (nit). Resolution: "flushed questions"; "decode-side scope queue".
- **session-bookmark-5** (nit). Resolution: "knob" to "setting" now; "seam" is a crate-wide ruling (owner-gated).
- **streaming-backend-window-3** (nit). Resolution: Reword; keep at most two antithesis openers per module.
- **tests-bookmark-22** (nit). Resolution: Mechanism words at each site; route "seam" to the crate-wide sweep.
- **tests-observation-31** (nit). Resolution: "party disjointness", or cite `fork_halves_disjoint`.
- **tree-core-12** (nit). Resolution: Reword per site.
- **tree-core-20** (nit). Resolution: Open with the mechanism in plain terms; link each borrowed term.
- **tree-typed-28** (nit). Resolution: Name `Span::at` and `Span::dominance`.

### T50 (owner decision 5)

- **remote-proxy-tests-14** (low). Resolution: Sweep to the recorded term: "misdeclared"/"under-declared"/"a declaration its traffic does not honor"; "the side hearing the misdeclaration" for "the deceived side"; panic messages "undetected set_len misdeclaration: ...". Do it crate-wide in one prose commit, renaming `GreetingLie` in the same pass; drop the secondary moralizers "genuinely batched" (declarations.rs:70) and "genuinely pristine" (greeting.rs:181) while there. Acceptance: `grep -rn -i -w 'lie\|lied\|lies\|deceived\|deceive' src tests` returns only positional uses of the verb.
- **session-bookmark-16** (low). Resolution: "a plain wire cut" / "an EOF"; "a newborn claimant (empty greeting version)"; "a bootstrap whose greeting version is empty"; "untouched"; drop "real" and "genuine" where the noun carries the meaning. Decide separately whether the sim harness keeps its "honesty of failures" vocabulary or renames toward "cut versus corruption", so the crate has one sense of the word. Acceptance: `grep -n -i 'honest\|genuine' src/peer/gossip.rs src/peer/gossip/tests.rs src/bookmark/format/tests.rs` returns nothing; "honest" appears in the crate only in its trust-model sense.
- **tests-common-23** (low). Resolution: rename to the mechanism (`is_injected_cut`, `assert_injected_cut`, `cut_io`, `cut_remote`, `assert_session_cut_or_ok`) and write "attributable to the cut" / "not attributable to a cut (a decode or protocol failure)" in prose; update the callers in tests/disruption.rs. Rule the vocabulary once for the crate (src/peer/gossip/tests.rs and six binaries share it) rather than renaming here alone. Rewrite wire.rs:25 ("keeps `-D warnings` clean") and seed_liveness.rs:347 ("the `.txt` seed stays"). Acceptance: no identifier in tests/common uses honest/dishonest/honesty; prose uses the word only in the model-of-record sense.
- **remote-codec-2** (low). Resolution: "Decoding is where conformance is checked" / "The encoder performs no conformance checks" / "a conforming encoder"; "independent" for "orthogonal"; drop "real" and "genuinely"; "at every chunk boundary" for "at every seam". Acceptance: `grep -rn "trust boundary\|honest encoder" src/tree/mirror/streaming/remote/codec` returns nothing; the other sites read in plain terms.
- **benches-envelope-26** (nit). Resolution: Plain terms; keep the model-of-record "honest peer". (`examples/envelope_sim.rs` is deleted by `p1-envelope`; `benches/support/grid.rs:142` remains.)
- **conformance-32** (nit). Resolution: Rename to accuracy terms (`accurate`, `Skewed`); owner-gated.
- **conformance-4** (nit). Resolution: Per-site pass; keep the "real sockets" contrasts.
- **mirror-common-36** (nit). Resolution: Reword; recast `Reaction` in `Greeting`'s third-person voice.
- **remote-adapter-tests-8** (nit). Resolution: Rewrite to the property that holds.
- **remote-capture-atlas-18** (nit). Resolution: Reword both.
- **tests-common-27** (nit). Resolution: Reword per site.
- **tests-disruption-handshake-1** (nit). Resolution: Delete, or name the counterpart contrasted.
- **tests-observation-19** (nit). Resolution: "yields an equal checkpoint"; drop "honest".
- **tests-resource-link-window-6** (nit). Resolution: Rename `HONEST_LEN` to `FULL_LEN`; reword per site.
- **tests-wire-format-15** (nit). Resolution: Rewrite each as mechanism; cite `DISPUTE_OVERHEAD_BYTES` by name.
- **swarm-example-4** (nit, T27, moot). Resolution: Delete or restate as mechanism. Disposed by the example's deletion (`p1-swarm`); `test ! -e examples/swarm.rs` at base, quoted in the report.

### T56 (owner decision 11), T57 (owner decision 12), T51 (owner decision 6)

- **session-bookmark-12** (low, T56). Resolution: 616: match the field doc at 1099-1101 ("inert unless a handler is attached"). 723-729: single-path prose ("The reconciliation runs behind the non-generic [`Reconciliation`], whose future is boxed so the protocol state machine stays out of this session future and out of consumer crates"). 63: "the protocol tower". Delete "under V2" at 258 and 473 and "whichever protocol carries it" at 1264. In tests.rs drop the `v2_` prefixes and the "under V2" qualifiers at 247 and 316. Whether "V2" survives as the dialect's name in prose (51, 1276, tests.rs:6, 184) follows the owner's ruling on the `Protocol` enum. Acceptance: `grep -n 'dialect\|selected protocol\|Both branches\|neither concrete\|both towers\|whichever protocol\|fn v2_' src/peer/gossip.rs src/peer/gossip/tests.rs` is empty. (T82 keeps the `Protocol` enum; the dialect name stays where the number is the subject.)
- **tests-common-7** (nit, T56). Resolution: Drop the qualifier.
- **remote-adapter-streams-11** (low, T57). Resolution: State the platform-independent mechanism without the roster, at all ten sites: "clippy's `missing_const_for_thread_local` fires on `const {}` initializers when `thread_local!` lowers to fallback TLS; the allow keeps `-D warnings` clean under either lowering." If illumos is meant to be a gate platform, make it one in the justfile or CI so the claim becomes checkable. Deduplicating the ten copies (a shared `thread_local!` helper or a crate-level allow) is a separate question. Acceptance: the comment names no platform the verification recipes do not name, or the recipes name it; "honest" is gone from every copy.
- **remote-proxy-22** (low, T57). Resolution: either record the illumos gate run where a reader can find it (a justfile recipe or an AGENTS.md line) and cite it, or restate the comment platform-free ("clippy's `missing_const_for_thread_local` denies `const`-block initializers on targets that lower `thread_local!` through fallback TLS; the allow keeps `-D warnings` clean there."). Apply the same text at all eight in-scope sites, or hoist to one note the others point to. Acceptance: the comment cites an in-tree location or makes no platform claim; all sites read identically. (T57: both halves, the AGENTS.md line and the platform-free text.)
- **inventory-13** (nit, T57). Resolution: One crate-level `#![allow(clippy::missing_const_for_thread_local)]` with the rationale once; delete the thirteen per-static allows and seven comments (superseded by T57: the allows stay at their sites.)
- **api-core-17** (nit, T51). Resolution: Owner ruling, then one sweep; at minimum make peer.rs:710, rumors.rs:410, and snapshot.rs:163 identical (owner-gated). **Stop before the sweep:** the three named sites are the `warm_caches` docs T96 deletes, and T51's words ("every public type's and module's rustdoc is imperative, matching `Peer`, `Rumors`, `Bootstrap`") do not match the tree, where those type docs open as noun phrases ("The start and end of a `Rumors`'s lifecycle") and the mixed mood is in method docs ("Force" against "Forces"). PROSE.md reads T51 as one mood per item kind. Report which reading the coordinator wants before rewriting any type doc.

## Hazards and stops

Launch after every P1 and P2 lane has merged (`p1-renderer` rewrites
`capture.rs`; `p1-harness-tests` rewrites `tests/common`, `transport.rs`,
`disruption.rs`; `p2-walk` runs T40's "aborts typed" sweep over the same
prose), after `p3-modules` (its AGENTS.md line) and `p3-lints` (its new
rustdoc is T51's input). A rename of a public identifier, or a rewrite that moves a snapshot, is a stop.
