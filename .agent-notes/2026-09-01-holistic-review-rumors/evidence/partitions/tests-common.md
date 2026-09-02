# Partition tests-common: The integration-test common library: sim, schedules, faults, oracles, wire, snapshots, seed liveness

## Partition summary

`tests/common` is the library every category binary under `tests/` builds on. It has three layers. The generators produce inputs that are valid by construction: `action.rs` draws single-peer insert/redact sequences, `schedule/arb.rs` draws multi-peer schedules (with a membership alphabet of mid-schedule bootstraps and retirements) while a shadow simulator tracks what each peer has observed so that every emitted `Redact` names a message its peer holds, `overlap.rs` draws schedules in which hand-driven sessions are opened, parked at chosen poll prefixes, and closed, and `window.rs` draws the per-peer window configuration the suites sweep. The executors run those inputs against real peers: `schedule/executor.rs` one session at a time over in-memory links under the closed-world poller (`wire.rs`), `sim.rs` all at once on a multi-thread runtime over links that `fault.rs` severs at chosen byte offsets, with a value ledger and a custody chain that decide which invariants are sharp. The oracles and instruments are kept structurally independent of the merge machinery: `oracle.rs` is a `BTreeMap` keyed by event index and reads redaction as absence, `peer.rs` keeps an observation log by pull-draining snapshots, `gossip_snapshot.rs` captures every wire byte for the `insta` pins and holds the public observation hook to that capture, `shape.rs` stages deterministic tree shapes, `tcp.rs` and `routed_tcp.rs` instantiate the link contract over sockets, `flaky.rs` fails bookmark storage on a schedule, and `tests/main.rs` plus `tests/seed_liveness.rs` anchor and audit where proptest persists its seeds.

I read all twenty files in full with line numbers, 5189 lines, every one of them test code, and verified the candidate findings against the crate sources they depend on (`src/rumors.rs`, `src/rumors/unordered.rs`, `src/snapshot.rs`, `src/peer/gossip.rs`, `src/link.rs`, `src/testing.rs`, `src/testing/transport.rs`, `src/tree/typed/hash.rs`, `src/tree/typed/untyped/fan.rs`, `src/bookmark/format.rs`, `src/protocol.rs`, `Cargo.lock`) and against git history where a finding turns on provenance. No cargo, just, or test command was run; nothing below rests on a build.

The harness is in good shape. I found no harness bug that masks a failure: every liveness bound panics at its source, every successful in-memory session asserts a drained control stream, the fault engine's error classifier rejects decode-class errors as protocol bugs (keeping the disruption engine a conformance-bug detector, as the model of record requires), the value oracle is gated on possible identity loss rather than on the presence of faults and has a committed tripwire, the schedule shadow is meta-tested against the live executor, and the seed sweep guards its own vacuity and carries fixtures for both verdict classes. Constants carry their rationale where they are declared, and where they are measurements they name the pin that re-derives them.

The dominant issues are housekeeping that a blanket `#![allow(dead_code, unused_imports)]` lets accumulate (two dead imports, a dead field, a no-op statement, a stray scope, all from named commits), duplication between sibling modules and against the test binaries (a second shadow simulator, a fingerprint tuple in twelve places, a bootstrap handshake reimplemented in four suites), two documentation claims that overstate what the code does (an "exact sequence" only the set of which is pinned; a fork "captured" at `Open` that the live session performs at its first poll), one module doc that motivates a number by a data structure removed from the tree the same day it was written, and two coverage duals the fault plans never construct (the donor side of a bootstrap, the absorber side of a retirement). Two findings are medium; nothing is high.

## Findings

### tests-common-1: serde imports appended as an orphan group with no blank line before the next item
- Where: tests/common/action.rs:7-9 (related: tests/common/peer.rs:20-22, tests/common/wire.rs:20-22, tests/common/gossip_snapshot.rs:53-55, tests/common/schedule/executor.rs:22-24, tests/common/overlap.rs:44-46; outside the partition tests/pairwise.rs:28-30)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn -A1 '^use serde::de::DeserializeOwned;' tests/common tests/pairwise.rs`: a non-blank item follows at all seven sites)
- Seen by: structure-prose; refutation: confirmed; history: no rationale (all seven produced by the mechanized sweep c6fe4018)
- Owner-gated: no

In six partition files the two serde imports sit after a blank line, apart from the other external-crate imports, and the next item (a `const`, a doc comment, a `pub struct`, a `//` comment) follows with no separator. rustfmt repairs neither. The import block is the first thing a reader scans; an orphaned group and a missing separator each cost a double-take.

Evidence:

         7	use serde::Serialize;
         8	use serde::de::DeserializeOwned;
         9	const MAX_ACTIONS: usize = 16;

Resolution: merge `serde::{Serialize, de::DeserializeOwned}` into the external-crate group and leave one blank line before the first item, at all six partition sites (and tests/pairwise.rs:28-30). Acceptance: the grep above shows a blank line after each `use serde::de::DeserializeOwned;`.

### tests-common-2: the version a `send` created is recovered five different ways
- Where: tests/common/action.rs:47-64 (related: tests/common/peer.rs:77-87, tests/common/shape.rs:35-44, tests/gossip_snapshot.rs:41-47, tests/retire_redaction.rs:25-30, src/rumors.rs:165, src/rumors.rs:190-203)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read `Rumors::send` at src/rumors.rs:165 and its rationale at 190-203; `sed` of all five recovery sites)
- Seen by: api-economics; refutation: confirmed (keep `insert_one`'s observation-count assertion if the routine is swapped); history: deliberate-and-holds for the API half (the rationale is recorded in the code itself)
- Owner-gated: yes: the API half reopens a ruling recorded at src/rumors.rs:190-203

`Rumors::send` returns `Result<(), EncodeError>` by documented decision. The harness, the crate's heaviest user, recovers the version five ways: `created_version` (range since a recorded frontier, asserting exactly one), `Peer::insert_one` (drain-diff asserting exactly one new observation), `shape::pool` (scan by payload), `version_for` in tests/gossip_snapshot.rs (scan by payload), and an inline `snapshot().iter().map(..).next()` in tests/retire_redaction.rs. One invariant ("a send creates exactly one live leaf") is asserted in two places and assumed in three. The batching half of the recorded rationale (sends are not 1:1 with insertions) does not bind the single-message `send`; the state-machine half does.

Evidence:

        47	/// Returns the [`Version`] of the single live leaf in `snapshot` above
        48	/// the causal frontier `pre`.
        49	///
        50	/// This is how a builder recovers the version a `send` just created,
        51	/// given the `latest()` it recorded before sending.

    (tests/retire_redaction.rs:25-30)
        let version = b
            .snapshot()
            .iter()
            .map(|(version, _)| version.clone())
            .next()
            .expect("the sent entry is live");

Resolution: test side now: make `created_version` the one recovery routine; have `Peer::insert_one` call it while keeping its own "exactly one new observation" assertion (that one is about the log, not the leaf); replace `version_for` and the retire_redaction.rs scan with `created_version` or a `shape::version_of_payload`. Owner side: rule whether the single-message `send` should return the `Version` it stamped; if not, the recorded rationale stands and the harness routine is the accepted cost. Acceptance: one recovery helper in tests/common; no ad hoc `find_map`/`.next()` version recovery in tests/*.rs; the API ruling recorded.

### tests-common-3: two byte-budget fault injectors coexist and neither doc names the other or what separates them
- Where: tests/common/fault.rs:1-26 (related: src/testing/transport.rs:186-215, src/testing/transport.rs:247-262, tests/common/sim.rs:412-419)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read `State::failure` and `State::remaining_bytes` in src/testing/transport.rs: byte-unit faults never constrain `Connect`/`Accept`, and every injected failure is `io::Error::other(injected)`, kind `Other`, which `honest_io` at sim.rs:412-419 rejects)
- Seen by: structure-prose; refutation: reframed (fault.rs, 2026-06-10, predates `testing::wrap_link`, 2026-07-16, so `wrap_link` is the second injector); history: no rationale for coexistence
- Owner-gated: no for the missing sentence; dissolving either injector is the owner's option

`rumors::testing::wrap_link` and `fault.rs` both sever a link at a byte offset. Two properties separate them, and neither module states them: fault.rs refuses new streams once a direction's budget is spent (lines 15-18, 201-207, 227-233), where transport.rs's byte-unit faults never constrain the stream supply; and fault.rs raises typed `BrokenPipe`/`ConnectionReset`, which the disruption engine's classifier keys on, where transport.rs raises `io::Error::other(InjectedIo)`. A maintainer meeting both cannot tell which to reach for or whether one can go. Infrastructure earns its place by naming what it serves that the existing tool does not.

Evidence:

         1	//! Wire-fault injection for the disruption simulations: deterministic,
         2	//! byte-budgeted severing of either direction of a gossip link.
    ...
        15	//! happens to travel. A severed direction also refuses new streams: once
        16	//! its budget is exhausted, [`FaultConnector::connect`] fails alongside the
        17	//! writers and [`FaultAcceptor::accept`] alongside the readers, because a
        18	//! dead connection cannot open or deliver streams any more than it can

    (src/testing/transport.rs:258-260)
                // Supply operations transfer no bytes, so a byte-counted fault
                // never constrains them.
                Operation::Connect | Operation::Accept => return usize::MAX,

Resolution: add one paragraph to fault.rs's module doc (or to transport.rs's, or both) naming `rumors::testing::wrap_link` and the two properties the disruption engine needs that it lacks: stream-supply refusal on exhaustion, and severed-transport error kinds the classifier recognizes. If the owner prefers one injector, extend `IoPlan`/`IoFault` with those two properties and dissolve fault.rs into `wrap_link`. Acceptance: each injector's module doc names the other and the property that distinguishes them, or one of them is gone.

### tests-common-4: `metered` repeats `faulty_link`'s wrapping body
- Where: tests/common/fault.rs:102-125 (related: tests/common/fault.rs:78-85, tests/common/fault.rs:128-155)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both bodies: the same `LinkParts { Cut, Fuse, FaultConnector, FaultAcceptor, session }.into_link()` construction, differing only in the two budget clones kept for the meter)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale (`metered` copied the body at 919132dc; the same-day fix round left it)
- Owner-gated: no

The `ByteMeter` doc claims "the meter and the cut draw on identical accounting" (lines 83-84). Today that is true because two copies of the wiring happen to agree; one private core taking the two budgets makes it true by construction.

Evidence:

       102	pub fn metered(link: MemoryLink) -> (FaultyLink, ByteMeter) {
       103	    let write = budget(None);
       104	    let read = budget(None);
    ...
       110	    let link = LinkParts {
       111	        control_read: Cut::new(parts.control_read, read.clone()),
       112	        control_write: Fuse::new(parts.control_write, write.clone()),

       141	    LinkParts {
       142	        control_read: Cut::new(parts.control_read, read_budget.clone()),
       143	        control_write: Fuse::new(parts.control_write, write_budget.clone()),

Resolution: add `fn wrap<CR, CW, C, A>(link: Link<CR, CW, C, A>, write: Budget, read: Budget) -> Link<Cut<CR>, Fuse<CW>, FaultConnector<C>, FaultAcceptor<A>>`; `faulty_link` calls it with `budget(plan.write_cut)`/`budget(plan.read_cut)`, `metered` with two `budget(None)` whose clones go into the meter. Acceptance: one `LinkParts { .. }.into_link()` in fault.rs.

### tests-common-5: hand-written `Clone` where `derive` is identical
- Where: tests/common/fault.rs:189-196 (related: tests/common/fault.rs:158)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read: `Budget` is `Arc<Mutex<usize>>`, so `#[derive(Clone)]` yields `impl<C: Clone> Clone` with these exact field clones)
- Seen by: structure-prose; refutation: confirmed (the alias already existed at the impl's birth commit); history: no rationale
- Owner-gated: no

A manual `Clone` impl signals that derive would be wrong (a bound to avoid, a field to skip). Here it signals nothing, so a reader looks for a reason that is not there.

Evidence:

       158	type Budget = Arc<Mutex<usize>>;

       189	impl<C: Clone> Clone for FaultConnector<C> {
       190	    fn clone(&self) -> Self {
       191	        Self {
       192	            inner: self.inner.clone(),
       193	            budget: self.budget.clone(),
       194	        }
       195	    }
       196	}

Resolution: `#[derive(Clone)]` on `FaultConnector`. Acceptance: no manual `Clone` impl in fault.rs.

### tests-common-6: the harness transcribes two crate derivations that `rumors::testing` could export, and a fixture self-check built on one compares it with itself
- Where: tests/common/flaky.rs:40-45 (related: tests/common/flaky.rs:57-64, tests/common/shape.rs:15-19, src/tree/typed/hash.rs:257-259, src/bookmark/format.rs:423, src/testing.rs:6-31, tests/gossip_snapshot.rs:107-108)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (`PathHash::of` at src/tree/typed/hash.rs:257-259 is `Sha3_256::digest(bytes).into()`, the same derivation as shape.rs:18; `bookmark::format::decode` at format.rs:423 is `pub(crate)`; the `testing.rs` export list has no path or bookmark helper; the comment at tests/gossip_snapshot.rs:107-108 read)
- Seen by: structure-prose (named constants), blind-spots (self-check independence), api-economics (transcribed internals); refutation: confirmed all three; history: deliberate-but-expired (flaky.rs's "cannot reach the crate's codec" was written a month before `rumors::testing` existed)
- Owner-gated: yes: the preferred fix adds exports to the feature-gated `testing` surface

`shape::leaf_path` recomputes the crate's leaf address by hand, and `persisted_record_bytes` re-walks the bookmark frame (tag `55799`, a three-item array, payload at index 2, tag `24`) against a rationale that has expired: `rumors::testing` is the sanctioned test-internals door and already exports comparable helpers. The cost is not only drift risk: the batched-run fixture's self-check in tests/gossip_snapshot.rs recomputes prefixes with the same `leaf_path` that `shaped_pair` selected by, so it can catch a `keep_only` mistake but not the derivation drift its comment says it catches. If the transcription stays, its numbers should at least be named.

Evidence:

        40	/// Integration tests cannot reach the crate's codec, and don't need to: the
        41	/// stored file is one self-described CBOR item, so a generic walk — unwrap
        42	/// tag 55799, take the frame array's payload item, unwrap tag 24, then strip
        43	/// each stored clock's tag — recovers the untagged record serde understands.
    ...
        57	    let Value::Tag(55799, frame) = file else {
    ...
        63	    let payload = items.into_iter().nth(2).expect("a three-item frame array");
        64	    let Value::Tag(24, payload) = payload else {

    (tests/common/shape.rs:15-19)
        15	/// A leaf's tree path: the full-width SHA3-256 hash of its version's
        16	/// canonical bytes.
        17	pub fn leaf_path(version: &Version) -> [u8; 32] {
        18	    sha3::Sha3_256::digest(version.as_bytes()).into()
        19	}

    (tests/gossip_snapshot.rs:107-108)
        // Self-check the landed shape: if hashing or version assignment
        // drifts, fail here with a clear message rather than in the hex.

Resolution: preferred: export `testing::leaf_path(&Version) -> [u8; 32]` delegating to the crate's leaf-address derivation and `testing::decode_bookmark_record(&[u8]) -> Result<BTreeMap<Network, Vec<Clock>>, FormatError>` delegating to `bookmark::format::decode`; delete both transcriptions; the self-check then compares the crate's derivation with the landed wire. Fallback: name the constants (`SELF_DESCRIBED_CBOR_TAG`, `EMBEDDED_CBOR_TAG`, `FRAME_PAYLOAD_INDEX`) at the top of flaky.rs, reword flaky.rs:40 to what is true today, and reword tests/gossip_snapshot.rs:107-108 to claim only what a same-function comparison checks. Either way, give the batched-run fixture a transcript-level assertion (a run frame carrying exactly two records) before its snapshot, as the radix fixtures already have. Acceptance: no `Sha3_256::digest`, `55799`, or `Tag(24` in tests/common, or each is a named constant; no comment claims a same-function comparison detects hash drift.

### tests-common-7: "V2" qualifiers with one protocol
- Where: tests/common/gossip_snapshot.rs:389-399 (related: tests/common/sim.rs:422, src/protocol.rs:15-19)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn V2 tests/common` returns exactly these three lines; src/protocol.rs defines the single variant `V2 = 2`)
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (the V1 retirement 368da2a5 swept "V2" from this module's header but not from `capture_session`'s doc or sim.rs:422)
- Owner-gated: no

With one dialect, "V2 protocol sessions" and "the V2 protocol's deterministic observable ordering" distinguish nothing; the qualifier is residue of the retired split. The variant itself is deliberately named `V2` (a knob with one position, kept for a V3), so only the qualifier where it distinguishes nothing is at issue.

Evidence:

       389	/// Capture and render an arbitrary pair of V2 protocol sessions.
    ...
       397	/// renderer preserves exact items per stream but keys data streams by
       398	/// their labeled index, which is the V2 protocol's deterministic
       399	/// observable ordering. A driver must run its session to completion

    (tests/common/sim.rs:422)
       422	/// Recognize only typed V2 surfaces directly caused by a severed transport.

Resolution: "an arbitrary pair of sessions", "the protocol's deterministic observable ordering", "typed mirror surfaces". Acceptance: `grep -rn V2 tests/common` is empty.

### tests-common-8: forty-two binaries compile the harness from source, and the arrangement needs a blanket allow
- Where: tests/common/mod.rs:3-4 (related: tests/common/mod.rs:31-33, Cargo.toml:145)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified for the count (`grep -l '^mod common;' tests/*.rs | wc -l` is 42 of 58 binaries; tests/common totals 5189 lines); the compile-time cost is assessed, not measured
- Seen by: api-economics; refutation: severity down to low (unmeasured; the cheaper half is finding 10); history: no rationale for the layout beyond the allow's comment
- Owner-gated: yes: a workspace layout decision

Every binary that writes `mod common;` re-parses and re-typechecks the whole harness, and the blanket allow at 31-33 exists only so that arrangement lints clean. A path dev-dependency crate (the manifest already carries a self-referential dev-dependency at Cargo.toml:145, so the cycle is permitted) compiles the harness once and restores private-item and import linting; `pub` items in a library are not dead-code-linted either, so the allow's removal buys less than the whole. The compile saving is a hypothesis until measured.

Evidence:

         3	//! Each per-category test binary pulls this module in via `mod common;`
         4	//! and reaches its pieces through `crate::common::*`.

Resolution: measure first (`cargo build --tests --timings` at HEAD and on a branch with `crates/rumors-testkit`, on a quiet machine), then decide. Keep `tests/main.rs` as the seed anchor either way. Acceptance: the two numbers recorded in the decision; if adopted, no `mod common;` in tests/*.rs and no module-wide allow.

### tests-common-9: the composition map omits `overlap` and `shape`
- Where: tests/common/mod.rs:6-29 (related: tests/common/mod.rs:40, tests/common/mod.rs:44)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (compared the twelve linked modules in 6-29 against the fourteen `pub mod` lines at 35-48)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale (each module landed as a one-line `pub mod` addition; the header rewrite at 0d48b153 postdates `overlap` and did not add it)
- Owner-gated: no

The "How the pieces compose" list is a hand-maintained enumeration of module contents, the kind the doctrine warns rots silently, and it has: `overlap` (the only harness that samples chosen interleavings) and `shape` (fixture staging for the wire pins) are absent. A reader using the map as the tour misses two of fourteen modules.

Evidence:

        28	//! - [`gossip_snapshot`] captures a session's exact bytes for the `insta`
        29	//!   pins.

        40	pub mod overlap;
    ...
        44	pub mod shape;

Resolution: add a bullet for each (`overlap`: a session parked at a chosen poll prefix while other events run, with its own valid-by-construction generator; `shape`: deterministic tree-shape staging for the pin fixtures). Alternatively restructure the map to state composition relations only and let each module's first doc sentence carry its role, so the list cannot rot. Acceptance: every `pub mod` in mod.rs appears in the map, or the map no longer enumerates modules.

### tests-common-10: `unused_imports` is allowed module-wide without a reason that covers it, and the residue it hides
- Where: tests/common/mod.rs:31-33 (related: tests/common/wire.rs:14, tests/common/sim.rs:86, tests/common/fault.rs:86-97, tests/common/flaky.rs:199, tests/common/peer.rs:57-61, tests/common/sim.rs:687-689, tests/common/wire.rs:180-182, tests/sanity.rs:84, tests/sanity.rs:92)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn Protocol tests/common` matches only wire.rs:14; `grep -n readout_multiset tests/common/sim.rs` matches only the import; `ByteMeter.read` has no accessor; `git show 368da2a5 -- tests/common/wire.rs` removes every `Protocol::V2` use; `git show 0d48b153 -- tests/common/fault.rs` removes `pub fn read`; `git blame -L687,689 tests/common/sim.rs` dates the braces to 9eadfc68 and the line inside to 212c6914; `.observations()` is called only at tests/sanity.rs:84,92)
- Seen by: all three lenses; refutation: confirmed, and added the `readout_multiset` import; history: deliberate-but-expired for `Protocol` (V1 retirement), `ByteMeter.read` (accessor ruled dead), and the braces (the scoped batch guard is gone); no rationale for `let _ = self.label;` and `observations()`
- Owner-gated: no

The allow's stated reason ("Not every binary uses every module") justifies `dead_code`: an item one binary does not reach. It does not justify `unused_imports`: a `use` unused inside a module is unused in every binary that includes it, so that half of the allow only hides rot. It hides two dead imports today, and the `dead_code` half hides a field whose accessor was already ruled dead, a no-op statement, an accessor that clones a `pub` field, and a scope that scopes nothing. A `WindowChoice::Default.apply(..)` on the seed in `divergent_pair` is the identity by `apply`'s definition; beside sim.rs:682-686, which sweeps the seed's window, it reads like a lost parameter.

Evidence:

        31	//! Not every binary uses every module; suppress unused-code warnings here
        32	//! rather than peppering allows across modules.
        33	#![allow(dead_code, unused_imports)]

    (tests/common/wire.rs:14)
        14	use rumors::{Peer, Protocol, Rumors, testing::run_to_quiescence};

    (tests/common/sim.rs:86)
        86	use crate::common::oracle::{readout, readout_multiset, version_key};

    (tests/common/fault.rs:87-88)
        87	    write: Budget,
        88	    read: Budget,

    (tests/common/flaky.rs:199)
       199	        let _ = self.label;

    (tests/common/peer.rs:59-61)
        59	    pub fn observations(&self) -> Vec<(Version, T)> {
        60	        self.observations.clone()
        61	    }

    (tests/common/sim.rs:687-689)
       687	    {
       688	        seed.send_all(plan.seed_messages.iter().copied()).unwrap();
       689	    }

    (tests/common/wire.rs:180-182)
       180	    let seed = WindowChoice::Default
       181	        .apply(Peer::<u64>::seed())
       182	        .into_rumors();

Resolution: narrow the allow to `dead_code`; if rustc then warns on the `pub use` re-exports in schedule/mod.rs:19-26 or gossip_snapshot.rs:94, allow `unused_imports` on those items only. Delete `Protocol` and `readout_multiset` from the imports; drop `ByteMeter.read` (the field, and its clone into the meter at fault.rs:107) and make the doc at 78-80 singular; delete `let _ = self.label;`; delete `Peer::observations()` and read the field at tests/sanity.rs:84,92; unwrap the braces; write `Peer::<u64>::seed().into_rumors()` in `divergent_pair`, or make the seed's window a parameter. Acceptance: `#![allow(dead_code)]` alone stands at mod.rs:33 with `just clippy` clean; none of the seven items remains.

### tests-common-11: overlap.rs motivates its harness by an incident and by a data structure the tree no longer has
- Where: tests/common/overlap.rs:9-11 (related: tests/common/overlap.rs:298-302, tests/common/overlap.rs:321-323, tests/common/overlap.rs:414-416, tests/common/overlap.rs:425-427; outside the partition tests/session_overlap.rs:13, 73, 148)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show -s 0b353ffc` names "the imbl OrdMap::diff defect"; `git show -s 8f87ddd0`, nineteen minutes later, removes imbl; `grep -c 'name = "imbl"' Cargo.lock` is 0; `FAN_INLINE = 2` at src/tree/typed/untyped/fan.rs:44; `grep -rn chunk src/tree` finds no fan chunking)
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (the "16-entry chunk" is imbl's `OrdMap` node; the dependency left the tree the day the comment was written)
- Owner-gated: no

The module doc and three comments motivate the harness by the story of a past defect ("where a real defect lived", "the discovering incident", "the one that found a real bug", "the known defect reproduces") rather than by the failure class it detects, and the preamble-size comment anchors its `12..=48` range to "the root fan to cross one 16-entry chunk". Today's radix fan has no 16-entry chunking and no chunk concept; the 16 was a node size of a dependency the tree no longer has. A maintainer cannot re-derive the range from what is, and the AGENTS.md hard rule (nothing refers to code that no longer exists) is what the parenthetical breaches. "Silently lost" names no mechanism.

Evidence:

         9	//! sessions) in between. That gap is where a real defect lived — its
        10	//! downstream symptom was an innocent leaf silently lost under exactly
        11	//! such an overlap — so overlap is a first-class, deterministically
    ...
       321	            // Preamble size: enough base content to span multiple
       322	            // radix-fan chunks (the discovering defect needed the root
       323	            // fan to cross one 16-entry chunk), sometimes much more.
    ...
       414	/// One deliberate overlap pincer, spliced into the generated soup: the
       415	/// motif distilled from the discovering incident, as choices the shadow
       416	/// processes like any others.

Resolution: restate the motivation forward: the failure class is an install that re-joins a session's fork-time state and must not drop a leaf a concurrent install added, and the pincer is the minimal schedule that puts a mutation between one session's fork and its install. Re-derive the preamble range from today's tree (for example: enough leaves that the root fan has several children, so an install touches a branch node) or drop the parenthetical. Apply the same treatment at 425-427 and at tests/session_overlap.rs:13, 73, 148. Acceptance: no sentence in overlap.rs refers to a past defect, incident, or the number 16; the preamble comment names a property of today's tree or names none.

### tests-common-12: the overlap shadow models the fork at `Open`; the live session forks at its first poll; the doc states the model as fact and the guard's skips are unmeasured
- Where: tests/common/overlap.rs:17-25 (related: tests/common/overlap.rs:92-117, tests/common/overlap.rs:178-179, tests/common/overlap.rs:233-241, tests/common/overlap.rs:562-564, tests/common/overlap.rs:636, src/rumors.rs:489, tests/session_overlap.rs:154-178)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read: `open` at 93-117 builds two futures without polling them; `Rumors::gossip` at src/rumors.rs:489 is `pub async fn`, so nothing runs until the first poll; the shadow snapshots at `Open` at line 636; the divergence case below is constructed by reasoning, not run)
- Seen by: blind-spots (guard hides shadow drift), api-economics (shadow unverified); refutation: reframed (the guard is load-bearing for a by-design imprecision; the gap is the unmeasured skip count); history: deliberate-and-holds for the guard's stated purpose
- Owner-gated: no

The module doc says `Open` "captures both endpoints' fork-time state" and the event doc says `Open` forks "both sides' working state here". The live session forks at the first poll that reaches its fork, after any events between `Open` and the first `Step`. Consider an endpoint that redacts, in that gap, a message its counterparty never held: the shadow's `Open`-time snapshot has the message live, so `merge_session` credits the counterparty with it at `Close`, and a later `RedactObservation` there emits a `Redact` the live peer never observed. The guard at 237-241 is therefore necessary, and the "shadow imprecision" its comment names is by design. What is missing is a measurement: nothing counts how many emitted `Redact`s the guard drops, so redaction coverage under overlap has no liveness floor, and the docs describe the model as if it were the protocol. The earlier proposal to replace the guard with an assertion would fail valid schedules.

Evidence:

        17	//! The alphabet extends [`schedule::events::Event`] with three session
        18	//! events over a small set of *slots*: [`OverlapEvent::Open`] captures
        19	//! both endpoints' fork-time state and parks, [`OverlapEvent::Step`]
    ...
       233	                // The generator's shadow makes this always-observed; the
       234	                // guard mirrors the serial executor's, so a shadow
       235	                // imprecision degrades to a skipped event on both sides
       236	                // of the comparison rather than an invalid `redact`.
       237	                let observed = peers[*peer].observations.iter().any(|(v, _)| v == version);
       238	                if observed {
    ...
       636	                open.insert(slot, (a, b, sim.clone()));

Resolution: (1) restate at 17-25, 178-179, and 562-564 that the shadow models the fork at `Open` while the live session forks at its first poll, and that the guard exists because the two differ; replace the guard comment's borrowed justification (there is no gossip filter here) with that one. (2) Return the skipped-`Redact` count from `execute_overlap_and_quiesce` and pin a ceiling in tests/session_overlap.rs (a fraction of emitted `Redact`s over the run), with a committed demonstration that a broken `merge_session` (skipping the `Close` merge) exceeds it. Acceptance: the docs describe the model as a model; the suite fails when the skip ceiling is exceeded and stays green at HEAD.

Construction: a hand-built `OverlapSchedule` with three peers after a converged preamble: `Insert { peer: a, value }` (event k), `Open { slot: 0, a, b }`, `Redact { peer: a, target_event_idx: k }`, `Close { slot: 0 }`, `Redact { peer: b, target_event_idx: k }`. The shadow emits the final `Redact` (its `Close` merge credited b with k); the live b never observes k (a's fork at first poll has k dead), so the executor's guard skips it. A skip counter reports 1; today nothing reports anything.

### tests-common-13: `Step { polls }` and `SESSION_POLL_BOUND`'s panic count alternation rounds, not polls
- Where: tests/common/overlap.rs:181-182 (related: tests/common/overlap.rs:59-60, tests/common/overlap.rs:120-127, tests/common/overlap.rs:153-159, tests/common/overlap.rs:494-497)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `Session::step`: `n` rounds of one poll per unfinished side, so up to `2n` polls)
- Seen by: api-economics; refutation: confirmed; history: no rationale (both docs unchanged since 0b353ffc)
- Owner-gated: no

The event doc says the session is polled "at most `polls` times"; `step(n)` runs `n` alternation rounds of one poll per unfinished side. `SESSION_POLL_BOUND` is documented as "Alternation rounds" but `finish`'s panic message reports it as "polls". A counterexample reading `Step { polls: 3 }` should mean what the executor does.

Evidence:

       181	    /// Poll the session in `slot` at most `polls` times.
       182	    Step { slot: usize, polls: usize },

       120	    /// Drive at most `n` alternation rounds — one poll of each unfinished
       121	    /// side per round — returning `true` once both sides have completed.

       156	            "overlapped session did not complete within {SESSION_POLL_BOUND} polls: \
       157	             a protocol deadlock"

Resolution: rename the field to `rounds` (and `Pincer.park`'s doc accordingly) or reword the doc to "at most `polls` alternation rounds (one poll of each unfinished side per round)"; say "rounds" in the panic message. Acceptance: the field doc, `Session::step`'s doc, `SESSION_POLL_BOUND`'s doc, and the panic message use one unit.

### tests-common-14: overlap.rs re-implements `schedule::arb`'s shadow and its `fork_tree`
- Where: tests/common/overlap.rs:350-356 (related: tests/common/overlap.rs:504-560, tests/common/schedule/arb.rs:161-165, tests/common/schedule/arb.rs:255-363)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'fn fork_tree' tests/` returns the two identical three-line bodies; read `Knowledge::merge_session` against `SimState::absorb` and traced the four known/live combinations per peer: `merge_session(&self.clone(), a, b)` and `absorb(a, b); absorb(b, a)` produce the same sets and the same per-peer observation order)
- Seen by: structure-prose (`fork_tree`), api-economics (`Knowledge`); refutation: confirmed the unification, and that an equality meta-test is not attainable (see finding 12); history: no rationale (`Knowledge` landed with the doc "as in `schedule::arb`'s shadow" and no shared code)
- Owner-gated: no

Both copies announce themselves as copies. `fork_tree` exists twice because arb.rs's is private; `Knowledge` re-encodes the deletion-honoring merge that `SimState::absorb` encodes, differing only in taking a fork-time snapshot as the source. Two hand-kept encodings of one semantics stay in agreement only by reading.

Evidence:

       350	/// Fold raw entropy into a valid fork tree (as
       351	/// [`schedule::arb`](super::schedule::arb) does).
       352	fn fork_tree(n_peers: usize, raw: &[usize]) -> Vec<usize> {
    ...
       504	/// Per-peer knowledge sets, as in `schedule::arb`'s shadow: everything
       505	/// the peer has ever held, the subset currently live, and the exact
       506	/// observation order.
       507	#[derive(Clone)]
       508	struct Knowledge {

Resolution: minimum: make `arb::fork_tree` `pub` and import it. Fuller: give `SimState` a snapshot-parameterized `merge_session(&mut self, snapshot: &SimState, a, b)`, express `gossip(a, b)` as `let frozen = self.clone(); self.merge_session(&frozen, a, b)`, build `build_overlap_schedule` on `SimState`, and delete `Knowledge`. Do not add an equality meta-test for the overlap shadow; state at the site that it is approximate by design (finding 12). Acceptance: one `fn fork_tree` and one shadow type in tests/common.

### tests-common-15: `common::peer::Peer` shadows `rumors::Peer`, and two unrelated `pub Session` types share a name
- Where: tests/common/peer.rs:22-23 (related: tests/common/overlap.rs:87-90, tests/common/sim.rs:152-158, tests/common/schedule/executor.rs:187, tests/common/overlap.rs:209, tests/sanity.rs:81, tests/redaction.rs:99)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (`grep -rn 'rumors::Peer::<\|rumors::Peer::seed' tests/`: 17 qualified sites in 6 files; the qualification is forced in sanity.rs, redaction.rs, executor.rs:187, and overlap.rs:209, which import the test `Peer`, and unforced in pairwise.rs and session_overlap.rs, which do not)
- Seen by: api-economics; refutation: reframed (8 forced sites, 9 unforced); history: the collision is accidental (`rumors::Peer` arrived at cb69fc95 after the test type was named)
- Owner-gated: no

Naming the simulated observing peer `Peer` forces `rumors::Peer::seed()` wherever a suite also seeds a universe, including twice inside the harness itself; `overlap::Session` (an in-flight hand-driven session) and `sim::Session` (a planned session record) mean unrelated things under one name in one library.

Evidence:

        22	/// One simulated peer.
        23	pub struct Peer<T> {

    (tests/common/overlap.rs:87; tests/common/sim.rs:153)
        87	pub struct Session {
       153	pub struct Session {

Resolution: rename the test type (`Observed<T>` says what it is: a `Rumors<T>` plus an observation log) and `sim::Session` (`PlannedSession`); import `rumors::Peer` plainly. The nine unforced qualifications in pairwise.rs and session_overlap.rs can go now with a `use rumors::Peer`. Acceptance: `rumors::Peer::` appears only where it disambiguates something real.

### tests-common-16: `Peer::drain` re-derives `UnorderedMessages`, so generated multi-peer schedules never drive the public observer
- Where: tests/common/peer.rs:63-75 (related: tests/common/peer.rs:6-14, src/rumors/unordered.rs:102-108, src/rumors/unordered.rs:217-219, src/rumors.rs:372)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (mechanism match verified by reading: unordered.rs:105 opens a pass with `range_owned(causally::since(checkpoint))` and 219 absorbs `checkpoint |= &ceiling`, the two steps `drain` performs; the swap's feasibility is assessed: `unordered_messages_since` exists and the stream holds a watch receiver, so `try_into_peer` is not blocked)
- Seen by: blind-spots; refutation: reframed (`Snapshot::range` is public API, so no internal-entry rule is breached; the gap is coverage); history: deliberate-and-holds for pull-based draining, silent on why the observer is not consumed
- Owner-gated: no

The module doc says the drain matches "the `UnorderedMessages` delivery contract", and it does, by re-implementing the observer's two steps rather than consuming the observer. The consequence is coverage: schedules of two to eight peers with bootstraps and retirements, the richest generated inputs in the suite, never run the public `UnorderedMessages` stream; it is exercised only by tests/listen.rs, tests/causal.rs, tests/api_send_bounds.rs, and sim.rs's concurrent `run_observers`. An observer-backed drain would pin the stream over every generated schedule and let the shadow meta-test cover it for free.

Evidence:

        65	    pub fn drain(&mut self) -> usize {
        66	        let snapshot = self.local.snapshot();
        67	        let mut new = 0;
        68	        for (version, message) in snapshot.range(causally::since(&self.checkpoint)) {
        69	            self.observations
        70	                .push((version.clone(), (*message).clone()));
        71	            new += 1;
        72	        }
        73	        self.checkpoint |= snapshot.latest();
        74	        new
        75	    }

    (src/rumors/unordered.rs:105, 219)
                    walk: inner.tree.range_owned(causally::since(checkpoint.clone())),
                        this.checkpoint |= &ceiling;

Resolution: hold an `UnorderedMessages<T>` in the test `Peer`, created in `new` via `local.unordered_messages_since(latest)`, and implement `drain` by polling it with `now_or_never` until quiet (the pattern tests/listen.rs and tests/causal.rs already use); delete the `checkpoint` field and the range re-derivation. Acceptance: peer.rs no longer calls `snapshot.range(causally::since(..))`; shadow_validity.rs, multi_peer.rs, membership.rs, partition.rs, and sanity.rs pass unchanged; deliberately skipping the observer's ceiling absorption fails them.

Construction: none is needed to show a defect (this is a coverage gap); the demonstration is the acceptance above.

### tests-common-17: duplicate and implied trait bounds
- Where: tests/common/peer.rs:115 (related: tests/common/peer.rs:125, tests/common/peer.rs:142, tests/common/schedule/executor.rs:68, tests/common/schedule/executor.rs:81, tests/common/schedule/executor.rs:115, tests/common/schedule/executor.rs:144, tests/common/schedule/executor.rs:156, tests/common/schedule/executor.rs:181, tests/common/overlap.rs:204)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read each cited line)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: the doubled `Eq` is a5a16f43's mechanical sweep
- Owner-gated: no

`quiesce`, `quiesce_slots`, and `quiesce_refs` list `Eq` twice; executor.rs alternates two bound orders across adjacent entry points, and lists `Eq` beside `Ord` (which implies it) throughout. A where-clause is read as a contract; a duplicated or implied bound makes the reader check whether it means something.

Evidence:

       115	    T: Clone + Eq + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,

Resolution: drop the second `Eq` at the three peer.rs sites; drop `Eq` where `Ord` is present; adopt one bound order across executor.rs, or define one trait alias (`pub trait Payload: Clone + Ord + Serialize + DeserializeOwned + Send + Sync + 'static {}` with a blanket impl) and use it across peer.rs, executor.rs, overlap.rs, wire.rs, action.rs. Acceptance: no bound list in tests/common repeats a trait or lists a supertrait beside its subtrait.

### tests-common-18: the fingerprint tuple appears twelve times and the quiesce loop three times, with the round bound declared twice
- Where: tests/common/peer.rs:140-181 (related: tests/common/sim.rs:106-107, tests/common/sim.rs:881-906, tests/common/sim.rs:926-929, tests/pairwise.rs:42-45, tests/retire.rs:379, tests/retire.rs:389, tests/retire.rs:421, tests/retire.rs:431, tests/bookmark_causality.rs:814, tests/bookmark_causality.rs:829, tests/multi_peer.rs:144, tests/session_overlap.rs:43-58)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'snapshot.hash(), snapshot.latest().clone()' tests/ src/`: 12 hits, all under tests/; `MAX_QUIESCE_ROUNDS_PER_PEER = 16` at peer.rs:181 and sim.rs:107)
- Seen by: structure-prose, api-economics; refutation: confirmed and enlarged (the api-economics citation of tests/lifecycle.rs:50-58 is a fixture builder, not a loop); history: the 0d48b153 dedup round unified peer.rs's two loops and left sim.rs's copy "the same criterion" without saying why
- Owner-gated: no

The convergence criterion `(snapshot.hash(), snapshot.latest().clone())` is the one fact every convergence assertion in the suite rests on, and it is spelled out in twelve places; the bounded full-mesh fixed-point loop around it exists in peer.rs (sync, over `Peer<T>`), sim.rs (async, over `Rumors<u64>`, its doc acknowledging the transcription), and tests/session_overlap.rs (over readouts, with its own `ROUNDS = 8`); the round bound is a hand-maintained duplicate constant.

Evidence:

       149	    let fingerprint = |peer: &Peer<T>| {
       150	        let snapshot = peer.local.snapshot();
       151	        (snapshot.hash(), snapshot.latest().clone())
       152	    };
    ...
       181	const MAX_QUIESCE_ROUNDS_PER_PEER: usize = 16;

    (tests/common/sim.rs:106-107)
       106	/// Headroom on the heal loop, as in `peer::quiesce`.
       107	const MAX_QUIESCE_ROUNDS_PER_PEER: usize = 16;

Resolution: add `pub fn fingerprint<T>(snapshot: &Snapshot<T>) -> ([u8; MERKLE_HASH_LEN], Version)` beside `readout` in oracle.rs and use it at every site; write one async core `quiesce_handles<T>(peers: &[&Rumors<T>])` (fingerprint fixed point, one bound, one panic message) that `peer::quiesce_refs` wraps with `block_on` plus per-peer drains, `sim::quiesce` awaits, and session_overlap.rs calls in place of `converge`; make the bound `pub` in one place. Acceptance: one definition of the fingerprint and one of the round bound in tests/; pairwise.rs and session_overlap.rs import rather than redefine.

### tests-common-19: socket-clamp dial and bind code duplicated between tcp.rs and routed_tcp.rs
- Where: tests/common/routed_tcp.rs:34-43 (related: tests/common/routed_tcp.rs:99-111, tests/common/tcp.rs:66-76, tests/common/tcp.rs:112-119)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the four blocks: the same send-buffer-clamped dial and the same recv-buffer-clamped loopback bind, differing only in the backlog constant and where the inherit-before-listen comment sits)
- Seen by: structure-prose; refutation: confirmed (the two files implement different traits, so only the socket-option blocks are shareable); history: no rationale (routed_tcp.rs landed twelve days after tcp.rs)
- Owner-gated: no

Two copies of socket policy invite divergence: one already documents the inherit-before-listen ordering inline and the other in a doc comment. One helper pair keeps the OS-clamp knowledge in one place.

Evidence:

        34	    async fn dial(&self, addr: &SocketAddr) -> io::Result<TcpStream> {
        35	        match self.send_buffer {
        36	            None => TcpStream::connect(*addr).await,
        37	            Some(send) => {
        38	                let socket = TcpSocket::new_v4()?;
        39	                socket.set_send_buffer_size(send)?;
        40	                socket.connect(*addr).await
        41	            }
        42	        }
        43	    }

    (tests/common/tcp.rs:112-119)
       112	        let stream = match self.0.send_buffer {
       113	            None => TcpStream::connect(self.0.peer).await?,
       114	            Some(send) => {
       115	                let socket = TcpSocket::new_v4()?;
       116	                socket.set_send_buffer_size(send)?;
       117	                socket.connect(self.0.peer).await?
       118	            }
       119	        };

Resolution: add `pub async fn dial_clamped(addr: SocketAddr, send_buffer: Option<u32>) -> io::Result<TcpStream>` and `pub async fn bind_loopback(recv_buffer: Option<u32>, backlog: u32) -> io::Result<TcpListener>` in tcp.rs and call them from both files. Acceptance: each socket-clamp `match` appears once in tests/common.

### tests-common-20: the shadow docs claim an exact observation sequence; only the set is pinned, and the live order is tree order
- Where: tests/common/schedule/arb.rs:248-250 (related: tests/common/schedule/arb.rs:58-60, tests/common/schedule/arb.rs:355-360, tests/common/peer.rs:33-35, tests/common/peer.rs:68, src/snapshot.rs:129-130, tests/shadow_validity.rs:25-26)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (src/snapshot.rs:129 documents `range` as "order is unspecified"; `Peer::drain` records in that order; the shadow's `absorb` pushes novel indices in sorted `EventIdx` order; tests/shadow_validity.rs:25-26 compares sets for exactly this reason)
- Seen by: blind-spots; refutation: confirmed; history: no rationale (the only recorded reason the shadow's order matters is seed stability of its own redact-target selection)
- Owner-gated: no

`SimState`'s doc says `observed_log` is "the exact sequence of `EventIdx`s that the live `Peer<T>` would have appended", and the gossip comment says per-peer observation order is what the live peer produces. Neither is guaranteed: the live log is in `Snapshot::range` order, which the crate documents as unspecified, and the meta-test compares sets because of that. The order is the shadow's own, used to draw redact targets deterministically. A model doc must be accurate; a reader who relied on the sequence claim (to add a sequence-wise comparison, say) would misdiagnose the failure.

Evidence:

       248	/// * `observed_log[p]` is the exact sequence of `EventIdx`s that the
       249	///   live `Peer<T>` would have appended to its observation vector by
       250	///   this point — driven by both local inserts and gossip events.

    (tests/shadow_validity.rs:25-26)
    //! Comparison is set-wise: callback order within a batch is
    //! unspecified, so a sequence-wise comparison would over-constrain.

Resolution: restate 248-250, 58-60, and 355-360: `observed_log[p]` is the set of `EventIdx`s the live peer has observed, kept in the shadow's own sorted-per-pass order so redact targets are drawn deterministically; the live peer's order is the tree's and is not compared. Acceptance: no passage in arb.rs claims sequence agreement with the live peer.

### tests-common-21: `EventIdx` is a synonym for the same `usize` that indexes peers
- Where: tests/common/schedule/events.rs:3-6 (related: tests/common/schedule/events.rs:20-23, tests/common/schedule/executor.rs:29, tests/common/schedule/executor.rs:42, tests/common/overlap.rs:171-174)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read)
- Seen by: structure-prose; refutation: confirmed as a fair nit; history: dates to the original suite, never revisited
- Owner-gated: no

Events carry a peer index and an event index side by side as `usize` (`Redact { peer: usize, target_event_idx: EventIdx }`), maps are keyed by each, and both shadows index vectors by both. A synonym gives none of the protection a newtype would against a `slots[target_event_idx]`-style swap. The cost of the change is conversions at `events.len()` and `enumerate()` sites and a cosmetic change to the Debug rendering inside committed shrink notes (replay is unaffected: cc hashes are RNG seeds).

Evidence:

         3	/// Index of an event in a `Schedule`'s flat `events` vector. Used as
         4	/// a stable cross-reference between the oracle, the schedule
         5	/// executor, and the shadow simulator.
         6	pub type EventIdx = usize;

Resolution: `#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)] pub struct EventIdx(pub usize);` (and, if wanted, `PeerIdx`), with `Display` so counterexamples still read. Acceptance: `EventIdx` is a struct and the executor and shadows compile without index-space casts.

### tests-common-22: cut offsets are only ever sampled; no suite sweeps every offset of a hand-off session
- Where: tests/common/sim.rs:282-293 (related: tests/common/sim.rs:104, tests/common/fault.rs:99-125, tests/disruption.rs:432, tests/session_overlap.rs:104-128)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`grep -rn 'for cut in\|for offset in\|write_cut: Some(\|read_cut: Some(' tests/*.rs`: no session-level offset loop; the only fixed offsets are point regressions at tests/bookmark_causality.rs:1305,1310 and tests/gossip_when.rs:714,718,950)
- Seen by: blind-spots; refutation: severity down to low (codec-level truncation is swept totally in src; what is missing is the session-level algebra); history: no rationale (MAX_CUT's inline argument is about sampling density, not totality)
- Owner-gated: no

The only source of session-level cut offsets is `arb_fault`, uniform over `0..MAX_CUT`. The party hand-off frame and the epilogue marker are narrow windows in a session of that length, so whether a run lands a cut inside them at a given role and direction is sampling. tests/session_overlap.rs already shows the total-sweep pattern for poll prefixes; `fault::metered` gives the exact extent to sweep to; the sweep is a few thousand in-memory runs. Construct, do not argue.

Evidence:

       286	    let cut = prop_oneof![2 => Just(None), 3 => (0..MAX_CUT).prop_map(Some)];

Resolution: add a deterministic sweep (tests/disruption.rs or a sibling): build the seed-plus-fork fixture, meter one clean bootstrap and one clean retirement, then for each role, each direction, and each offset `0..=measured` run the session with that single cut and assert the classifier plus party conservation (the newcomer holds the fork xor the server rejoined it, allowing the documented post-take leak) and, for retirement, the `Retired`/`Recovered`/`Uncertain` algebra against the absorber's outcome. Pair with finding 25 so the donor and absorber sides are in the sweep. Acceptance: a committed test iterates every offset up to the metered extent for both directions of both hand-offs, its doc states the invariant, and it stays inside the nextest slow-test budget (60 s period, terminate after 3).

Construction: as the resolution describes; the metered extent from `fault::metered` on a clean run of each hand-off bounds the loop.

### tests-common-23: "honest" already names the model of record; here it names a different predicate
- Where: tests/common/sim.rs:359-385 (related: tests/common/sim.rs:188, tests/common/sim.rs:390-399, tests/common/sim.rs:412, tests/common/sim.rs:422-423, tests/common/sim.rs:453, tests/common/sim.rs:663, tests/common/sim.rs:942, tests/common/wire.rs:25, tests/seed_liveness.rs:347; callers in tests/disruption.rs)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rniw 'honest\|dishonest\|honesty'` over the partition lists the sites; the word appears in 39 files under src/ and six other test binaries)
- Seen by: structure-prose; refutation: confirmed, and noted the register is crate-wide; history: no rationale (dates to the engine's first commit)
- Owner-gated: no, but the vocabulary should be ruled crate-wide

AGENTS.md names the model of record "authenticated-honest-peer": there, honest means non-adversarial, and the word is a term of art the crate uses widely in that sense. sim.rs's `is_honest_error` decides something else: whether an error is attributable to the injected cut (a truncation or a severed-transport kind) rather than a decode or protocol failure. The same word carries a second meaning in the one place the first meaning matters most (the engine that must stay on-model). The writing rule says to name the property; here the property is "attributable to the cut". Two further sites use the word for unrelated things: wire.rs:25 ("keeps `-D warnings` honest") and seed_liveness.rs:347 ("the honest seed stays").

Evidence:

       359	// ---- honesty of failures ---------------------------------------------------
    ...
       361	/// Assert `e` is an injected I/O fault that *truncated* a frame: the only
       362	/// error an honest, single-universe simulation can surface.
    ...
       385	pub fn is_honest_error(error: &Error) -> bool {

Resolution: rename to the mechanism (`is_injected_cut`, `assert_injected_cut`, `cut_io`, `cut_remote`, `assert_session_cut_or_ok`) and write "attributable to the cut" / "not attributable to a cut (a decode or protocol failure)" in prose; update the callers in tests/disruption.rs. Rule the vocabulary once for the crate (src/peer/gossip/tests.rs and six binaries share it) rather than renaming here alone. Rewrite wire.rs:25 ("keeps `-D warnings` clean") and seed_liveness.rs:347 ("the `.txt` seed stays"). Acceptance: no identifier in tests/common uses honest/dishonest/honesty; prose uses the word only in the model-of-record sense.

### tests-common-24: em-dashes in `//` comments
- Where: tests/common/sim.rs:390 (related: tests/common/sim.rs:759, tests/common/flaky.rs:212; repo-wide, 116 such comments under src/ and 43 under tests/)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn '^\s*//[^/!].*—'` over the partition returns exactly the three sites; the same grep over src/ and tests/ gives the repo-wide counts)
- Seen by: structure-prose; refutation: confirmed; history: no ruling permits em-dashes in `//` comments; the Jul 24 style sweep targeted rendered prose
- Owner-gated: no, but the convention should be ruled repo-wide

The house rule is colons or spaced double-hyphens in `//` comments, with true em-dashes reserved for rendered prose. The three partition sites are instances of a repo-wide pattern; fixing them alone would make the partition inconsistent with the rest of the tree.

Evidence:

       390	        // that it lied. A malformed preamble stays dishonest — a cut

    (tests/common/sim.rs:759; tests/common/flaky.rs:212)
       759	    // move only now — over wires that may still drop mid-hand-off.
       212	        // exactly as it was — the atomicity the crate's recovery relies on.

Resolution: rule the convention once and sweep the repo mechanically (the grep above is the sweep); replace with a colon or `--` at the three sites as part of it. Acceptance: the grep returns nothing under src/ and tests/.

### tests-common-25: fault plans never fault the donor side of a bootstrap or the absorber side of a retirement
- Where: tests/common/sim.rs:490-519 (related: tests/common/sim.rs:118-122, tests/common/sim.rs:160-167, tests/common/sim.rs:330-333, tests/common/sim.rs:787-800, src/peer/gossip.rs:782-793, src/link.rs:532, tests/disruption.rs:909-923)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n 'FaultPlan::NONE' tests/common/sim.rs`: the serving side at 503 and the absorber at 798; `MEMORY_STREAM_CAPACITY = 8 * 1024` at src/link.rs:532; the donor's `party::send` error branch at src/peer/gossip.rs:782-793; a corpus grep of every `wrap_link`, `faulty_link`, and `fault::faulty` call under tests/ finds no donor- or absorber-side plan anywhere)
- Seen by: blind-spots; refutation: confirmed; history: deliberate-and-holds for the stated asymmetry (the joiner's plan "covers both observable directions"), which makes no claim about the donor's or absorber's own budget expiry
- Owner-gated: no

`run_boot` wraps the serving endpoint with `FaultPlan::NONE` and `run_plan` wraps the absorber the same way, so in every plan the donor of a party and the receiver of a whole party observe only their counterparty's death. The doc argues the joiner's read cut "models the server's frames dying in flight", but the joiner's read cut leaves the server's writes succeeding into the 8 KiB memory buffer; the donor's `party::send` error branch ("A lost fork merely leaks its region") is reached only when scheduling happens to drop the joiner before the server writes, never at a chosen offset and never mid-frame. `Session` carries `fault_a` and `fault_b` for gossip; the asymmetric sessions are the one place the endpoint-by-direction matrix is half empty. The donor and the receiver of a party run different code with different recovery semantics (guard snap-back before `party.take()`, leak after); a shape that wedges one side can look benign on its dual.

Evidence:

       491	/// The serving side stays clean; the joiner's fault plan covers both
       492	/// observable directions of a duplex (its read cut models the server's
       493	/// frames dying in flight). A joiner that fails may or may not have cost
       494	/// the server its donated fork, so it conservatively counts as a possible
       495	/// loss either way.
    ...
       503	        let mut link = fault::faulty(serve_side, FaultPlan::NONE);

       798	                let mut link = fault::faulty(absorber_side, FaultPlan::NONE);

    (src/peer/gossip.rs:782-787)
                match party::send(donated, write, &observe).await {
                    Err(e) => {
                        // A retiring donation in limbo must be assumed received:
                        // report `Intent::Retire` alongside the error so that the
                        // `Peer` is not handed back. A lost fork merely leaks its
                        // region; we remain.

Resolution: add a serving-side plan per boot entry and an `absorber: FaultPlan` to `RetireOp`, mirroring `Session`'s two plans. Draw the new fields after `windows`: sim.rs:330-333 states that draw order is the seed-compatibility surface, and appending keeps every committed disruption seed regenerating its existing prefix. Accounting: a donor whose session errors after `party::send` began is a possible loss (the newcomer may or may not hold the fork); one that errors before the donation is not (the guard rejoins). The absorber-side arms already exist (`absorbed.is_err()`), and finding 25 makes the `Retired`-with-absorber-`Err` arm reachable. Re-run `max_cut_spans_the_envelope_session`. Acceptance: a committed case in tests/disruption.rs runs a bootstrap whose server-side write budget expires inside the party frame and a retirement whose absorber-side read budget expires inside the party frame, both passing `assert_party_invariants` and the classifier; `grep -n 'FaultPlan::NONE' tests/common/sim.rs` no longer matches the serve and absorber wrap sites.

Construction: in tests/disruption.rs, build the seed-plus-fork fixture; meter one clean bootstrap with `fault::metered` to learn the server's write extent; run `run_boot` with a server plan `write_cut: Some(k)` for each `k` up to that extent (or the chosen `k` inside the party frame) and assert: the classifier accepts the server's error; afterward either the newcomer holds a party disjoint from the server's or the server's party equals its pre-donation party (fold-join), with the leak permitted only for errors after `party.take()`. Do the same for a retirement with an absorber `read_cut` swept over the absorber's read extent.

### tests-common-26: qualified paths where an import is the idiom
- Where: tests/common/sim.rs:692-696 (related: tests/common/sim.rs:88, tests/common/sim.rs:412-418, tests/common/sim.rs:467, tests/common/sim.rs:501, tests/common/sim.rs:790, tests/common/sim.rs:895, tests/common/action.rs:67, tests/common/window.rs:36, tests/common/wire.rs:147, tests/common/wire.rs:151, tests/common/wire.rs:248, tests/common/schedule/executor.rs:258, tests/common/gossip_snapshot.rs:213, tests/common/gossip_snapshot.rs:439, tests/common/overlap.rs:509-510, tests/common/overlap.rs:535, tests/common/flaky.rs:196, tests/common/flaky.rs:203, tests/common/peer.rs:156)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read each cited line)
- Seen by: structure-prose; refutation: confirmed; history: no rationale
- Owner-gated: no

sim.rs spells `crate::common::wire::bootstrap_fork_with_window_async` while importing `wire_gossip_async` from the same module at line 88; `rumors::link::memory()` and `memory_with_capacity` are spelled inline in four files; `rumors::link::MemoryLink` beside imported `MemoryAcceptor, MemoryConnector`; `rumors::Rumors`, `rumors::Peer`, `rumors::Gossiped` twice each in one signature; `std::collections::BTreeSet` three times while `BTreeMap` is imported; `std::io::Cursor`, `std::io::Error`, `std::io::ErrorKind` where `io::` is the convention; and two closure-result annotations `([u8; rumors::MERKLE_HASH_LEN], Version)` on inferable types. (executor.rs:187 and overlap.rs:209 qualify `rumors::Peer` to disambiguate from the local `Peer`; those are correct until finding 15.)

Evidence:

       692	        let child = crate::common::wire::bootstrap_fork_with_window_async(
       693	            &fleet[0],
       694	            plan.windows.choice(i),
       695	        )
       696	        .await;

        88	use crate::common::wire::wire_gossip_async;

Resolution: import at the top of each file and drop the two redundant annotations. Acceptance: no `rumors::link::memory`, `std::collections::`, or `std::io::Error` spelled inline in tests/common where an import exists or fits.

### tests-common-27: default-dialect tells: "genuine(ly)", loose "sound", "conviction", and a "silent" divergence inside an assertion
- Where: tests/common/sim.rs:1007-1011 (related: tests/common/mod.rs:19, tests/common/sim.rs:11, tests/common/sim.rs:31, tests/common/sim.rs:203, tests/common/sim.rs:531, tests/common/sim.rs:567, tests/common/sim.rs:633, tests/common/wire.rs:189, tests/common/schedule/executor.rs:167, tests/common/overlap.rs:55, tests/common/overlap.rs:62, tests/seed_liveness.rs:326)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep and read of each site)
- Seen by: structure-prose; refutation: reframed (drop "seam": it is established crate vocabulary, 48 uses under src/); history: no rationale
- Owner-gated: no

Word choices the writing rules flag: "genuine(ly)" as an intensifier where the property meant is party-disjoint or multi-threaded; "sound" meaning justified or complete ("Sound by [`Peer::retire`]'s contract", "the log stays sound", "Sound mid-flight"); "conviction" for what the code calls a `verdict`; and an assert message that calls the divergence "silent" when the assertion is exactly what makes it loud, naming no mechanism.

Evidence:

      1007	        assert_eq!(
      1008	            actual, expected,
      1009	            "silent divergence from the value ledger: survivor {i}'s \
      1010	             converged multiset differs from inserts minus redactions"
      1011	        );

       203	    /// Sound by [`Peer::retire`]'s contract — a retirement session

Resolution: delete "genuine(ly)" or write the property; "Sound by" to "Justified by", "stays sound" to "stays complete", "Sound mid-flight" to "Valid mid-flight because"; "conviction" to "verdict"; drop "silent divergence from" in favor of "survivor {i}'s converged multiset differs from the value ledger". Acceptance: the listed words no longer appear at the listed sites.

### tests-common-28: "the *asynchronous* gossip path" qualifies against a twin that no longer exists
- Where: tests/common/wire.rs:1-6 (related: tests/async_wire.rs:1)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S'asynchronous' -- tests/common/wire.rs tests/simulation/wire.rs` bottoms out at 691909e79 "Split wire-equivalence tests into async and sync binaries"; `ls tests | grep -i sync` returns only async_wire.rs)
- Seen by: api-economics; refutation: confirmed; history: deliberate-but-expired (the sync twin was deleted at 83edcd94 and its retirement recorded at 9d9eaac0; neither header was re-denominated)
- Owner-gated: no

The emphasized qualifier distinguished this module from a synchronous wire binary that was deleted; no sync binary and no synchronous gossip path exist, so it sends a reader looking for the other path.

Evidence:

         1	//! Wire helpers for the *asynchronous* gossip path.
         2	//!
         3	//! These drive `rumors::Rumors::gossip` over an in-memory [`rumors::link`]
         4	//! pair with both peers polled concurrently via `tokio::join!`. The two
         5	//! tasks progress directly against each other through the link's streams;
         6	//! no runtime is required unless a caller explicitly spawns a task.

Resolution: retitle ("In-memory gossip and bootstrap drivers under the closed-world poller") and drop the qualifier from tests/async_wire.rs:1 (that binary's overlap with tests/pairwise.rs is a finding for the partition owning the test binaries). Acceptance: `grep -rn asynchronous tests/` returns nothing about a gossip path.

### tests-common-29: four suites alias `tokio_block_on as block_on`, erasing the distinction wire.rs draws
- Where: tests/common/wire.rs:44-48 (related: tests/bookmark_attach.rs:17, tests/bookmark_causality.rs:71, tests/bookmark_transmit_window.rs:42, tests/gossip_when.rs:49)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (`grep -rn 'tokio_block_on as' tests/`: four files)
- Seen by: api-economics; refutation: confirmed and added tests/gossip_when.rs:49; history: the alias was a call-site-preserving rename in the WIP streaming swap (83edcd94) that was never removed
- Owner-gated: no

wire.rs names the two pollers differently so a reader knows which sessions run under the closed-world stall detector. In four suites every call reads as the deterministic poller while running a real runtime. Those suites do need the runtime (they spawn tasks); the alias hides the choice the naming exists to show.

Evidence:

        44	/// Block on `future` using this thread's reused current-thread Tokio runtime.
        45	///
        46	/// Tests should use this only when the behavior under test explicitly needs
        47	/// Tokio facilities such as task spawning, timers, or networking. Ordinary
        48	/// protocol futures should use [`block_on`].

Resolution: drop the alias and call `tokio_block_on` by name in the four suites. Acceptance: `grep -rn 'tokio_block_on as' tests/` is empty.

### tests-common-30: the bootstrap and retire driver family: a pass-through layer the V1 retirement left, and drivers four suites reimplement
- Where: tests/common/wire.rs:230-240 (related: tests/common/wire.rs:213-218, tests/common/wire.rs:244-264, tests/common/schedule/executor.rs:256-275, tests/hop_trace.rs:479-493, tests/bookmark_when.rs:217-228, tests/bootstrap.rs:31-49, tests/retire.rs:52-68, tests/retire_redaction.rs:36-43)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (`git show 368da2a5 -- tests/common/wire.rs` removes the `protocol: Protocol` parameter that gave the wrapper layer its purpose; `sed` of each suite copy: hop_trace.rs and bookmark_when.rs repeat the join-and-double-`expect` handshake and neither calls `assert_control_drained`; bootstrap.rs repeats it to surface the `Option`; retire.rs, retire_redaction.rs, and the executor's `Retire` arm each write the `try_into_peer` + `tokio::join!(retire, gossip)` driver with a slightly different contract)
- Seen by: structure-prose (pass-through; hop_trace/bookmark_when copies), api-economics (retire-into and `Option` bootstrap); refutation: confirmed (the "no `bootstrap().join(` outside common" acceptance is too broad: many of the roughly 45 such sites are the test's subject or run over non-memory links); history: deliberate-but-expired for the wrapper (it fixed `Protocol::V2` until 368da2a5), no rationale for the missing drivers
- Owner-gated: no

`bootstrap_fork_with_window_async(parent, window)` has the same signature as the private `bootstrap_fork_configured(parent, window)` and only awaits it: two names for one function since the protocol parameter left. Around it, the harness offers no driver that returns the newcomer as a `Peer` (bookmark_when.rs needs one to attach a probe), none that surfaces the mutual-bootstrap `None` (bootstrap.rs), none with a chosen capacity (hop_trace.rs), and no retire-into driver at all, so the handshake and the drain assertion are protocol knowledge copied into binaries, where two copies skip the drain and three retire copies carry three contracts.

Evidence:

       232	pub async fn bootstrap_fork_with_window_async<T>(
       233	    parent: &Rumors<T>,
       234	    window: WindowChoice,
       235	) -> Rumors<T>
       236	where
       237	    T: Serialize + DeserializeOwned + Eq + Send + Sync + Clone + 'static,
       238	{
       239	    bootstrap_fork_configured(parent, window).await
       240	}
    ...
       244	async fn bootstrap_fork_configured<T>(parent: &Rumors<T>, window: WindowChoice) -> Rumors<T>

    (tests/bookmark_when.rs:217-218)
    async fn bootstrap_fork_peer(origin: &Rumors<u64>) -> Peer<u64> {
        let (mut o_link, mut n_link) = rumors::link::memory_with_capacity(LINK_BUF);

Resolution: make `bootstrap_fork_with_window_async` the core (delete `bootstrap_fork_configured`; `bootstrap_fork_async` calls it with `WindowChoice::Floor`). Add `bootstrap_over_wire_async(parent, window) -> Option<Rumors<T>>` (control drained) with the existing helpers as `.expect("parent served the bootstrap")` wrappers, a `Peer`-returning core the `Rumors` helpers wrap, and `retire_into_async(retiree: Peer<T>, absorber: &Rumors<T>) -> Retire<T>` (absorber gossip expected `Ok`, control drained) plus a `block_on` wrapper. Migrate hop_trace.rs, bookmark_when.rs, bootstrap.rs, retire.rs, retire_redaction.rs, and the executor's `Retire` arm; sim.rs's faulted retire stays separate. Acceptance: no `bootstrap_fork_configured`; the five suites and the executor call the common drivers; a retire+gossip join outside tests/common remains only where the wire or the fault is the test's subject.

### tests-common-31: the proptest version the seed sweep transcribes is pinned in prose only
- Where: tests/seed_liveness.rs:26-29 (related: Cargo.lock:1802-1803, Cargo.toml:1)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (Cargo.lock:1802-1803 pins proptest 1.11.0; the root Cargo.toml is both the workspace and the package, so `CARGO_MANIFEST_DIR/Cargo.lock` is the lock file)
- Seen by: structure-prose; refutation: confirmed; history: deliberate-and-holds as a prose convention (390160a7 added the provenance paragraph "re-verify on upgrades"); the finding strengthens it into a check
- Owner-gated: no

The sweep's correctness rests on a transcription of proptest 1.11.0's `FileFailurePersistence::resolve`, and the doc asks the reader to re-verify when the dependency moves. Nothing fires when Cargo.lock changes; a bump that changed persistence resolution would leave this sweep passing against stale rules. Every hole found becomes a committed check, never a convention held in memory.

Evidence:

        26	//! Provenance of the transcribed rules: proptest 1.11.0,
        27	//! `FileFailurePersistence::resolve` (`failure_persistence/file.rs`).
        28	//! Re-verify the transcription when the workspace's proptest dependency
        29	//! moves to a release that touches persistence resolution.

Resolution: add `const TRANSCRIBED_PROPTEST_VERSION: &str = "1.11.0";` and a test that reads `Cargo.lock` from `CARGO_MANIFEST_DIR`, finds the `[[package]] name = "proptest"` block, and asserts its `version` equals the constant, with a message telling the bumper to re-check `FileFailurePersistence::resolve` and update the constant. Acceptance: bumping proptest in Cargo.lock fails the seed-liveness binary until the constant is updated.

### tests-common-32: the sweep judges paths only; a committed seed whose shrink note predates the strategy awaits an owner ruling
- Where: tests/seed_liveness.rs:129-134 (related: tests/seed_liveness.rs:148-203, proptest-regressions/shadow_validity.txt:9-14, tests/common/schedule/arb.rs:141-150, tests/common/sim.rs:330-333)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read the seed file; `git log -S` on the hash: added 80a3155f 2026-05-18, consolidated 2720a1ff 2026-08-13; the `fork_parents` draw entered the strategy at 1af0ab9a 2026-06-03; the in-file disposition note was added at ce3664dd 2026-09-01; arb.rs:141-150 draws `n_peers - 1` raw parents before the choices, so the same RNG seed now generates an unrelated schedule)
- Seen by: blind-spots; refutation: confirmed with date corrections; history: already-known (a pending owner ruling recorded at 390160a7, 2026-08-13, has persisted across three commits)
- Owner-gated: yes: the seed's disposition is the owner's ruling

The third `cc` line in proptest-regressions/shadow_validity.txt records a `Schedule` with no `fork_parents` and a bare observed-log tuple; the current strategy draws parents before the choices and yields `ShadowFinal`, so replaying that seed regenerates an unrelated case. The file annotates this and asks for a ruling. The doctrine says every contradiction resolves to a fix or a declared model, and a pending status that persists across commits is a process failure. The seed sweep cannot see the class: `collect_regressions` judges only that a `.txt` reverse-resolves to a live source, never whether its note still matches the strategy, so the next draw-order change (finding 25 adds one) will orphan seeds the same way, caught only by the comment convention at sim.rs:330-333.

Evidence:

       132	/// A `.txt` must reverse-resolve to a live source whose deepest anchor
       133	/// is one of `anchors`, and any other file is an orphan outright —
       134	/// proptest writes and reads only `<suffix>.txt` here.

    (proptest-regressions/shadow_validity.txt:9-14; the cc line is elided after its opening)
    # The next entry predates the current Schedule strategy shape (its shrink
    # note lacks fork_parents), so it no longer replays the failure it was
    # written for. It awaits owner disposition; do not strip it without a
    # ruling. Proptest reads only the hash before the first '#' on a cc line,
    # so this comment and the stale shrink note cost nothing at replay.
    cc 86723fa839f009875eafd473501eceafe68d5e6b00c28572965fb6fa4da8a381 # shrinks to (schedule, shadow_observed) = (Schedule { n_peers: 3, events: [...]

Resolution: owner rules on the entry: delete it (the failure it recorded is fixed, and its replay under the current strategy is a different case) or re-derive it by reintroducing the fixed defect on a branch and letting proptest write a fresh seed. Then extend seed_liveness.rs so a shrink note whose field skeleton disagrees with the strategy's current value type fails the sweep (per suite, a required-field list derived from one generated value's Debug rendering, or a per-suite regex), with a fixture seed demonstrating the verdict. Acceptance: no `cc` line in proptest-regressions/ carries a note lacking a field the current strategy always emits; the disposition comment is gone; seed_liveness fails a fixture seed whose note lacks a required field.

## Positives

- oracle.rs:22-29 implements `Default` by hand so `Oracle<T>` does not acquire a spurious `T: Default` bound: a manual impl with a reason to exist. The oracle is a `BTreeMap` keyed by event index and reads redaction as absence through the public `Snapshot::iter` (73-88), sharing no code with the merge.
- fault.rs draws every stream of a direction on one shared budget and fails the connector and acceptor alongside the writers and readers (198-241), with comments at 202-204 and 228-230 naming the deterministic path each refusal reaches (`SendError::Connect`, the parked accept driver); the `Done` threading at 208-213 and 234-239 preserves the connector's recycle contract instead of discarding it.
- wire.rs:65-93 makes `assert_control_drained` a post-condition of every successful in-memory session (gossip_pair_async:156, bootstrap_fork_configured:262, the executor's retire arm at executor.rs:275), turning a latent control-stream desynchronization into a failure at the session that caused it; gossip_snapshot.rs:449-471 reproduces the same invariant from its log.
- gossip_snapshot.rs:333-387 renders pins from the public observation hook while holding it to the transport capture as a totality oracle (`assert_items_account_for` per stream plus the observed-versus-wire stream count), and the doc at 333-337 says exactly what licenses the pin.
- sim.rs:90-104 derives `MAX_CUT` from a measurement with a two-sided pin (`max_cut_spans_the_envelope_session`, tests/disruption.rs:432) via `fault::metered`, which shares the counters the cuts spend, so generated cuts provably reach every byte of the envelope session.
- sim.rs states its loss accounting (53-71), the custody chain (`lost_custody`, 214-243), and the premise that gates `assert_value_oracle` (961-974) at the declaration sites; the classifier (361-375) admits only truncation and severed-transport kinds and rejects decode-class errors as protocol bugs, keeping the engine on-model; tests/disruption.rs:172 carries the tripwire that the oracle bites known-bad mechanisms; the clippy allow at 597-600 carries its reason.
- schedule/arb.rs keeps generated schedules valid by construction through a shadow simulator that is meta-tested against the live executor (tests/shadow_validity.rs), for both alphabets, comparing sets for a stated reason.
- window.rs puts `Floor` first so failures shrink toward the deadlock-certified baseline (23-26), and each budget endpoint constant names the pin in tests/window_sweep.rs that keeps the sweep from degenerating (85-99); that file exists.
- overlap.rs states why its link buffer is 48 bytes (46-57) and why sides are polled separately (67-77); `SESSION_POLL_BOUND` turns a protocol hang into a failure at its source.
- peer.rs:169-173 and sim.rs:905 turn non-termination into a named failure whose message says what class of bug it indicates.
- flaky.rs designs `FaultFeed` so shrinking toward empty is shrinking toward fault-free (116-122), and `store` fails before touching the durable bytes (210-215), modeling the atomicity the crate's recovery relies on.
- seed_liveness.rs reconstructs proptest's persistence resolution in reverse, guards its own vacuity (220-223), commits a fixture for each verdict class it produces (298-357), and its stated provenance matches Cargo.lock; tests/main.rs is an empty binary whose doc says exactly why it exists and what enforces it.

## Open questions for Finch

- Test-support crate (finding 8): should `tests/common` become a path dev-dependency crate? Recommendation: measure `cargo build --tests --timings` at HEAD and on a branch first; adopt only if the number is material, since finding 10 restores import linting without restructuring.
- `Rumors::send` returning the `Version` it stamped (finding 2): the state-machine argument at src/rumors.rs:190-203 stands; the batching argument does not bind the single-message method. Recommendation: keep the ruling, consolidate the harness's recovery routine, and record that the friction was weighed.
- The overlap harness pins every peer at the floor (overlap.rs:209 and `bootstrap_fork`) while every other schedule engine sweeps windows. Recommendation: give `arb_overlap_schedule` a `WindowAssignment` with `Floor` as the shrink target, unless fork/install overlap is argued regime-independent at the site.
- `Retire::Retired` with absorber `Err` (sim.rs:807-815): with faults only on the retiree's side this arm looks unreachable (`Retired` means the retiree read the absorber's post-commit marker), and no committed seed shows it firing. Recommendation: adopt finding 25, which makes it reachable, and keep the arm; otherwise state at the site that it is conservative and unexercised.
- Cancellation: overlap.rs:81 says dropping an unfinished `Session` models a cancelled one, but no executor drops one, and tests/lifecycle.rs cancels only gossip/gossip. Recommendation: a deterministic cancel-at-every-poll-prefix sweep over a serving bootstrap (guard snap-back before `party.take()`, leak after) and over a retirement, the dual of the cut sweep in finding 22, unless it already exists with forged peers in src/tests.rs.
- Newcomers born mid-chaos (sim.rs:744-750, 768-771) are neither probed by `probe_disjointness` nor given observers until the audit after the chaos phase. Recommendation: hand each newcomer to the prober through a shared `Mutex<Vec<Rumors<u64>>>` as soon as `run_boot` returns, so disjointness is probed over the window in which the fresh fork exists.
- Vocabulary rulings (findings 23 and 24): "honest" as a second sense and em-dashes in `//` comments are both crate-wide. Recommendation: rule each once and sweep mechanically rather than fixing the partition's instances alone.
- The stale seed (finding 32): delete or re-derive? Recommendation: delete; the failure it recorded is fixed, and a fresh regeneration under today's strategy is a different case.
- `WindowChoice::Floor` versus `Budget(1 << MIN_BUDGET_EXPONENT)`: tests/window_sweep.rs pins the latter to one slot. Recommendation: keep both; `Floor` exercises the explicit `sync_window_floor` configuration path, the budget arm the solver path that reaches the same regime.

## Dropped

- pairwise::gossip_converges subsumed by async_wire (api-economics [41]): anchored in tests/async_wire.rs and tests/pairwise.rs, outside this partition; route to the partition owning the test binaries, with the history pass's caveat that proptest-regressions/async_wire.txt holds four cc seeds that must be re-homed into pairwise.txt or seed_liveness fails.
- sanity::arbitrary_schedules_dont_panic re-executes the multi_peer generator (api-economics [42]): anchored in tests/sanity.rs, outside this partition; route likewise.
- "seam" as jargon (part of structure-prose [19]): established crate vocabulary, 48 uses under src/; replacing it in tests/common alone would split the vocabulary. The rest of [19] survives as finding 27.
- Replace the overlap redact guard with `assert!(observed)` (blind-spots [26] resolution): refuted; the guard is load-bearing for a by-design imprecision (finding 12 explains the mechanism and asks for a meter instead).
- Dead `Protocol` import ([30]), unread `ByteMeter.read` ([31]), `let _ = self.label;` ([32]), residue list ([35]), no-op constructs ([16]), `ByteMeter.read` ([1]), suppression leftover ([2]): merged into finding 10.
- Named CBOR constants ([23]), self-check independence ([33]), transcribed internals ([43]): merged into finding 6.
- `fork_tree` duplicate ([5]) and second shadow ([36]): merged into finding 14.
- Pass-through wrapper ([9], [47]), bootstrap helper copies ([10]), retire-into and Option bootstrap ([37]): merged into finding 30.
- `metered` duplication ([45]): duplicate of finding 4. Duplicate bounds ([46]): duplicate of finding 17. Composition map ([48]): duplicate of finding 9. Convergence loop copies ([38]): duplicate of finding 18, and its tests/lifecycle.rs:50-58 citation is the `divergent_pair` fixture, not a loop.
- Refutation's new item 2 (Open doc versus the live fork point): merged into finding 12 as its mechanism.
