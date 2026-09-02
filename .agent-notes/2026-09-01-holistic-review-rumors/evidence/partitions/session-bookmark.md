# Partition session-bookmark: Gossip session driver, bookmark persistence, reconciliation docs, wire observation, message encoding

## Partition summary

This partition is the layer between a `Peer` and the streaming mirror protocol. `src/peer/gossip.rs` erases a caller's `Link` into type-erased parts, runs the preamble, holds the bookmark-and-snapshot critical section, hands reconciliation to the boxed, `inline(never)` `Reconciliation::reconcile` and `bootstrap_reconcile`, performs the party hand-off under `PartyGuard`, commits, and exchanges the epilogue; it also carries the `gossip_when` unfold driver and the `bootstrap`, `gossip`, and `retire` funnels. `src/bookmark.rs` pairs a caller's raw byte store (`Bookmark`) with the in-memory identity record (`Bookmarked`: `reclaim`, `slice`, `record`, and a staged-then-committed write-suppression token), and `src/bookmark/format.rs` is the self-describing, hash-checked CBOR frame it persists. `src/observe.rs` is the rumors-blind wire hook with its crate-internal `Attachment`, `SessionHandle`, and `CaptureRead`; `src/message.rs` is the type-erased payload with its cached CBOR bytes and the `PayloadCodec` fn-pointer pair that keeps sessions non-generic; `src/reconciliation.rs` is a public explanation page with no code.

I read all ten files in full, 4910 lines. Test code is `src/peer/gossip/tests.rs` (385), `src/bookmark/format/tests.rs` (509), `src/observe/tests.rs` (105), and `src/message/tests.rs` (315); `src/reconciliation.rs` (249) is documentation only. I traced every panic site in the production files to a guarding branch or a crate-bug precondition and found none reachable from wire, payload, bookmark bytes, or storage failure; the frame reader is total, the epilogue distinguishes EOF from a wrong item, payload decode is depth-bounded with trailing bytes rejected, and the party donation leaves the guard only after its slice is durable and immediately before `party::send`.

The code is in good shape, and several pieces are exemplary: the monomorphization boundary is placed and priced in its own docs; `PartyGuard` and the donation ordering make identity duplication structurally impossible on every exit path; the bookmark's staged and committed tokens are a careful cancel-safety design with named hazard sections; the frame format and its test suite (every single-byte corruption, every truncation prefix, trailing bytes, non-canonical spellings, a rumors-blind parse, hex-first pins with the re-accept rule stated on the test) are the standard the rest of the crate should be held to; send-side admission is literally the receiver's decode.

The dominant issues are second-order. First, prose that drifted behind code changes: the V1 retirement left two-protocol dispatch prose in `gossip.rs`; the epilogue marker widened to two bytes while its test docs still say one and the first byte is never swept; `Bookmark::load`'s public "once per Peer" is false after a failed store; `slice`'s doc claims a `watch` section its only caller deliberately omits; `is_current`'s doc says "exactly" where the code compares own-party projections; `reconciliation.rs` derives the stream count wrongly against `STREAM_COUNT`'s own doc. Second, structure that could be simpler with no behavior change: `gossip_inner` hand-builds eleven `(Intent, Err(..))` tuples because its return type defeats `?`; `Bookmarked` proves a three-field "loaded" invariant with three `expect`s; the `Persist` trait and `Bookmark::store`'s lent-writer closure both outlived the two-face (async plus blocking) design that justified them; the reclaim-if-stale block is duplicated; `try_from_arc` erases and downcasts a value it could keep typed. Third, a handful of API-surface questions that are the owner's to rule on: `Message` is `pub` with user-voice docs but unreachable; `EncodeError` lacks `#[non_exhaustive]` where its siblings have it; `store`'s shape; whether the observer needs an end-of-session hook.

## Findings

### session-bookmark-1: The bookmark-attach path lives in the wire-session module, which its own doc does not admit
- Where: src/peer/gossip.rs:1-6 (related: src/peer/gossip.rs:136-157, src/peer/gossip.rs:344-399, src/peer/gossip.rs:501-511, src/peer.rs:32, src/peer.rs:272-277)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep: `peer.rs:32` re-exports `Unbookmarked` from `gossip`; `peer.rs:272-277` is a one-line delegate to `bookmark_inner`)
- Seen by: structure; refutation: confirmed; history: no rationale found (historical accretion; the module doc was written later around the drivers)
- Owner-gated: no

The module doc scopes the file to the wire-session drivers, the preamble constants, and `PartyGuard`. `Unbookmarked`, `bookmark_inner`, and `bookmark_record` are the attach path: no session, no wire. A reader following `Peer::bookmark` from `peer.rs` is bounced into the gossip module for semantics unrelated to gossip. Modules have a single clear responsibility (Principle 5 doctrine).

Evidence:

    1	//! The wire-session drivers for [`Peer`]: [`bootstrap`](Bootstrap::join),
    2	//! [`gossip`](crate::Rumors::gossip), and [`retire`](Peer::retire).
    3	//!
    4	//! Also here: the preamble constants every session leads with, and the
    5	//! [`PartyGuard`] that snaps a speculatively donated party back in place
    6	//! on failure.

Resolution: Move `Unbookmarked`, `bookmark_inner`, and `bookmark_record` beside `Peer::bookmark` in `peer.rs` (or a `peer/bookmark.rs` sibling) and drop the `pub use gossip::Unbookmarked` re-export; the public path `rumors::Unbookmarked` is unchanged. `bookmark_update` and `bookmark_donate` are session-time and stay. At minimum, make the module doc list what the file holds. Acceptance: the module doc enumerates the file's contents accurately; `Peer::bookmark`'s implementation is declared in the same file as its public method; `just gate` clean.

### session-bookmark-2: Stray trailing `use serde::..` lines jammed against the next item's doc comment
- Where: src/peer/gossip.rs:43-45 (related: src/message.rs:9-12, src/message/tests.rs:11-12; outside the partition: src/peer.rs:26-28, src/peer/bootstrap.rs:21-23)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read the three sites)
- Seen by: structure, prose; refutation: confirmed (all from the mechanical import sweep c6fe4018); history: no rationale found (an artifact of the sweep)
- Owner-gated: no

Three files end their import section with one or two `use serde::..` lines placed after the main import block and immediately before a `///` doc comment with no blank line. rustfmt leaves separate groups alone, so this passes `just fmt` while reading as an accident; the doc comment visually attaches to the import above rather than the item below.

Evidence:

    43	use serde::Serialize;
    44	use serde::de::DeserializeOwned;
    45	/// The epilogue marker each side writes on the control stream after all

Resolution: Fold each into the main import block (`use serde::{Serialize, de::DeserializeOwned};`) and restore the blank line before the doc comment; sweep `peer.rs` and `bootstrap.rs` in the same pass. Acceptance: no `use` line directly precedes a `///` line in the five files; `just fmt-check` still clean.

### session-bookmark-3: `Gossiped` lacks `PartialEq`/`Eq` although every field is `Eq`; `NoBookmark` derives only `Debug`
- Where: src/peer/gossip.rs:165-167 (related: src/bookmark.rs:196-197)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the derive lines: `Version` is `#[derive(Clone, Eq)]` at crates/before/src/version.rs:94, `Led` derives `PartialEq, Eq` at gossip.rs:186, `SessionStats` derives `Copy, Default, PartialEq, Eq` at stats.rs:39)
- Seen by: perfapi; refutation: confirmed; history: no rationale found
- Owner-gated: no

`Gossiped { converged: Version, led: Led, stats: SessionStats }` cannot be compared, so a caller or test cannot `assert_eq!` two session outcomes. `NoBookmark` is a unit marker used as a type-level default; the free derives (`Clone, Copy, Default, PartialEq, Eq`) are the std idiom for such a type. Zero cost.

Evidence:

    165	#[derive(Debug, Clone)]
    166	#[non_exhaustive]
    167	pub struct Gossiped {

    196	#[derive(Debug)]
    197	pub struct NoBookmark;

Resolution: `#[derive(Debug, Clone, PartialEq, Eq)]` on `Gossiped`; `#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]` on `NoBookmark`. Acceptance: both derives present; gate clean.

### session-bookmark-4: `Gossiped::converged` doc overstates what both sides hold at commit
- Where: src/peer/gossip.rs:168-171 (related: src/peer/gossip.rs:581-583, src/peer/gossip.rs:810-812, src/peer/gossip.rs:869)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: correctness; refutation: confirmed; history: no rationale found (original wording from the driver commit; the private doc was later made precise without updating the public field)
- Owner-gated: no

The public field doc says both replicas "held exactly this version" at commit. The commit joins the reconciled tree into a tree that may have advanced during the session (`inner.tree.join(merged)` at 869), so a side that ran a `send` or `redact` concurrently holds a frontier strictly above `converged`. The private doc at 581-583 ("before any commits that ran concurrently with the session") and the comment at 810-812 state the accurate rule; the public contract disagrees with them.

Evidence:

    168	    /// The causal frontier the two replicas converged on.
    169	    ///
    170	    /// At the instant the session committed, both held exactly this version.
    171	    pub converged: Version,

Resolution: "The reconciled frontier both replicas absorbed at commit; a side that committed local work during the session holds a frontier above it." Acceptance: the field doc agrees with `gossip_inner`'s doc at 581-583.

### session-bookmark-5: "Seam" and "knob" as metaphor-jargon, one of them in public rustdoc
- Where: src/peer/gossip.rs:177-178 (related: src/peer/gossip.rs:611, src/peer/gossip.rs:1201; crate-wide: src/tree/mirror/streaming/stats.rs:6, 29, 104, 122 and about 48 `seam` / 46 `knob` sites under src/)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep: three partition sites; `grep -rn -i '\bseam' src/ | wc -l` = 48, `\bknob` = 46; `SessionStats`' public docs use "seam" at stats.rs:6, 29, 104, 122)
- Seen by: prose; refutation: reframed (crate-wide vocabulary, not a partition edit); history: no rationale found (the `mint` purge, 2c73d032, is the precedent for a crate-wide sweep)
- Owner-gated: yes: "seam" is the crate's established public term in `SessionStats`, so replacing it is a crate-wide vocabulary decision

`Gossiped::stats` (public) tells the user to see `SessionStats` for "the seam it is counted at"; the word is a metaphor promoted to jargon that the writing-style rule flags, and a user has no anchor for it. Fixing gossip.rs:177 alone would make the two public docs inconsistent, because `SessionStats` uses the same word. "Knob" at 1201 is a private one-off with a plain replacement.

Evidence:

    177	    /// See [`SessionStats`] for each field's mechanism and the seam it is
    178	    /// counted at.

    1201	        // knob: the greeting advertises it, and the provider's supply runs

Resolution: Owner decision on "seam" crate-wide ("boundary" or "layer"); if it goes, sweep `SessionStats` and this site together. "Knob" at 1201 becomes "setting" now. Acceptance: no "knob" in the partition; "seam" either everywhere or nowhere in public rustdoc.

### session-bookmark-6: Em-dashes in `//` comments (23 sites in the partition, 116 crate-wide)
- Where: src/peer/gossip.rs:256-257 (related: gossip.rs 325, 445, 544, 646, 688, 838, 843, 847, 858, 863, 879, 891, 944, 978, 984, 985, 990, 1002; src/bookmark/format.rs:239; src/message.rs:377-378)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n -E '^\s*//[^/!].*—'` over the ten files: 23; over all of src/: 116; no em-dash in any string literal in the partition)
- Seen by: prose; refutation: confirmed (crate-wide sweep plus a lint, not a partition edit); history: no rationale found (the recorded em-dash passes targeted rendered prose and one assert string; no lint exists)
- Owner-gated: no

Plain comments use the typographic em-dash where the house rule is the spaced double-hyphen (dashes by register: chat and code comments use `--`). Fifteen of the partition's sites predate the recorded em-dash passes and eight postdate them, which is the signature of a convention held in memory rather than a check.

Evidence:

    256	            // Un-poison on clean completion: both `Ok` arms — a completed
    257	            // donation and a mutual-bootstrap bail — end with the epilogue

Resolution: Replace ` — ` with ` -- ` (or a colon) in `//` comments crate-wide, and add a `tools/` lint that rejects U+2014 on non-doc comment lines so the rule is a check, not a convention. Acceptance: the grep above returns nothing over src/; the lint is wired into `just gate`.

### session-bookmark-7: `let _ = remote.intent;` is a no-op standing in for a comment
- Where: src/peer/gossip.rs:297-300
- Class / severity / confidence: vestigial / nit / high
- Provenance: assessed (read; `remote` is used at 308 and 329, so no unused-binding warning depends on the statement)
- Seen by: structure; refutation: confirmed; history: deliberate but expired (it suppressed an unused-binding warning for a destructured `remote_intent`; when the preamble started returning a struct the line was rewritten mechanically and became a no-op)
- Owner-gated: no

The statement does nothing: `remote` is a struct whose other fields are read below, so no warning is being silenced. Code that exists only to host a comment is noise; the comment carries the whole content.

Evidence:

    297	            // In the bootstrap case, it doesn't matter whether the remote intends
    298	            // to remain or retire; they will hand us a party regardless, and we can
    299	            // absorb it.
    300	            let _ = remote.intent;

Resolution: Delete line 300; keep the comment where the bootstrap ignores the remote's intent. Acceptance: line gone; no new warning.

### session-bookmark-8: `bookmark_inner` rebuilds `Peer` field-by-field twice to swap the bookmark type parameter
- Where: src/peer/gossip.rs:344-399 (related: src/peer/gossip.rs:328-339, src/peer.rs:216-225)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed; history: no rationale found (three fields when written, accreted to seven by extending both lists)
- Owner-gated: no

Attaching a bookmark destructures `self` and rebuilds a `Peer<T, B>` (349-366), then on a failed record rebuilds a `Peer<T, NoBookmark>` (387-395), copying seven fields each time. Struct-update syntax cannot change the type parameter, so a helper must destructure too, but once: a private `fn with_bookmark<B2: BookmarkError>(self, bookmark: B2) -> Peer<T, B2>` collapses both and states the intent ("same peer, different bookmark") that the field copies only imply. A field added to `Peer` is then threaded in one place instead of three.

Evidence:

    386	            Err(error) => Err(Unbookmarked {
    387	                peer: Peer {
    388	                    network: peer.network,
    389	                    window: peer.window,
    390	                    run_budget: peer.run_budget,
    391	                    inner: peer.inner,
    392	                    bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
    393	                    codec: peer.codec,
    394	                    observe: peer.observe,
    395	                },

Resolution: Add `with_bookmark`; `bookmark_inner` becomes `let peer = self.with_bookmark(bookmark); if pristine { return Ok(peer) } match peer.bookmark_record().await { Ok(()) => Ok(peer), Err(error) => Err(Unbookmarked { peer: peer.with_bookmark(NoBookmark), error }) }`. Acceptance: `bookmark_inner` contains no field list; gate clean.

### session-bookmark-9: `gossip_inner`'s return contract under-states the Retire-with-error arm
- Where: src/peer/gossip.rs:575-579 (related: src/peer/gossip.rs:782-802, src/peer/gossip.rs:894-899, src/peer/gossip.rs:126-129)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the doc named the only such path when written; the epilogue commit added the post-send epilogue failure without touching the doc)
- Owner-gated: no

The doc says `Intent::Retire` arrives with an error "when sending the party itself fails". The body also returns `(outcome, Err(..))` with `outcome == Intent::Retire` when the epilogue fails after a successful send (897-899, explained at 894-896). A reader of the doc alone would believe a post-hand-off epilogue failure maps to `Remain`, the duplication hazard the code guards against. The public `Retire::Uncertain` doc (126-129) covers both cases; only this private contract is incomplete.

Evidence:

    577	    /// off to the counterparty via retirement. `Intent::Retire` can arrive
    578	    /// *with* an error: when sending the party itself fails, we cannot know
    579	    /// whether the remote received it, so we must assume it might have.

Resolution: "`Intent::Retire` can arrive with an error whenever the party may already be held by the peer: the send itself failed, or the send succeeded and the epilogue after it failed." Acceptance: the doc names both post-hand-off failure paths.

### session-bookmark-10: `gossip_inner` hand-builds eleven `(Intent, Err(..))` returns because its tuple return type defeats `?`
- Where: src/peer/gossip.rs:598-905 (related: src/peer/gossip.rs:443-455, src/peer/gossip.rs:472-487, src/peer/gossip.rs:1028-1064)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (grep: nine `return (Intent::Remain, Err(` sites at 627, 639, 679, 714, 746, 762, 775, 873, 885 and two `return (outcome, Err(` at 793, 898, both after `party::send` at 782)
- Seen by: structure; refutation: confirmed (with one implementation detail: the error type must not offer both `From<Error>` and `From<Error<B>>`, which overlap at `B = NoBookmark`); history: no rationale found (the tuple shape is original to the first split of the gossip module and was only ever justified by its semantics)
- Owner-gated: no

The session transaction returns `(Intent, Result<..>)` so that a retiree whose party crossed the wire is never handed back. That property matters at exactly two sites (793 and 898, which carry `outcome`), but the shape forces nine more sites to spell `return (Intent::Remain, Err(..))` by hand, so a reviewer must check every early return in a 300-line function to confirm the Intent is right, rather than seeing the post-hand-off region as the only place a non-Remain outcome can arise. Finished code should be obviously, reviewably correct; the safety argument (identity never duplicated) should be checkable at the two sites where a party is in flight.

Evidence:

    603	    ) -> (Intent, Result<(Version, SessionStats), Error<B>>)

    627	                Err(error) => return (Intent::Remain, Err(Error::from(error).widen())),

    679	                return (Intent::Remain, Err(Error::Bookmark(e)));

    793	                    return (outcome, Err(e.widen()));

Resolution: Introduce a module-private `struct Aborted<B: BookmarkError> { outcome: Intent, error: Error<B> }` with `From` impls that fix `outcome: Intent::Remain`: `From<Error>` (the widening source, since `Error = Error<NoBookmark>`; sites that `.widen()` today keep it before `?`), `From<BookmarkIo<B::Error>>`, and `From<handshake::Error>`; do not also implement `From<Error<B>>`, which overlaps at `B = NoBookmark`, and route the `Error::PartyOverlap` site (873) through the widening impl. Return `Result<(Intent, Version, SessionStats), Aborted<B>>`; the nine pre-hand-off sites become `?`; only 793 and 898 construct `Aborted { outcome, error }` explicitly. `retire_inner`, `gossip`, and `gossip_when` destructure at their match. Alternative with the same payoff: split at the reconciliation boundary into an `open` phase returning a plain `Result` and a `commit` phase that owns the Intent. Acceptance: `grep -c 'return (Intent::Remain, Err(' src/peer/gossip.rs` is 0; exactly two sites construct a non-Remain outcome and both sit after `party::send`; `tests/retire.rs` and `tests/lifecycle.rs` pass unchanged; `just gate` clean.

### session-bookmark-11: `bookmark_update`'s reclaim-if-stale block is duplicated inline in `gossip_inner`, with a needless `Version` clone at both sites
- Where: src/peer/gossip.rs:681-694 (related: src/peer/gossip.rs:537-553, src/peer/gossip.rs:884, src/bookmark.rs:282-298, src/bookmark.rs:436-479, tests/bookmark_when.rs:26 and 49)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `bookmark_update()` has one caller, line 884; `is_current` is called at 541 and 685 with the identical `let version = inner.tree.latest().clone();` above each; `Tree::latest` returns `&Version` at src/tree.rs:220; `reclaim` takes `&Version` and clones internally at bookmark.rs:469, 478)
- Seen by: structure; refutation: confirmed (adds that tests/bookmark_when.rs:26 and 49 name `Bookmarked::is_current` in prose and must move with it); history: deliberate and holds for placement (the single-critical-section design, 3f33afab, stated inline at 645-670), not for the duplicated decision
- Owner-gated: no

The block `if let Some(party) = inner.party.as_mut() { let version = inner.tree.latest().clone(); if !bookmark.is_current(..) { bookmark.reclaim(..); persist = true } }` appears in `bookmark_update` (539-549) and again inside `gossip_inner`'s combined critical section (683-694). The inline copy is a deliberate consequence of the one-critical-section design, which is a reason to factor the decision, not to copy it: a future change to the suppression rule must be made twice. Both copies clone `latest()` although `inner.party` and `inner.tree` are disjoint field borrows and `reclaim` clones internally, so the call-site clone is a third copy of the version. The decision "does this identity need re-recording" belongs to `Bookmarked`, whose `is_current` doc already calls itself the suppression test.

Evidence:

    683	                if let Some(party) = inner.party.as_mut() {
    684	                    let version = inner.tree.latest().clone();
    685	                    if !bookmark.is_current(party, &version) {

    539	            if let Some(party) = inner.party.as_mut() {
    540	                let version = inner.tree.latest().clone();
    541	                if !bookmark.is_current(party, &version) {

Resolution: Make `Bookmarked::reclaim` return `bool` (true when it recorded, i.e. when the token was stale), folding `is_current` in as private. Both sites become `if let Some(party) = inner.party.as_mut() { persist = bookmark.reclaim(self.network, party, inner.tree.latest()); }` with no clone. Update the two prose references in tests/bookmark_when.rs to state the rule ("own region advanced or party changed") without the private name. Acceptance: `grep -rn is_current src/peer tests/` is empty; one place in the crate decides suppression; the two `.clone()` lines are gone; bookmark suites green.

### session-bookmark-12: Two-protocol dispatch prose survives the V1 retirement
- Where: src/peer/gossip.rs:723-729 (related: src/peer/gossip.rs:63, 258, 473, 615-617, 1264; src/peer/gossip/tests.rs:6, 184, 247, 261, 316, 326; src/observe.rs:236-264)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for `V2|dialect|selected protocol|Both branches|neither concrete|both towers|whichever protocol` over the partition; read `Attachment::begin` at observe.rs:236-264, which has no dialect branch; `.agent-notes/2026-09-01-v1-retirement/README.md:225-226` records "drop the dialect guard in `Attachment::begin`"; README:263-265 records the sweep pattern `alternating\|protocol-v1\|\bv1\b`, which explains why V2-side qualifiers survived)
- Seen by: structure, prose, perfapi (open question); refutation: reframed (dispatch and contrast prose is stale; bare names of the live `Protocol::V2` variant are accurate); history: deliberate but expired (every phrase was accurate under two dialects; 368da2a5 removed the second; the `Protocol` enum itself stays public as wire vocabulary by recorded ruling, README:155-158)
- Owner-gated: no for the dispatch prose and the inert `v2_` test-name prefixes; yes for dropping "V2" as the name of the current dialect (tied to the `Protocol` enum, protocol.rs, outside this partition)

With one protocol, prose that describes a choice, a contrast, or a gate no longer present is a ghost reference (Principle 5). Line 616 conditions the observation handle on "the dialect is observable", a guard `Attachment::begin` does not have (the sibling field doc at 1099-1101 was corrected by the retirement; this comment was not). Lines 723-729 describe "this peer's selected protocol", "Both branches", and "neither concrete protocol state machine" over a single `Reconciliation`. Line 63 "re-instantiate both towers" dates to when `mirror/alternating` existed; 258 and 473 "under V2" imply a contrast partner; 1264 "whichever protocol carries it" has one carrier. In tests.rs the `v2_` prefixes on the two test names (261, 326) distinguished them from `v1_` twins the retirement deleted. Sites that merely name the live `Protocol::V2` (gossip.rs:51, 1276; observe/tests.rs:58, which asserts `protocol: Protocol::V2`; reconciliation.rs:246, fresh authorship by 368da2a5) are accurate today.

Evidence:

    615	        // The session's observation handle: inert unless a handler is
    616	        // attached and the dialect is observable, and shared, like the
    617	        // recorder, by every layer that moves a wire item.

    723	        // Reconcile using this peer's selected protocol. Both branches meet at
    724	        // the lifecycle boundary the surrounding transaction needs: a local
    725	        // root plus raw transport halves positioned after reconciliation.
    726	        // The protocol bodies live behind the non-generic [`Reconciliation`],
    727	        // whose methods return their futures boxed: neither concrete
    728	        // protocol state machine becomes part of this outer session future,
    729	        // or of the consumer crate that instantiates it.

Resolution: 616: match the field doc at 1099-1101 ("inert unless a handler is attached"). 723-729: single-path prose ("The reconciliation runs behind the non-generic [`Reconciliation`], whose future is boxed so the protocol state machine stays out of this session future and out of consumer crates"). 63: "the protocol tower". Delete "under V2" at 258 and 473 and "whichever protocol carries it" at 1264. In tests.rs drop the `v2_` prefixes and the "under V2" qualifiers at 247 and 316. Whether "V2" survives as the dialect's name in prose (51, 1276, tests.rs:6, 184) follows the owner's ruling on the `Protocol` enum. Acceptance: `grep -n 'dialect\|selected protocol\|Both branches\|neither concrete\|both towers\|whichever protocol\|fn v2_' src/peer/gossip.rs src/peer/gossip/tests.rs` is empty.

### session-bookmark-13: "Unreachable in practice" on an arm whose one-line proof is available
- Where: src/peer/gossip.rs:825-828
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read; the `None` arm runs only when absorbing, which excludes `self_retiring` via the both-retiring bail at 637-643 and the preamble's rejection of bootstrap-plus-retire, so `inner.party` was never taken at 702; `Peer` is `!Clone` and `retire` consumes it)
- Seen by: prose; refutation: confirmed; history: no rationale found (original comment, type renamed only)
- Owner-gated: no

The arm is total (it adopts the donation, no panic), so behavior is fine, but the comment uses the exact phrase the doctrine says carries no weight ("in practice") where the invariant that makes the arm dead is short and true: a live `Peer` always holds its party, because a retiree is consumed and a recovered one has its party re-joined by `PartyGuard`.

Evidence:

    825	                    // Unreachable in practice: we hold a live `Peer` and are
    826	                    // not retiring, so our party is present. Adopting the
    827	                    // donation keeps the arm total without a panic path.

Resolution: "A live `Peer` always holds its party (a retiree is consumed; a recovered one has its party re-joined by `PartyGuard`), so this arm cannot run; adopting the donation keeps it total." Acceptance: the comment names the invariant rather than its rarity.

### session-bookmark-14: Prose mechanics sweep: misplaced comment, mid-doc link definition, ragged wraps, wrong term of art, stale cross-reference, grammar
- Where: src/peer/gossip.rs:901-904 (related: src/peer/gossip.rs:68-69, 592-597, 699; src/peer/gossip/tests.rs:156-157; src/bookmark.rs:72-73, 118-119, 438; src/bookmark/format/tests.rs:1; src/observe.rs:116; src/reconciliation.rs:55, 233-235)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read each site)
- Seen by: prose; refutation: confirmed (each item); history: no rationale found for the items kept; (g) the AGENTS.md pointer in format/tests.rs:490-492 was owner-ratified (66d782e5) and is dropped; (h) the observe.rs:116 pointer expired when the module doc's "# Back-pressure" section became the "Never block" bullet; reconciliation.rs:202 "Why 17?" and 214 "twiddle its thumbs" are Finch's own words (77334965) and are left as register choices
- Owner-gated: no

Small items, one pattern each. (a) 901-904 narrates the caller's decision ("so don't hand back the `Peer`") at a return of `(Intent, Result)` that never touches the peer; the decision lives in `retire_inner`'s match at 450-455. (b) The link reference definition at 592 sits mid-doc with a paragraph after it (594-597). (c) Ragged wraps: gossip.rs:68-69 ("The price is one vtable" / "call per stream"), tests.rs:156-157 ("a redaction advances" / "the ceiling"), bookmark.rs:72-73 ("Every unreclaimed incarnation" / "stays"), 118-119 ("A frame" / "that loads"), reconciliation.rs:55 (well past the paragraph's measure). (d) bookmark.rs:438 `// Get the clocks for this network` narrates the next line. (e) format/tests.rs:1 "self-inverse" means f(f(x)) = x; the property tested is invertibility. (f) observe.rs:116 points to "the module docs' back-pressure contract"; the bullet is titled "Never block". (g) reconciliation.rs:233-235 "The budget's full details ... lives at" has a plural subject. (h) gossip.rs:699 "We only can have" reads as "We can only have".

Evidence:

    901	        // In the case where we successfully retired (only callable on the
    902	        // !Clone `Peer<T>`), we've given away our inner party and no more
    903	        // actions are possible, so don't hand back the `Peer`.
    904	        (outcome, Ok((converged, stats.snapshot())))

    1	//! The frame is self-inverse and self-checking: it round-trips any payload,

Resolution: (a) delete or move to `retire_inner`'s match; (b) move the reference definition to the end of the doc block; (c) re-wrap; (d) delete; (e) "invertible" or "round-trips"; (f) "the module docs' never-block rule"; (g) "live at"; (h) reorder. Acceptance: each listed site reads as described.

### session-bookmark-15: The two erased reconciliation drivers assemble identical protocol towers
- Where: src/peer/gossip.rs:1186-1231 (related: src/peer/gossip.rs:1071-1079, 1127-1168, 1143, 302-327; src/tree/mirror/streaming/materialized.rs:427-441; src/tree/mirror/streaming/remote/proxy/start.rs:69-79)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified for the equivalence premises (read both `start` constructors: materialized defaults `stats: Recorder::default()` at materialized.rs:439 and the proxy defaults `stats: Recorder::default()` and `observe: SessionHandle::default()` at start.rs:75, 77; `Handshaking<B, V>` is constructed only by `impl<B: ..> Handshaking<B, Start>::start` at materialized.rs:427, so the `::<_, _>` turbofish at 1143 asserts only an arity); the duplication itself assessed by reading both bodies
- Seen by: structure, perfapi (turbofish); refutation: confirmed (merging is a judgment call; the duplication and the doc imprecision are real); history: no rationale found (the split mirrors the pre-existing gossip_inner/bootstrap_erased split and was carried through the sealing commit as a rename; the turbofish dates to that commit while the sibling body used the plain form)
- Owner-gated: no

`Reconciliation::reconcile` (1143-1151) and `bootstrap_reconcile` (1203-1209) build the same two `Handshaking` towers with the same builder chains and `Link::for_session`, differing only in the post-greeting check (`Claimant` newborn check versus `NetworkMismatch`), the bootstrap's mutual-bail epilogue (which `bootstrap_erased` could perform itself; it already calls `epilogue` at 327), and a `.stats(..)` call present in one and absent in the other (equivalent, since both `start`s default it). A builder added to the tower must be added in two places, and the asymmetry has already begun. The `Reconciliation` struct doc (1071-1079) says the struct "exists so that body is non-generic", but non-genericity comes from the boundary (free fn plus boxed future plus `inline(never)`), as `bootstrap_reconcile` itself demonstrates; the struct is a parameter bundle, which is fine but not the stated reason.

Evidence:

    1143	            let local = materialized::Handshaking::<_, _>::start(Local, root.into())
    1144	                .window(window)
    1145	                .target_message_size(run_budget.bytes() as u64)
    1146	                .stats(stats.clone());

    1203	        let local = materialized::Handshaking::start(Local, local_root)
    1204	            .window(window)
    1205	            .target_message_size(run_budget.bytes() as u64);

Resolution: Keep `Reconciliation` as the bundle and add an admission variant (`Claimant`, `Network { remote, local, local_min_events }`, `None`); `bootstrap_erased` builds it with `Root::default()`, a default `Recorder`, and the claimant admission, then performs the mutual-bail epilogue and returns `Ok(None)` itself. Delete `bootstrap_reconcile`, its `#[allow(clippy::type_complexity)]`, and the `Option` in its return. Drop the `::<_, _>` at 1143 either way. Fix the struct doc to say it bundles the erased inputs and that the boundary is what pins codegen. Acceptance: one `Handshaking::start` pair in gossip.rs; bootstrap and gossip snapshot suites unchanged (the wire is untouched); `just gate` clean.

### session-bookmark-16: "Honest", "genuine", and "real" as moralized adjectives where the crate's trust model owns the word
- Where: src/peer/gossip.rs:1282-1284 (related: src/peer/gossip.rs:383; src/peer/gossip/tests.rs:10, 84, 112, 155, 158, 318, 345, 349, 375, 377, 383; src/bookmark/format.rs:48; src/bookmark/format/tests.rs:19; tests/common/sim.rs:359-395)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep -n -i for `honest|genuine|real` across the partition; AGENTS.md's model-of-record rule uses "authenticated-honest-peer"; tests/common/sim.rs:359-395 uses "honesty of failures" and `is_honest_error` for EOF-versus-corruption)
- Seen by: prose; refutation: confirmed; history: no rationale found for most sites; one collision noted: "honest wire cut" borrows the sim harness's failure-honesty vocabulary (Finch's b7fb409f), which predates the AGENTS.md trust-model sense (a59dc786), so two in-crate senses of "honest" collide
- Owner-gated: no

"Honest wire cut" (a plain EOF), "honest bootstrap" / "honest newborns" / "genuinely newborn" (a claimant with an empty greeting version), "genuinely untouched", "real semantics", "a genuine `events`-tick version". The cost is specific: the model of record is "authenticated-honest-peer", so "honest" is a term of art for the trust assumption, and using it for "an EOF rather than a bad byte" or "a newborn rather than a misdeclaring claimant" blurs the term at exactly the sites (bootstrap history conflict) that are conformance-bug detectors, not trust boundaries. The writing-style rule asks for the property instead of the adjective.

Evidence:

    1282	/// resolves, so the exchange cannot deadlock. Failure is [`Error::Epilogue`]:
    1283	/// post-commit by construction, with a non-marker byte surfaced as an
    1284	/// invalid-data protocol violation rather than an honest wire cut.

    318	/// encounter were two honest newborns.

Resolution: "a plain wire cut" / "an EOF"; "a newborn claimant (empty greeting version)"; "a bootstrap whose greeting version is empty"; "untouched"; drop "real" and "genuine" where the noun carries the meaning. Decide separately whether the sim harness keeps its "honesty of failures" vocabulary or renames toward "cut versus corruption", so the crate has one sense of the word. Acceptance: `grep -n -i 'honest\|genuine' src/peer/gossip.rs src/peer/gossip/tests.rs src/bookmark/format/tests.rs` returns nothing; "honest" appears in the crate only in its trust-model sense.

### session-bookmark-17: `PartyGuard` has `pub(crate)` fields on a module-private struct and a `//` comment where the module doc links a rustdoc item
- Where: src/peer/gossip.rs:1375-1382 (related: src/peer/gossip.rs:5, justfile:270)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep: `PartyGuard` appears only in gossip.rs at lines 5, 671, 1379, 1384, 1395; the gate's `docs-internal` leg at justfile:270 runs `cargo doc --document-private-items` with `-D warnings`)
- Seen by: structure, prose, perfapi; refutation: confirmed; history: no rationale found (Finch's original comment from the module split; the module-doc link was added later without promoting the comment)
- Owner-gated: no

The struct is private to this module and constructed only at 671, so `pub(crate)` on its fields grants nothing beyond private. The module doc links `[`PartyGuard`]` at line 5, but the type's explanation is a `//` block, so under `--document-private-items` the link lands on an undocumented item and the rationale is invisible in the rendered internal docs.

Evidence:

    1375	// To ensure that a speculatively forked party always snaps back in place, even
    1376	// if we return an error or panic, we place it in a drop-guard that joins it
    1377	// back into the remaining party in the `inner` if we don't donate it
    1378	// successfully along any return path.
    1379	struct PartyGuard<T> {
    1380	    pub(crate) party: Option<Party>,
    1381	    pub(crate) recover: watch::Sender<Inner<T>>,
    1382	}

Resolution: Drop the `pub(crate)`; turn the comment into `///` on the struct, stated positively ("Holds a party removed from `Inner` for donation; on drop, re-joins it unless taken, so every non-donating exit path restores the identity"), with a line each on the two fields. Acceptance: `PartyGuard` renders with a doc under `--document-private-items`; gate clean.

### session-bookmark-18: Epilogue marker tests document a one-byte marker and never vary the first byte
- Where: src/peer/gossip/tests.rs:79-89 (related: src/peer/gossip/tests.rs:6-8, src/peer/gossip.rs:45-54, src/peer/gossip.rs:1297-1307, src/tree/mirror/cbor.rs:73)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`git show 4dd2053c -- src/peer/gossip/tests.rs` changed only the comparison lines, `[byte]` to `[EPILOGUE_MARKER[0], byte]` and `== EPILOGUE_MARKER` to `== EPILOGUE_MARKER[1]`; `git show 4dd2053c^:src/peer/gossip.rs` has `const EPILOGUE_MARKER: u8 = b'.';`; `SELF_DESCRIBED_HEAD` is `[0xd9, 0xd9, 0xf7]` at cbor.rs:73)
- Seen by: prose, correctness; refutation: confirmed, severity lowered from medium (the comparison at gossip.rs:1299 is a whole-array `!=`, so the untested first byte cannot currently diverge); history: deliberate but expired (docs written for the `u8` marker; the CBOR wire change widened the constant and edited only the two comparison lines)
- Owner-gated: no

The module doc and the exhaustiveness test's doc describe the marker as a single byte and claim the desynchronized-preamble case is covered, but `EPILOGUE_MARKER` is `[u8; 2]` and the body of `marker_byte_space_is_exhaustive` holds the first byte fixed at `EPILOGUE_MARKER[0]` and sweeps only the second. The one input the marker's own doc (gossip.rs:50-53) is designed around, a preamble's self-described tag `0xd9` arriving where the marker belongs, is constructed by no test in the crate. An inaccurate testdoc is a bug in the test (AGENTS.md), and a regression that compared only `marker[1]` would pass this suite.

Evidence:

    6	//! - The V2 session epilogue marker: the last wire ingress of every V2
    7	//!   session, one byte read from the control stream after all session
    8	//!   work. The suite exhausts that byte space and its truncation directly

    79	/// Marker decoding is exhaustive: exactly the one marker byte is accepted
    80	/// and every other byte is a typed protocol violation.

    88	    for byte in u8::MIN..=u8::MAX {
    89	        let bytes = [EPILOGUE_MARKER[0], byte];

Resolution: Re-state the module doc (6-8) and the testdoc (79-85) for the two-byte CBOR text item `"."`. Sweep both positions: either the full 65536-pair space (cheap) or the first byte over `u8` with the second fixed plus the existing sweep, asserting `InvalidData` for every non-marker pair. Add a named case feeding `[0xd9, 0xd9]` (the opening of `SELF_DESCRIBED_HEAD`) and asserting `Error::Epilogue` with `InvalidData`, so the preamble-desync claim is pinned rather than asserted. Acceptance: the docs say "two-byte item"; a committed case rejects a preamble opening; a mutant that compares only `marker[1]` fails that case.

### session-bookmark-19: Test helpers carry braces left over from removed `.batch(..)` closures, and one helper doc promises a value the function does not return
- Where: src/peer/gossip/tests.rs:162-165 (related: src/peer/gossip/tests.rs:172-174, 225-234, 263)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (`git show 212c6914^:src/peer/gossip/tests.rs` shows the three blocks wrapping `.batch(|batch| { .. })` closures; `send_all`/`redact_all` replaced the bodies and left the braces)
- Seen by: structure, prose; refutation: confirmed; history: deliberate but expired (the braces first scoped a RAII `Batch` guard, then a closure, and now nothing)
- Owner-gated: no

Three bare `{ ... }` blocks (163-166, 172-174, 228-232) each wrap a single statement and scope nothing, so a reader looks for the guard that is not there. `provider_with`'s doc (225) promises "plus its pre-session root hash" but the fn returns only `Peer<u64>`; the caller computes the hash itself at 263.

Evidence:

    162	    {
    163	        donor
    164	            .send_all(0..events)
    165	            .expect("flat test payloads are within any depth limit");
    166	    }

    225	/// A provider holding `values`, plus its pre-session root hash.
    226	fn provider_with(values: &[u64]) -> Peer<u64> {

Resolution: Remove the three brace pairs; make the doc "A seeded provider holding `values`." Acceptance: no single-statement bare blocks remain in the file; the doc matches the signature.

### session-bookmark-20: The bookmark format's decoder is crate-private, so an operator holding a bookmark file has no read-only way to inspect it
- Where: src/bookmark.rs:24 (related: src/bookmark/format.rs:423-426, src/bookmark.rs:77-82, src/peer/gossip.rs:153-156)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: verified (grep: `pub(crate) fn decode` at format.rs:423; `BOOKMARK_FORMAT_VERSION`, `FormatError`, `FrameDefect`, `RecordDefect` re-exported at bookmark.rs:24 and lib.rs:331-334)
- Seen by: perfapi; refutation: reframed (the error taxonomy does have a public producer, a live peer's session or attach failure; the real gap is an offline reader); history: no rationale found
- Owner-gated: yes: new public API

The crate owns, versions, and refuses to migrate the persistence format, and the trait doc (77-82) says a mis-filed bookmark from another universe is silent ("the foreign identities simply lie dormant"). An operator deciding whether a file is safe to delete, or is the right one to attach, has no way to ask the crate which networks and how many stranded identities it holds; the only decoder runs inside a live peer.

Evidence:

    24	pub use format::{BOOKMARK_FORMAT_VERSION, FormatError, FrameDefect, RecordDefect};

    423	pub(crate) fn decode(bytes: &[u8]) -> Result<BTreeMap<Network, Vec<Clock>>, FormatError> {

Resolution: Owner decision. Expose a read-only `inspect` over frame bytes returning the record or a summary (`Network` to count and per-clock own version), documented as diagnostic only. Acceptance: a doctest decodes the bytes a `Bookmark` implementor stored and lists networks; the format pins are unaffected.

### session-bookmark-21: The `Bookmark` doc promises reclaim at the first dominating gossip; the code reclaims at the first unsuppressed pre-session checkpoint, and no test pins the timing
- Where: src/bookmark.rs:61-66 (related: src/peer/gossip.rs:682-694, src/bookmark.rs:285-298, src/bookmark.rs:412-417, src/peer.rs:250-253, tests/bookmark_when.rs:12-28, tests/bookmark_causality.rs:905-951)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read the gate at gossip.rs:685, the predicate at bookmark.rs:296, the pre-session version read at gossip.rs:684, and `assert_no_leak`'s counting of checkpointed regions at tests/bookmark_causality.rs:905-914)
- Seen by: correctness; refutation: confirmed (adds a second gap: even an unsuppressed checkpoint uses the pre-session version, so the session that first achieves dominance never reclaims); history: deliberate and holds for the code (Finch's ruling, 9cf90dfb, "Only persist bookmark when there have been *local* changes", with the reclaim-delay consequence acknowledged in Finch's own `record` doc at 414-416, "rather than stranded until the next event"); the public trait sentence is from the July doc passes and is the drift
- Owner-gated: yes: choosing between re-wording the public doc to the code's rule and changing the persist gate, which relaxes a pinned schedule

The trait doc says a prior incarnation's identity is reclaimed "at the first gossip that causally dominates everything that incarnation had itself recorded". `reclaim` runs only inside the pre-session checkpoint (gossip.rs:682-694), which is skipped whenever `is_current(party, version)` holds, and `is_current` compares own-party projections only (bookmark.rs:296). A session that advances the frontier purely by learning other parties' events leaves the projection unchanged, so a gossip that newly dominates a stranded incarnation's own writes does not reclaim it; the identity waits for this peer's next local send or redact, or its next restart. Further, the checkpoint reads the pre-session version (684), so even an unsuppressed checkpoint cannot reclaim on the session that achieves dominance. Safety is unaffected (a late reclaim never recycles a coordinate), and the code's rule is deliberate; the public contract is what drifted. The verification gap: `assert_no_leak` deliberately counts checkpointed-but-unreclaimed regions as accounted for, and `tests/bookmark_when.rs` models writes, not reclaims, so nothing would fail if either the doc's rule or the code's rule were wrong.

Evidence:

    61	/// The prior incarnation's identity is then reclaimed out of the record at
    62	/// the first gossip that causally dominates everything that incarnation
    63	/// had itself recorded: its own writes, not everything it had observed.

    685	                    if !bookmark.is_current(party, &version) {

    296	            p == party && v / p == version / p

Resolution: Owner decision between two consistent states. (a) Recommended, consistent with the recorded ruling: re-word bookmark.rs:61-66 to the actual gate ("reclaimed at the first session whose pre-session checkpoint is not suppressed after the frontier dominates it: the first session after attach, or after any own event or party change following the dominating gossip"); peer.rs:250-253's "behind that path's persist gate" is already accurate. (b) Make the pre-session gate also fire when the record holds a clock with `own_version() <= version` that the live party does not cover; this relaxes the never-write-on-hearsay rule `tests/bookmark_when.rs` pins, so its model needs a `pending` rule for reclaimable clocks. Under either choice, add a test asserting the persisted record's contents after a hearsay-only dominating gossip. Acceptance: a committed test constructs the scenario below and its expectation matches the documented rule; the trait doc and the code state the same rule.

Construction: Seed S; bootstrap Q from S with a `FlakyInMemoryBookmark` (or `Probe`) store; Q sends m and gossips it to a third peer R only; drop Q's peer (crash), keeping its store. Bootstrap a new incarnation P from S (which lacks m) with the same store via `Bootstrap::bookmark`; gossip P with S (the attach-time checkpoint reclaims nothing: Q's own version is not dominated). Gossip P with R so P learns m and now dominates Q's own version. Under the doc's rule, `persisted_record(store)` no longer holds Q's clock after that session; under the current code it still does, and only a later `P.send(..)` followed by another gossip removes it. Assert whichever rule the owner selects.

### session-bookmark-22: `Bookmark::load` is documented as called once per `Peer`, but a failed store makes the crate call it again
- Where: src/bookmark.rs:96-102 (related: src/bookmark.rs:166-168, 308-313, 354-358, 244-246, 337-339; tests/bookmark_when.rs:9-11, 35-38)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read the path: `write`'s `Err` arm sets `self.inner = None` at 355; `ensure_loaded` calls `self.persist.read()` whenever `inner` is `None` at 309-311; `Persist::read` calls `Bookmark::load` at 166 and maps `Ok(None)` to `BTreeMap::new()` at 167-168)
- Seen by: prose, perfapi; refutation: confirmed (both); history: no rationale found (the "once" sentence and the failed-write reset were written in the same commit, b7fb409f; the contradiction is original)
- Owner-gated: no (the doc-correction policy sanctions fixing prose toward the code)

The public trait doc promises implementors that `load` runs once per `Peer`. After any failed `store` the record driver discards its in-memory record and the next `ensure_loaded` re-reads storage, so `load` runs again; a `load` future dropped mid-read leaves `inner` as `None` too. The private docs in the same file (244-246, 337-339) and `tests/bookmark_when.rs:35-38` state the re-read. An implementor that takes "once" literally, handing out a one-shot reader and returning `Ok(None)` afterwards, is read on the reload as "nothing has ever been written" (99), so the next `reclaim` or `record` pushes only the live alias and the next successful write persists a record without the stranded clocks: the identity loss the bookmark exists to prevent. A public contract clause the implementation contradicts is a correctness hazard for the implementor (Principles 4 and 5).

Evidence:

    96	    /// Open the stored record for reading, or `Ok(None)` if nothing is stored.
    97	    ///
    98	    /// Called once per [`Peer`](crate::Peer), lazily, before the first write.

    354	            Err(_) => {
    355	                self.inner = None;

Resolution: Rewrite the sentence to the actual schedule: "Called lazily before the first store, and again whenever the crate discards its in-memory record (after a failed `store`, or after a `load` that did not complete); `load` must be repeatable, and each call must return the current stored bytes." Add a test in tests/bookmark_when.rs (or tests/common/flaky.rs) that injects one store failure and asserts a second `Io::Read` on the next session. Acceptance: the `load` rustdoc states repeatability; the test pins the second read.

Construction: Implement `Bookmark` with a `Mutex<Option<Vec<u8>>>` whose `load` does `take()` (one-shot) and whose `store` fails once via an injected fault. Seed a peer, bookmark it with a record already holding a stranded clock for the same network, run one session with the fault armed, then one clean session; decode the stored frame and assert the stranded clock is still present. Under the current driver it is gone.

### session-bookmark-23: `Bookmark::store` lends a writer through an HRTB closure and a boxed future, but the crate hands it an already-materialized `Vec`
- Where: src/bookmark.rs:121-123 (related: src/bookmark.rs:33-39, 180-189, 210-217; src/lib.rs:333; tests/common/flaky.rs:206-220; tests/bookmark_when.rs:111-124; tests/bookmark_transmit_window.rs:115-128)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (read every `impl Bookmark for` in the tree: `NoBookmark` plus three test implementors, each of which copies the lent bytes into a fresh `Vec`; `Persist::write` encodes to `bytes` first and the closure body is `w.write_all(&bytes)`; `git show 80898f19` records the introducing rationale; `83edcd94` deleted `src/sync.rs`, the blocking twin)
- Seen by: perfapi; refutation: confirmed (with a correction to the proposed signature: a `&[u8]` borrowed from a temporary cannot ride the returned future in the combinator shape the comment at 158-160 protects; the parameter should be owned); history: deliberate but expired (the closure shape was chosen when two faces, async and blocking, shared one engine and was justified as bracketing the serialize step for atomicity; the blocking face is gone, and even at introduction the async `write` pre-encoded the bytes, so the bracket never enclosed serialization)
- Owner-gated: yes: public trait signature

`store` asks implementors to accept `F: for<'a> FnOnce(&'a mut (dyn AsyncWrite + Unpin + Send)) -> Serialized<'a> + Send` and to lend a writer into it; the public `Serialized` alias exists to name that boxed future, and its doc justifies the boxing only relative to the closure. The only caller first does `let bytes = format::encode(bookmarks);` and then passes a closure whose whole body writes that `Vec`; the frame's hash requires the whole covered region before the first byte, so a streaming encoder is not on the table. Every implementor in the tree copies the lent bytes into another `Vec`. Nothing outside the closure, the HRTB, the trait-object writer, and the alias needs any of them (Principle 3: circular justification). The cost is borne by every implementor, who must spell an HRTB over a trait object and box a future to store a byte slice; a byte parameter brackets atomicity identically (open temp, write, fsync, rename on `Ok`).

Evidence:

    121	    fn store<F>(&self, write: F) -> impl Future<Output = Result<(), Self::Error>> + Send
    122	    where
    123	        F: for<'a> FnOnce(&'a mut (dyn AsyncWrite + Unpin + Send)) -> Serialized<'a> + Send;

    184	        let bytes = format::encode(bookmarks);
    185	        Bookmark::store(self, move |w| {
    186	            Box::pin(async move { w.write_all(&bytes).await })
    187	        })

Resolution: Owner decision (public trait, pre-release). Proposed: `fn store(&self, frame: Vec<u8>) -> impl Future<Output = Result<(), Self::Error>> + Send;` (owned, so the future carries the bytes without a `B: Sync` requirement or a borrowed temporary), keeping the atomicity obligation in its doc verbatim; delete `Serialized` and its re-export; `Persist::write` becomes `Bookmark::store(self, format::encode(bookmarks)).map(|r| r.map_err(BookmarkIo::Io))`. `load` may stay reader-shaped (a file handle is natural there) or become byte-shaped for symmetry, since the crate `read_to_end`s anyway (171-176); decide once. Acceptance: `Serialized` is gone from lib.rs:333; `NoBookmark::store` and the three test implementors reduce to storing the bytes; bookmark suites pass with unchanged assertions.

### session-bookmark-24: The `Persist` trait has one implementor (the blanket) and exists only to be allowed past `private_bounds`
- Where: src/bookmark.rs:144-190 (related: src/bookmark.rs:301, src/peer/gossip.rs:345, 402-406)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep across src/, tests/, benches/, examples/: `Persist` appears only as the trait, its blanket impl, and bounds at bookmark.rs:301, gossip.rs:345, 406; the three test bookmarks implement the public `Bookmark`)
- Seen by: structure; refutation: confirmed; history: deliberate but expired (the trait dispatched over two I/O faces via a `Mode` type parameter with two non-overlapping blanket impls; 83edcd94 deleted the blocking face and the `Mode` parameter, leaving one implementor and a doc that states what the layer does, not why a trait is needed)
- Owner-gated: no

`Persist` is implemented exactly once, by `impl<B: Bookmark> Persist for B`. No test double implements it, so its only observable effect is forcing `#[allow(private_bounds)]` with an apology comment on `impl<T, B: Persist> Peer<T, B>` and a `B: Persist` bound on `bookmark_inner`. Steelman: it names the decoded layer and would admit a byte-free test double; neither payoff is realized. What a thing does is not why it should exist (Principle 3).

Evidence:

    144	pub(crate) trait Persist: BookmarkError {

    161	impl<B: Bookmark> Persist for B {

    405	#[allow(private_bounds)]
    406	impl<T, B: Persist> Peer<T, B> {

Resolution: Replace the trait with two free fns in bookmark.rs, `read_record<B: Bookmark>(&B) -> impl Future<Output = Result<Record, BookmarkIo<B::Error>>> + Send` and `write_record<B: Bookmark>(&B, &Record) -> impl Future<..> + Send`, keeping the combinator shape (the `B: Sync` comment at 158-160 still applies); bound `Bookmarked<B: Bookmark>`, `impl<T, B: Bookmark> Peer<T, B>`, and `bookmark_inner<B: Bookmark>`; delete the `#[allow(private_bounds)]` and its comment. If finding 23 lands, the write fn shrinks further. Acceptance: `grep -rn Persist src` hits only prose (the word at gossip.rs:876 and bootstrap.rs:185); `grep -rn private_bounds src` is empty; `just gate` clean.

### session-bookmark-25: `Bookmarked`'s loaded state is a three-field invariant proven by three `expect`s instead of one `Option<Loaded>`
- Where: src/bookmark.rs:238-269 (related: src/bookmark.rs:343-361, 372, 419, 437; src/peer/gossip.rs:677-715)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `expect("loaded before mutation")` at 372, 419, 437; `write`'s `Err` arm resets `inner`, `staged`, and `last` separately at 354-358)
- Seen by: structure; refutation: confirmed (the `Loaded` shape compiles by reading: `ensure_loaded(&mut self) -> Result<&mut Loaded, _>`, the `send_if_modified` closure captures the `&mut Loaded`, and the borrow ends before `bookmark.write().await`); history: no rationale found (`last` and `staged` arrived at different times for stated reasons; nothing records why the three `Option`s stay separate)
- Owner-gated: no

`inner: Option<Record>` carries the unloaded/loaded distinction while `staged` and `last` must be reset in lockstep with it on a failed write; `slice`, `record`, and `reclaim` each re-prove "loaded before mutation" with an `expect`, and every method doc restates the call order ("after `ensure_loaded`"). Moving `record`, `staged`, and `last` into one `Loaded` struct behind a single `Option`, with `ensure_loaded` returning `&mut Loaded` and the mutators (plus `is_current`) living on it, makes the order compiler-checked, deletes the three panics, and turns the three-way reset into one assignment that cannot be done partially. Types-first: the compiler catches errors, not humans reading carefully.

Evidence:

    247	    inner: Option<BTreeMap<Network, Vec<Clock>>>,

    259	    staged: Option<(Party, Version)>,

    268	    last: Option<(Party, Version)>,

    354	            Err(_) => {
    355	                self.inner = None;
    356	                self.staged = None;
    357	                self.last = None;
    358	            }

Resolution: `struct Loaded { record: BTreeMap<Network, Vec<Clock>>, staged: Option<(Party, Version)>, last: Option<(Party, Version)> }`; `Bookmarked<B> { persist: B, state: Option<Loaded> }`; `ensure_loaded(&mut self) -> Result<&mut Loaded, ..>`; `reclaim`, `slice`, `record`, `is_current` become `Loaded` methods; `write(&mut self)` matches `self.state` once (`None` stays `Ok(())`) and sets `self.state = None` on `Err`. Callers: `let loaded = bookmark.ensure_loaded().await?; ...; bookmark.write().await`. Acceptance: `grep -c 'expect("loaded before mutation")' src/bookmark.rs` is 0; `write`'s `Err` arm is a single assignment; the "after `ensure_loaded`" sentences are deleted as redundant; `tests/bookmark_when.rs` and `tests/bookmark_transmit_window.rs` green.

### session-bookmark-26: `is_current`'s doc says "exactly" while the code compares own-party projections, and its inline comment states only half the recording condition
- Where: src/bookmark.rs:282-298 (related: src/peer/gossip.rs:528-532, src/peer/gossip.rs:650-656, crates/before/src/version.rs:1703-1711, tests/bookmark_when.rs:12-28)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `impl<'a> Div<&'a Party> for &'a Version` at crates/before/src/version.rs:1703-1711, whose output is `OwnVersion`, the own-party projection; compared the doc text with the expression)
- Seen by: structure, prose; refutation: reframed (the comment is incomplete, omitting the party-changed arm, not inverted; severity lowered because tests/bookmark_when.rs:12-28 states the predicate correctly one grep away); history: deliberate but expired (the summary was accurate when the predicate was `v == version`; Finch's 9cf90dfb changed it to projections and added the load-bearing inline comment without touching the summary; gossip.rs:528-532 predates the change)
- Owner-gated: no

The doc says the token matches when `(party, version)` is "exactly what the last update persisted"; the code compares `v / p == version / p`, the versions' projections onto the recorded party, so an advance confined to other parties' identity space counts as current. That is the load-bearing design decision (persist before gossiping own events only) and it lives in a code comment inside the closure rather than in the contract. The inline comment states the recording condition as "party is the same ... and the projections are not equal", which is the version-advance half only; the code records when the party differs or the projections differ. `bookmark_update`'s doc (gossip.rs:528-532) names both halves but leaves "the version advancing on new content" unqualified by own-party. This predicate gates the durability guarantee the bookmark exists for (gossip.rs:650-656 argues safety from it), so a maintainer needs the precise statement at the method.

Evidence:

    282	    /// Whether `(party, version)` is exactly what the last update persisted, so
    283	    /// re-recording it would be a no-op. The suppression test for
    284	    /// [`update`](crate::Peer::bookmark_update).

    287	            // We only need to record the bookmark when our party is the same as
    288	            // the last time we recorded, and the two versions *quotiented by
    289	            // our current party* are not equal, because we're trying to ensure

    296	            p == party && v / p == version / p

Resolution: State it positively at the method: "Current when the party is unchanged and the version's projection onto that party is unchanged. Only own events must be durably covered before they cross the wire; an advance confined to other parties' identity space changes nothing the record must dominate, so it does not defeat suppression." Rewrite the inline comment to describe what returns true (or delete it as redundant with the doc), and qualify "the version advancing on new content" in `bookmark_update`'s doc with "in this party's own identity space". Acceptance: doc and comment read as the same boolean the expression computes; a reader of the rustdoc alone can predict the answer for a version that advanced only in a foreign party's space.

### session-bookmark-27: `Bookmarked::slice` and the struct doc claim a `watch` critical section its only caller deliberately omits
- Where: src/bookmark.rs:363-370 (related: src/bookmark.rs:228-231, src/peer/gossip.rs:561-571)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '\.slice(' src/`, excluding `as_slice`, returns exactly one call site, gossip.rs:569, inside `bookmark_donate`, whose body holds only the bookmark mutex)
- Seen by: prose; refutation: confirmed, severity lowered (private prose, single caller, correct statement already at that caller, no lock-order hazard in code); history: no rationale found (the slice doc and the contradicting caller comment were written in the same commit; the false statement is original)
- Owner-gated: no

`slice`'s doc says it runs "inside the caller's `watch` critical section (so it moves with the party leaving `Inner`)", and the struct doc groups `reclaim` and `slice` under "a brief `watch` critical section nested inside the mutex". The single caller takes no `watch` section and says why: the party has already left `Inner`. Lock-discipline prose is where a false statement costs the most (Principle 5).

Evidence:

    366	    /// The synchronous half of donation, run inside the caller's `watch`
    367	    /// critical section (so it moves with the party leaving `Inner`); the

    562	    /// and persist. The party has already left `Inner` (forked off or taken
    563	    /// whole), so this needs no `watch` critical section.

Resolution: Re-state `slice`'s placement: "Runs under the bookmark mutex, after the donated party has already been removed from `Inner` (in the session's speculative critical section) and before it is sent; the caller writes afterwards." Amend the struct doc at 228-231 so only `reclaim` is described as nested inside a `watch` section. Acceptance: no doc claims `slice` runs under `watch`.

### session-bookmark-28: Hand-maintained format-version numbers in prose and a ghost reference to prior frame shapes
- Where: src/bookmark/format.rs:61-69 (related: src/bookmark/format/tests.rs:171-173, 190-191)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read; the `FormatError::VersionMismatch` message at 207-209 already interpolates the constant, the pattern the prose should follow)
- Seen by: prose; refutation: confirmed; history: no rationale found (the prose was rewritten at both bumps, 35572cfc and 4f18c347, which is exactly the maintenance cost the finding names)
- Owner-gated: no

The constant's doc opens "Version 5 is the fully CBOR-parseable frame", restating the constant's value in prose that rots on the next bump; format/tests.rs:190-191 hardcodes "0x18 0x05" in a comment beside code that derives the byte from the constant; format/tests.rs:171-173 says "the earlier frame shapes share no decoder with this one", a reference to formats no longer in the tree.

Evidence:

    63	/// Version 5 is the fully CBOR-parseable frame: the self-described tag, this

    171	/// Every earlier format version is strictly rejected: the earlier frame
    172	/// shapes share no decoder with this one, and there is deliberately no

Resolution: "The current version is the fully CBOR-parseable frame: ..."; "the widened two-byte header `0x18 <version>`"; "no decoder exists for any lower version number". Acceptance: a bump of `BOOKMARK_FORMAT_VERSION` requires no prose edit in format.rs or its tests.

### session-bookmark-29: `frame_as` builds the hash-covered region in a scratch `Vec` and copies it into the output, where `unframe` hashes slices in place
- Where: src/bookmark/format.rs:241-256 (related: src/bookmark/format.rs:370-373, 16-21)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read)
- Seen by: perfapi; refutation: confirmed; history: no rationale found
- Owner-gated: no

The encoder writes the version item and payload item into `covered`, digests it, then copies `covered` piecewise into `out` around the digest: two copies of the payload per frame. `unframe` (370-373) expresses the same coverage rule as two slice updates on the frame bytes. Writing `out` once with a reserved digest slot and hashing the two slices of `out` deletes one copy and, more usefully, makes the encoder spell the coverage rule the way the decoder and the module doc do, so a reviewer checks one rule. Per bookmark write, so the payoff is legibility; the round-trip proptest and both pins hold the bytes fixed.

Evidence:

    249	    let mut out = Vec::with_capacity(SELF_DESCRIBED_HEAD.len() + 1 + 2 + HASH_LEN + covered.len());
    250	    out.extend_from_slice(&SELF_DESCRIBED_HEAD);
    251	    out.push(FRAME_ARRAY);
    252	    out.extend_from_slice(&covered[..version_item_len]);
    253	    out.extend_from_slice(&INTEGRITY_HEAD);
    254	    out.extend_from_slice(&hash);
    255	    out.extend_from_slice(&covered[version_item_len..]);

Resolution: Build `out` directly, record `version_start`/`version_end`/`payload_start`, reserve `HASH_LEN` bytes after `INTEGRITY_HEAD`, hash `out[version_start..version_end]` and `out[payload_start..]`, and `copy_from_slice` the digest into the slot. Acceptance: `framing_round_trips`, the corruption and truncation proptests, and both `insta` pins pass byte-identically.

### session-bookmark-30: `unframe` pre-checks the payload bound that `Reader::take` already enforces, then `expect`s it; two magic numbers are untied from their constants
- Where: src/bookmark/format.rs:359-363 (related: src/bookmark/format.rs:282-293, 75, 78, 241; src/tree/mirror/cbor.rs:77)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read `Reader::take` at 282-293: `checked_add(n).filter(|&end| end <= self.bytes.len())` reports `Truncated { len: self.bytes.len() }` on any overrun; `MAX_HEAD_LEN: usize = 9` at cbor.rs:77)
- Seen by: structure; refutation: confirmed (equivalent on every pointer width: a `declared` above `usize::MAX` exceeds any slice length); history: no rationale found (introduced together in 35572cfc)
- Owner-gated: no

`unframe` re-implements the truncation check by hand to justify an `as` cast and an `expect("length checked")`. `usize::try_from(declared).map_err(|_| FormatError::Truncated { len: bytes.len() })?` followed by `reader.take(declared)?` is behaviorally identical, including on 32-bit targets, with no panic site; the `Reader` type exists so every byte is "compared, parsed, hashed, or payload" under one truncation rule, and duplicating that rule at one call site undercuts the totality argument the type carries. Nearby, `INTEGRITY_HEAD = [0x58, 0x20]` spells `HASH_LEN` (32) as `0x20` with nothing tying the two, and `Vec::with_capacity(9 + 2 + 9 + payload.len())` spells `cbor::MAX_HEAD_LEN` twice as `9`.

Evidence:

    359	    let declared = reader.head(MAJOR_BSTR, FrameDefect::PayloadByteString)?;
    360	    if declared > (bytes.len() - reader.at) as u64 {
    361	        return Err(FormatError::Truncated { len: bytes.len() });
    362	    }
    363	    let payload = reader.take(declared as usize).expect("length checked");

    75	const INTEGRITY_HEAD: [u8; 2] = [0x58, 0x20];

    241	    let mut covered = Vec::with_capacity(9 + 2 + 9 + payload.len());

Resolution: `let declared = usize::try_from(declared).map_err(|_| FormatError::Truncated { len: bytes.len() })?; let payload = reader.take(declared)?;`. `const INTEGRITY_HEAD: [u8; 2] = [0x58, HASH_LEN as u8];` with a `const { assert!(HASH_LEN < 256) }` beside it. Use `cbor::MAX_HEAD_LEN` in the capacity arithmetic. Acceptance: no `expect` in `unframe`; `truncation_at_every_prefix_is_rejected` and `framing_round_trips` green; `grep -n '0x20\|9 + 2 + 9' src/bookmark/format.rs` empty.

### session-bookmark-31: The single-byte corruption proptest samples one mask and never the empty payload, narrower than the module doc's claim
- Where: src/bookmark/format/tests.rs:67-77 (related: src/bookmark/format/tests.rs:1-3, src/bookmark/format.rs:342-378)
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (read; by reading `unframe` every byte is compared, head-parsed, hashed, or payload, so any nonzero XOR is rejected today; the gap is the oracle's width)
- Seen by: correctness; refutation: confirmed; history: no rationale found (mask and floor original, carried unchanged through the format rewrite)
- Owner-gated: no

The module doc claims the frame "rejects every single-byte corruption"; the proptest applies only `^= 0xff` and draws payloads of length 1..64, so an empty payload's frame (payload head `0x40`) is never corrupted and a single-bit flip is never tried. Property tests should state the family the claim is about: any nonzero XOR at any offset of any frame.

Evidence:

    71	    fn any_single_byte_corruption_is_rejected(
    72	        payload in prop::collection::vec(any::<u8>(), 1..64),
    73	        index: prop::sample::Index,
    74	    ) {
    75	        let mut framed = frame(&payload);
    76	        let i = index.index(framed.len());
    77	        framed[i] ^= 0xff;

Resolution: Draw `mask in 1u8..=255` and payload `0..64`; apply `framed[i] ^= mask`. Acceptance: the widened proptest passes and its doc matches the module doc's claim.

### session-bookmark-32: Two format tests rebuild the frame by hand, each a copy of `frame_as` with one spelling widened
- Where: src/bookmark/format/tests.rs:189-214 (related: src/bookmark/format/tests.rs:273-300, src/bookmark/format.rs:237-257)
- Class / severity / confidence: simplification / nit / medium
- Provenance: assessed (read; the six `framed` lines at 201-206 and 287-292 are identical)
- Seen by: structure; refutation: confirmed (keep the helper test-side; production must not depend on a test helper); history: no rationale found
- Owner-gated: no

`non_canonical_version_spelling_is_rejected` and `non_canonical_payload_spelling_is_rejected` each reproduce `frame_as`'s body to widen one head, so a reader must diff fourteen lines against production code to find the one widened byte. A test-side helper `frame_spelled(version_item: &[u8], payload_head: &[u8], payload: &[u8]) -> Vec<u8>` would make the single deviation the only visible line in each.

Evidence:

    201	    let mut framed = SELF_DESCRIBED_HEAD.to_vec();
    202	    framed.push(FRAME_ARRAY);
    203	    framed.extend_from_slice(&covered[..version_item_len]);
    204	    framed.extend_from_slice(&INTEGRITY_HEAD);
    205	    framed.extend_from_slice(&hash);
    206	    framed.extend_from_slice(&covered[version_item_len..]);

Resolution: Add the helper in tests.rs; both tests become the two spelled heads plus the assertion. Acceptance: each of the two tests is under ten lines; both still fail if the corresponding `FrameDefect` arm is removed.

### session-bookmark-33: The greeting field list omits the payload depth limit
- Where: src/reconciliation.rs:53-57 (related: src/tree/mirror/streaming/message.rs:62-118, src/peer/gossip.rs:1419-1429)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `pub` fields in `Greeting`: `version`, `set_len`, `max_version_bytes`, `target_message_size`, `payload_depth_limit` (line 115), `listing`)
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the list was complete when written; the commit that added the field, 71de90c1, updated the codec and its tests but not this page)
- Owner-gated: no

The page lists five greeting fields; the struct carries six. The omitted one is user-visible: `Peer::payload_depth_limit` is a public setting and `Error::PayloadDepthMismatch` a public error whose cause is the greeting comparison, so the list that should explain that error omits its field. Line 55 also runs far past the paragraph's measure.

Evidence:

    53	//! After a fixed transport preamble, a session opens with a *greeting*:
    54	//! each side sends its version, its live-message count, its version-size
    55	//! bound, its message-size target, and its root's child listing. Equal versions mean identical

Resolution: Add "its payload depth limit" with one clause ("the two limits must match, or the session fails before any descent") and re-wrap. Acceptance: the list names every field of `Greeting`; no line exceeds the surrounding measure.

### session-bookmark-34: The digest-width section carries an adversary work-factor sentence that lost its "off-model" label
- Where: src/reconciliation.rs:164-177 (related: src/reconciliation.rs:157-162; AGENTS.md hard rule on adversary economics)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git show 2d1e6ea5 -- src/reconciliation.rs` introduced the sentence under an "Off-model note:" heading, and its commit message records "adversary-economics arguments are demoted to explicit off-model notes per the model of record"; 9c73d7b4 replaced that text with the current unlabeled form)
- Seen by: prose; refutation: reframed (the acceptance at 157-162 is on-model; the supplementary sentence is what the rule forbids resting on); history: already known (owner ruling to keep it as an explicit off-model note; what drifted is the label and the framing)
- Owner-gated: no for restoring the recorded framing; deleting the sentence would reverse the ruling and is the owner's call

The page prices the width on the on-model accident bound (157-162), then adds that "against any such actor, the 24-byte width keeps the offline birthday floor at 2⁹⁶ evaluations". The recorded ruling was to keep that as an explicit off-model note; the current text reads as a load-bearing argument, and AGENTS.md's hard rule says no pricing argument may rest on adversary economics. A reader meeting the paragraph reasonably infers the width was partly chosen for collision resistance against an actor.

Evidence:

    170	//! versions get created (an actor steering gossip schedules steers the
    171	//! version set); against any such actor, the 24-byte width keeps the
    172	//! offline birthday floor at 2⁹⁶ evaluations, an unconditional bound that
    173	//! rests on no premise about capabilities. Hostile *peers* remain

Resolution: Restore the ruling's framing: keep the structural claim (message bytes contribute zero bits to any compared digest, a property of the construction) and mark the birthday-floor sentence explicitly as an off-model aside that is not part of the acceptance ("Off-model note: ..."), or move it after the hostile-peers sentence so the paragraph's argument visibly ends at the accident bound. Acceptance: the section's acceptance rests on the per-comparison accident bound alone, and any adversary sentence is labeled as outside the model.

### session-bookmark-35: Stream-count arithmetic contradicts `STREAM_COUNT`'s own doc and the same page
- Where: src/reconciliation.rs:201-205 (related: src/reconciliation.rs:181-184, src/link.rs:161-169)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read src/link.rs:161-169: `STREAM_COUNT: usize = 17` is "Logical data streams a session may open in one direction ... (per direction, plus the control stream)", derived as "the descent's 32 tree heights at a two-height stride per stream, plus the shared opening stream: `ceil(32 / 2) + 1 = 17`"; `git show 77334965 -- src/reconciliation.rs` shows this text replacing a derivation that matched link.rs)
- Seen by: prose; refutation: confirmed; history: no rationale found (the wrong derivation is Finch's own hand edit, 77334965 "Manual doc edits", replacing a correct one; no ruling accompanies it, so it reads as a misderivation)
- Owner-gated: no (a factual correction toward the code, sanctioned by the doc-correction policy; the wording being replaced is Finch's, so it is reported here rather than assumed)

The page says the maximum is 17 streams made of "16 data streams plus a control stream". `STREAM_COUNT` defines 17 as data streams per direction with the control stream additional; the +1 is the shared opening data stream, not the control stream. The same page at 183-184 says data streams are "at most [`STREAM_COUNT`] per direction", so the page disagrees with itself. A transport implementor sizing a pool from 201-205 under-provisions by one data stream per direction. The literals 16 and 17 are also hand-maintained restatements of a constant the prose can cite by name (Principle 5).

Evidence:

    201	//! In *theory*, the maximum number of streams needed on either side of the link
    202	//! is 17, though in practice, far fewer will ever be needed. Why 17? A 32-byte
    203	//! key gives the descent 32 levels; the schedule of traversal asks each side to
    204	//! hop down the tree by 2 levels at a time, so at most 16 data streams plus a
    205	//! control stream are ever needed.

Resolution: "A 32-byte address gives the descent 32 heights; each reply phase descends two, and the opening question rides its own stream, so a side needs at most [`STREAM_COUNT`] data streams per direction, plus the persistent control stream; in practice far fewer are opened." Drop the literal 16 and 17. Acceptance: the page's two statements agree with each other and with link.rs:161-169; no literal stream count remains in the prose.

### session-bookmark-36: Public explanation names `Tree::join`, unreachable from the API
- Where: src/reconciliation.rs:246-249 (related: src/lib.rs:322, 348-349; AGENTS.md orientation)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: lib.rs:322 `mod tree;` is private and lib.rs:348-349 re-export only `MERKLE_HASH_LEN` and `SessionStats` from it; lib.rs:315 `pub mod reconciliation`)
- Seen by: prose; refutation: confirmed (a plain code span, not a broken link, but a user cannot find the item); history: no rationale found (fresh authorship by 368da2a5 beyond what the retirement plan asked for; AGENTS.md already places the oracle fact at the streaming module doc)
- Owner-gated: no

`Tree` is not exported, so a user of this page cannot find `Tree::join`. The sentence is maintainer content (the test oracle) and already lives where the maintainer looks. Public rustdoc names nothing the API does not reach.

Evidence:

    246	//! case. [`Protocol::V2`](crate::Protocol::V2) instead runs the descent
    247	//! using the bounded-memory streaming approach described above; its
    248	//! behavioral oracle in the test suite is the in-memory merge
    249	//! (`Tree::join`), which honors deletions through the same filter.

Resolution: End the paragraph at "described above"; leave the oracle statement to the streaming module doc and AGENTS.md. Acceptance: `reconciliation.rs` names no item absent from the public surface.

### session-bookmark-37: `Observer::session`'s "before the session's first byte crosses the wire" is not met for remote-led `gossip_when` sessions
- Where: src/observe.rs:71-73 (related: src/peer/gossip.rs:961-970, src/peer/gossip.rs:622, src/tree/mirror/handshake.rs:319-335)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: the driver's idle select reads the remote preamble into the staging buffer at gossip.rs:967, `drive.staged.fill(&mut *drive.read)`, before `gossip_inner` calls `self.observe.begin(kind)` at 622; delivery is intact because `handshake::preamble` reports the validated item from `staged.received()` at handshake.rs:334, after `begin`)
- Seen by: correctness; refutation: confirmed; history: no rationale found (written with the hook, after the driver already staged remote bytes before entering a session)
- Owner-gated: no

For a remote-led session, up to the whole preamble has been read from the wire before the observer is asked. The item-level coverage contract holds (the validated preamble is delivered whole to the control-received handler), so the cost is a misleading clause, not lost data; the clause is inherently about this side's writes.

Evidence:

    71	    /// Called once per session, before the session's first byte
    72	    /// crosses the wire. `session` identifies it; the returned
    73	    /// handler's lifetime is the session's.

Resolution: "Called once per session, before this side writes its first byte and before any item of the session is delivered to a handler." Acceptance: the clause reads true for one-shot and driver-led sessions; `tests/observe.rs`'s byte-for-byte mirror property passes unchanged.

### session-bookmark-38: `SessionObserver` has no end-of-session hook carrying the outcome; `Drop` is the only end signal and it cannot say whether the session converged or failed
- Where: src/observe.rs:82-105 (related: src/observe.rs:39-44, 71-74; src/peer/gossip.rs:897-904; crates/rumors-tracing/src/lib.rs:175-182)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: verified (read `SessionObserver` at 82-105: methods `elected` and `stream` only; `rumors-tracing`'s `SessionAdapter` holds a `Span` and grep finds no `impl Drop` in that crate, so it cannot record span status or the converged version)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (the design record specifies three levels plus `elected` and says nothing about a session-end signal)
- Owner-gated: yes: public trait addition

The module doc names tracing adapters and session recorders as the hook's consumers and admits an aborted session "may have observed fewer items than crossed the wire" (43-44), yet a consumer learns the end only by implementing `Drop`, which carries no outcome. The shipped adapter cannot set span status (converged version, `Error::Epilogue`, a protocol violation), and a recorder cannot tag a capture as complete versus aborted.

Evidence:

    82	pub trait SessionObserver: Send + Sync {

    92	    fn elected(&self, role: Role) {

    104	    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>>;

Resolution: Owner decision. Add a defaulted `fn finished(&self, outcome: ...)` to `SessionObserver`, invoked from `gossip_inner` and `bootstrap_erased` on every exit path via `SessionHandle`, with a rumors-blind payload (an outcome enum plus the public `Gossiped` or an error kind), following the existing `elected` default-method pattern. Acceptance: `rumors-tracing` sets span status from the hook; `tests/observe.rs` asserts the hook fires exactly once per session on both `Ok` and an injected failure.

### session-bookmark-39: Observer identity types derive `PartialEq, Eq` but not `Hash`/`Ord`, so they cannot key a per-stream table
- Where: src/observe.rs:129-131 (related: src/observe.rs:146-148, 159-161, 169-171, 189-190, 201-202; src/protocol.rs:13)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the six derive lines; `Protocol` at protocol.rs:13 derives `Clone, Copy, Debug, Default, Eq, PartialEq` only)
- Seen by: perfapi; refutation: reframed (five of six can derive for free; `SessionInfo` needs `Protocol` to derive `Hash`/`Ord` first, outside this partition); history: no rationale found
- Owner-gated: no

The module doc tells the consumer to do its own per-stream bookkeeping (29-33), and the natural store, `HashMap<StreamInfo, _>` or `BTreeMap<StreamInfo, _>`, is unavailable; a consumer must invent a surrogate key. Identity values are usable as keys in std (`SocketAddr`, `ThreadId`).

Evidence:

    129	#[non_exhaustive]
    130	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    131	pub struct SessionInfo {

    159	#[non_exhaustive]
    160	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    161	pub struct StreamInfo {

Resolution: Add `Hash, PartialOrd, Ord` to `SessionKind`, `StreamInfo`, `StreamId`, `Direction`, and `Role` now; add them to `SessionInfo` once `Protocol` (protocol.rs) derives them. Acceptance: `HashMap<StreamInfo, Vec<u8>>` compiles in tests/observe.rs.

### session-bookmark-40: A second `Peer::observe` or `Bootstrap::observe` replaces the first, and neither doc says so
- Where: src/observe.rs:226-230 (related: src/peer.rs:486-510, src/peer/bootstrap.rs:172-183, src/peer/bootstrap.rs:321-324)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read `attach`; grep of the `Peer::observe` and `Bootstrap::observe` docs for "replace", "again", "twice", "last" finds nothing)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (the design record describes a single handler attached at construction and never addresses repeated attach)
- Owner-gated: no for the doc sentence; yes for the accumulate-and-fan-out alternative

`Attachment::attach` overwrites `self.handler`. The public builders document what an observer sees and how it follows the peer, but not that attaching twice keeps only the last. A user who attaches `rumors_tracing::TracingObserver` and then a session recorder gets only the recorder, with no error and no doc to consult; composing by hand requires fan-out at all three levels.

Evidence:

    226	impl Attachment {
    227	    /// Attach `observer`; later sessions ask it for session handlers.
    228	    pub(crate) fn attach(&mut self, observer: Arc<dyn Observer>) {
    229	        self.handler = Some(observer);
    230	    }

Resolution: One sentence on both builders: "Attaching again replaces the earlier handler; to feed several consumers, attach one observer that fans out." Owner-gated alternative: accumulate into a `Vec<Arc<dyn Observer>>` and fan sessions out in `begin`. Acceptance: the sentence is present on both builders, or a test in tests/observe.rs attaches two observers and asserts both see the session.

### session-bookmark-41: Long qualified paths at use sites where an import (or an already-in-scope name) would do
- Where: src/observe.rs:368-380 (related: src/observe.rs:218-219; src/peer/gossip.rs:867, 1300-1301, 1415-1416; src/bookmark/format/tests.rs:329-331, 392, 451, 350, 409-410)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep over observe.rs and gossip.rs for `std::task::|std::pin::|std::fmt::|std::convert::|std::io::|std::cmp::|tokio::io::` outside `use` lines, and over format/tests.rs for `ciborium::value::Value::|crate::tags::`; `use ciborium::value::Value;` (format.rs:54) and `use crate::tags::CLOCK_TAG;` (format.rs:58) reach the test module through `use super::*;` at tests.rs:10)
- Seen by: structure; refutation: confirmed; history: no rationale found (observe.rs postdates the import-style sweep whose one stated exception, bare `Error` collisions, does not apply here)
- Owner-gated: no

`CaptureRead`'s `AsyncRead` impl spells `tokio::io::AsyncRead`, `std::pin::Pin`, `std::task::Context`, `std::task::Poll`, and `std::io::Result` in full; `Attachment`'s `Debug` impl spells `std::fmt::Debug/Formatter/Result`. gossip.rs: `std::cmp::Ordering::Greater`, `std::io::Error::new(std::io::ErrorKind::InvalidData`, `std::convert::Infallible` twice. format/tests.rs: `ciborium::value::Value::*` four times although `Value` is in scope, `crate::tags::CLOCK_TAG` twice likewise, and fn-local `use`s at 350 and 409-410. Imports over long qualified paths, except where the qualification informs (`io::Error`, `fmt::Result`); none of these is that case.

Evidence:

    368	impl<R> tokio::io::AsyncRead for CaptureRead<'_, R>
    369	where
    370	    R: tokio::io::AsyncRead + Unpin + ?Sized,
    371	{
    372	    fn poll_read(
    373	        self: std::pin::Pin<&mut Self>,
    374	        cx: &mut std::task::Context<'_>,
    375	        buf: &mut tokio::io::ReadBuf<'_>,
    376	    ) -> std::task::Poll<std::io::Result<()>> {

Resolution: Import `Pin`, `Context`, `Poll`, `AsyncRead`, `ReadBuf`, `fmt`, `io`, `Ordering`, `Infallible` at the top of each file (keeping `io::Error`/`fmt::Result` as the informing short forms); use the in-scope `Value` and `CLOCK_TAG` in the test and lift the fn-local `use`s. Acceptance: the two greps above return nothing.

### session-bookmark-42: Testdoc claims "whatever the session kind"; the body exercises one kind
- Where: src/observe/tests.rs:45-56
- Class / severity / confidence: test-quality / nit / high
- Provenance: assessed (read; `begin` returns at observe.rs:237-239 before touching `kind` when unattached, so the claim is true by reading but is not what the body checks)
- Seen by: prose; refutation: confirmed; history: no rationale found (original overclaim)
- Owner-gated: no

The doc quantifies over session kinds; the body calls `begin(SessionKind::Gossip)` once. A testdoc states what the body checks.

Evidence:

    45	/// An unattached peer's session handle is inert: nothing is created and
    46	/// every invocation is a no-op, whatever the session kind.

    50	    let handle = attachment.begin(SessionKind::Gossip);

Resolution: Iterate `[SessionKind::Bootstrap, SessionKind::Gossip, SessionKind::Retire]`, or delete "whatever the session kind". Acceptance: the doc's quantifier matches the body's coverage.

### session-bookmark-43: `Message`'s `# Panics` lists two serializing constructors; `try_new` serializes too
- Where: src/message.rs:36-43 (related: src/message.rs:357-364, 375)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: `try_new` at 359-364 calls `try_from_arc`, which calls `to_vec` at 375, which panics on a `Serialize` error at 285-290; `try_new`'s own doc at 357-358 defers to this section)
- Seen by: prose; refutation: confirmed; history: no rationale found (`try_new` was added before the prose-trimming pass that kept the two-item list)
- Owner-gated: no

The type-level hazard section is the contract of record for the serialize panic, and `try_new`'s doc points the reader here; the enumeration omits it. A hand-maintained list already out of date.

Evidence:

    38	/// Every payload value must serialize: methods that serialize
    39	/// ([`new`](Self::new), [`from_arc`](Self::from_arc)) panic if the

Resolution: Drop the enumeration: "every constructor that serializes panics if ...". Acceptance: every method whose body reaches `to_vec` is covered by the section's wording.

### session-bookmark-44: `pub struct Message` and eight `pub fn`s are unreachable outside the crate but documented in library-user voice; `try_new`'s doc misnames it as the send-path constructor
- Where: src/message.rs:47-51 (related: src/message.rs:320-364, 399-420, 465-528; src/lib.rs:310, 337; src/peer.rs:14; src/tree.rs:714-715; src/conformance.rs:18-19)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep: `mod message;` is private at lib.rs:310 and only `EncodeError` (lib.rs:337) plus `PayloadDepthLimit`/`DEFAULT_PAYLOAD_DEPTH_LIMIT` (peer.rs:14) are re-exported; `src/testing.rs` names no `Message`; `mod tree;` is private (lib.rs:322); every non-`tests.rs` caller of `Message::new` is under a `#[cfg(test)]` module (`tree::arb`, `conformance::backend`, `streaming::testing`, and the streaming `tests/` directories gated at streaming.rs:219, proxy.rs:49, codec.rs:242, adapter.rs:86); `try_new`, `from_slice`, `from_bytes`, `from_arc` have no callers outside test code; the production send path is `PayloadCodec::new` -> `serialize_payload` -> `Message::try_from_arc` at 213-221; no `unreachable_pub` lint is configured)
- Seen by: structure, perfapi; refutation: confirmed (both); history: deliberate but expired (`Message` was public until 94da12b5 purged the exports to restart the surface; 6e4b6eea noticed the module is private and chose documentation over visibility; no owner ruling on visibility is recorded)
- Owner-gated: yes: whether `Message` becomes public API or `pub(crate)`

The type and `new`, `try_new`, `from_slice`, `from_bytes`, `from_arc`, `arc`, `as_slice`, `bytes` are `pub` with docs written to a caller ("the caller's own `Arc<T>` allocation", a `# Panics` section pointing at the crate docs) that no library user can read, while `from_slice` says "Crate-internal rehydration" on a `pub fn`. Visibility should say what the compiler enforces, and documentation altitude is misapplied when the item has no library reader. Separately, `try_new`'s doc calls it "the constructor behind `Rumors::send`"; the send path calls `try_from_arc`, of which `try_new` is the owned-value wrapper.

Evidence:

    47	#[derive(Clone)]
    48	pub struct Message {
    49	    message: Arc<dyn Any + Send + Sync>,
    50	    serialized: Bytes,
    51	}

    346	    /// Creates an admission-checked `Message`: the constructor behind
    347	    /// [`Rumors::send`](crate::Rumors::send) and
    348	    /// [`Batch::send`](crate::Batch::send).

    310	mod message;

Resolution: If `Message` stays internal: `pub(crate)` the type and its methods, recast the docs at maintainer altitude, move `try_new`, `from_slice`, `from_bytes`, `from_arc` under `#[cfg(test)]` or into the test modules that use them, correct `try_new`'s doc to name `try_from_arc` as the send path, and add `#![warn(unreachable_pub)]` crate-wide (other partitions will show the same pattern). If `Message` is meant to become public API: re-export it deliberately from lib.rs so the docs have a real reader. Acceptance: either `unreachable_pub` is clean for message.rs, or `rumors::Message` appears in the public re-exports with its docs reviewed for that reader; `try_new`'s doc names `try_from_arc`.

### session-bookmark-45: Dated rationale on the default depth limit: a fleet upgrade with no prior release
- Where: src/message.rs:53-59
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (read `.agent-notes/2026-08-20-payload-depth-limit/payload-depth-limit.md:212-216`, which records the sentence as Finch's ruling with "the previous code" meaning in-repo history)
- Seen by: prose; refutation: confirmed (nit); history: deliberate and holds for the fact (the default equals the decoder's own bound); the positive rewording states the same fact without the temporal frame and is compatible with the ruling
- Owner-gated: no

The constant's doc justifies its value by "a fleet upgrading together sees no acceptance change on existing content", which presumes a prior shipped state; pre-release, "existing content" has no referent. The positive form is available and stronger: at the default, admission enforces nothing the decoder does not already enforce. Dated rationale at a declaration site is a ghost reference in disguise (Principle 5).

Evidence:

    55	/// Exactly the CBOR decoder's own default recursion bound, so a fleet
    56	/// upgrading together sees no acceptance change on existing content.

Resolution: "Exactly the CBOR decoder's own default recursion bound: at the default, admission enforces nothing the decoder would not already enforce." Keep the interop sentence. Acceptance: the doc's rationale refers to no upgrade or prior state.

### session-bookmark-46: `EncodeError` is not `#[non_exhaustive]` while every other public error taxonomy in the partition is
- Where: src/message.rs:116-117 (related: src/bookmark/format.rs:87-89, 127-129, 185-187; src/error.rs:61-63)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep: `FrameDefect`, `RecordDefect`, `FormatError`, and `Error` carry `#[non_exhaustive]`; `EncodeError` does not; every downstream match in tests/payload_depth.rs and tests/single_peer.rs uses `matches!`, and the rumors.rs doctest does not match on it, so adding the attribute breaks nothing in the tree)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (the crate's criterion for `non_exhaustive`, 1e458d69, predates `EncodeError` by a day and the enum landed without a recorded classification under it)
- Owner-gated: yes: changes downstream matching (cheap pre-release)

`EncodeError` is the admission-failure taxonomy a caller matches on; an added admission rule (a payload size ceiling is a plausible one) would be a breaking change. Closed outcome sets in the partition (`Retire`, `Led`, `Gossip`, `Direction`, `Role`, `BookmarkIo`) reasonably stay exhaustive; `EncodeError` is a taxonomy that grows as enforcement grows, the class the crate's own criterion marks `non_exhaustive`.

Evidence:

    116	#[derive(Debug, thiserror::Error)]
    117	pub enum EncodeError {

    87	#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
    88	#[non_exhaustive]
    89	pub enum FrameDefect {

Resolution: Add `#[non_exhaustive]` to `EncodeError`. Acceptance: the attribute is present; the doctest at rumors.rs and the `matches!` tests still compile.

### session-bookmark-47: `try_from_arc` erases a `T` to `Arc<dyn Any>` and downcasts it back (with a panic arm) to route through the fn pointer; `from_slice` duplicates `from_bytes`
- Where: src/message.rs:375-392 (related: src/message.rs:300-318, 440-463, 411-420, 472-482, 209-227)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read: `Message::deserializer`'s inner fn at 455-461 is `decode_exact::<T>(bytes, limit)?` then `Ok(Arc::new(message))`; `deserializer` has two callers, 224 and 380; `from_slice` at 411-420 is `from_bytes` at 472-482 plus `Bytes::copy_from_slice`)
- Seen by: structure, perfapi; refutation: confirmed (both); history: deliberate and holds for the intent ("admission is the ingress computation", 6e4b6eea), which calling `decode_exact::<T>` directly preserves since the deserializer only wraps it in `Arc::new`; nothing pins fn-pointer identity
- Owner-gated: no

Send-side admission calls the fn pointer, receives `Arc<dyn Any + Send + Sync>`, downcasts to `Arc<T>` with `unwrap_or_else(|_| panic!(..))`, dereferences for `Eq`, and drops it. The pointer is `decode_exact::<T>` plus `Arc::new`, so calling `decode_exact::<T>` directly is the same computation and yields a typed `T`, removing one `ArcInner<T>` allocation, one `TypeId` check, and one unreachable panic per admitted send (denominated per `Rumors::send`/`Batch::send`; strict deletion of redundant work, fixed sign). The codec's two halves then become symmetric (both inner fns of `PayloadCodec::new`) instead of one living on `Message`. Separately, `from_slice` can delegate to `from_bytes`.

Evidence:

    380	        let decoded = match Self::deserializer::<T>()(&serialized, limit) {
    381	            Ok(decoded) => decoded,
    382	            Err(PayloadDecodeError::Depth(limit)) => return Err(EncodeError::Depth { limit }),
    383	            Err(PayloadDecodeError::Io(source)) => return Err(EncodeError::Roundtrip(source)),
    384	        };
    385	        // Faithfulness: what a receiver reads must be the value that was
    386	        // sent, judged by the payload type's own equality.
    387	        let decoded: Arc<T> = decoded
    388	            .downcast()
    389	            .unwrap_or_else(|_| panic!("a payload decodes to its own type"));

Resolution: In `try_from_arc`: `let decoded: T = match decode_exact::<T>(&serialized, limit) { Ok(v) => v, Err(PayloadDecodeError::Depth(limit)) => return Err(EncodeError::Depth { limit }), Err(PayloadDecodeError::Io(source)) => return Err(EncodeError::Roundtrip(source)) }; if decoded != *arc { return Err(EncodeError::Unfaithful) }`, and reword the comment: `decode_exact` is the one parse ingress runs (the deserializer wraps it in the `Arc`). Move `deserializer`'s inner fn into `PayloadCodec::new` beside `serialize_payload` and delete `Message::deserializer`. Make `from_slice` call `Self::from_bytes::<T>(Bytes::copy_from_slice(bytes), limit)`. Acceptance: no `downcast` in `try_from_arc`; `Message::deserializer` is gone; message/tests.rs (including `from_bytes_matches_from_slice`, `try_new_admits_exactly_the_limit`, `try_new_prices_an_enums_own_decode`, `a_type_that_cannot_read_its_own_output_fails_admission`, `codec_serializes_through_the_carried_limit`) green unchanged. Optional meter in the style of tests/encode_alloc.rs: allocations of `Message::try_new(())` drop by one.

## Positives

- The monomorphization boundary is placed and priced in its own docs: `DynRead`'s doc (gossip.rs:56-70) states the cost model (one vtable call per stream open and per poll beneath frame buffering), and `Reconciliation::reconcile`'s doc (1114-1126) explains why both the boxed `dyn` coercion and `inline(never)` are needed, so neither can be removed by accident.
- `PartyGuard` and the donation ordering (gossip.rs:765-802) make identity duplication structurally impossible: the party is sliced out of the durable record while still held by the guard, taken out of the guard only immediately before `party::send`, and every failure after that point reports `Intent::Retire`; the drop path handles both the fork and the taken-whole case, and the comment explains why the recovery join cannot live inside a `debug_assert!`.
- `Bookmarked`'s staged-then-committed suppression token (bookmark.rs:248-268, 343-361) is a careful cancel-safety design: the token moves only on `write`'s `Ok`, a failed write resets record, stage, and token so the next use reloads the disk, and `# Cancel safety` and `# Errors` sections appear on a private method. `tests/bookmark_transmit_window.rs` constructs the in-flight-write race deterministically.
- `Bookmark::store`'s contract (bookmark.rs:104-120) names atomicity as a safety obligation the crate cannot check and spells out the consequence (re-issuing causal coordinates the network durably holds): the right altitude for a caller-implemented trait.
- The frame format (format.rs) is fully CBOR-parseable, deterministic, and totally shape-checked, with three error enums that each tell the reader what to conclude (corruption, logic error, foreign file). Its test suite is the standard the rest of the crate should be held to: every single-byte corruption, every truncation prefix with the exact `len`, trailing bytes, non-canonical spellings per head, version rejection unshadowed by the hash via the `frame_as` split (a test-design decision stated at the code, 231-236), a rumors-blind CBOR parse, and hex-first pins whose re-accept rules are stated on the tests themselves (tests.rs:465-496). `version_item_len()` derives an offset from the constant so a version bump cannot skew the flips, and says so.
- The epilogue mirrors the preamble as a concurrent `try_join` write-then-read, so it cannot deadlock at any positive transport capacity (pinned with `duplex(1)`), and it distinguishes a cut (`UnexpectedEof`) from a protocol violation (`InvalidData`), with the offending bytes named in hex in the error message.
- `observe.rs` keeps the wire hook rumors-blind (no protocol type in the signature, one whole CBOR item per call); the module doc's four named contract bullets (Ordering, Never block, Coverage, Cost) are the right altitude for a hook consumer, and the cost claim is true at every site (`CaptureRead` wraps the reader only when `observe.attached()`). `SessionInfo`'s doc explains a deliberate omission (no session number) with the alternative the reader would otherwise ask for.
- `message.rs` makes send-side admission literally the receiver's decode (`try_from_arc` runs `decode_exact` at the same limit), so the two verdicts cannot drift; the depth case stays typed end to end (`PayloadDecodeError::Depth` to `EncodeError::Depth`); every `expect`/`panic!` message in the file is a one-line proof; `PayloadCodec` is `Copy` (two fn pointers and a limit), so per-message wire decode is a direct call with no `dyn` dispatch; `PayloadDepthLimit` is a complete newtype with a saturating internal conversion whose bound is argued in one sentence.
- gossip.rs's two critical-section comments (645-670, 805-871) state the safety obligations and the three-term wake predicate with the reason for each term, price the frontier comparisons, and name the test suite that pins the no-loop property; `Retire`'s variant docs tell the caller exactly what survived and what to do with the link; `impl From<()> for Gossip` lets `rumors.changes()` plug straight into `gossip_when`.
- gossip/tests.rs drives the misdeclaring bootstrap claimant through the crate's own protocol machinery rather than hand-forged bytes (191-223), pinning the rejection on both sides that can face a claimant, with the provider's content, party, and link state asserted afterwards.
- No em-dash appears in any error, panic, or assert string in the partition (grep-verified).

## Open questions for Finch

- Is `Message` intended to become public API? If yes, re-export it deliberately and keep the constructors' docs at user altitude; if no, `pub(crate)` plus a crate-wide `unreachable_pub` warning closes the class (finding 44). My recommendation: `pub(crate)`, since `Snapshot` yields `Arc<T>` and nothing in the public surface needs the erased type.
- Should "V2" survive as the name of the current dialect in session prose (gossip.rs:51, 1276; tests.rs:6, 184), given the recorded ruling that the `Protocol` enum stays public as wire vocabulary? Recommendation: keep the enum and the wire-facing names, drop the qualifier from prose that describes behavior rather than the wire ("the epilogue", "the preamble"), and rename the two `v2_` tests now (finding 12).
- `Bookmark::store`: byte parameter or lent writer (finding 23)? And if bytes, should `load` become byte-shaped for symmetry, since the crate `read_to_end`s anyway? Recommendation: owned bytes for `store`; keep `load` reader-shaped for file-handle implementors, decided once.
- Reclaim timing (finding 21): re-word the public doc to the code's rule (a), or extend the persist gate so a hearsay-only advance that newly dominates a stranded identity triggers reclaim (b)? Recommendation: (a), which is consistent with the never-write-on-hearsay ruling `tests/bookmark_when.rs` pins; either way, add the test.
- bookmark.rs:456-464: `reclaim` retains overlapping clocks the fully grown party does not `covers`. In a well-formed universe I could not construct a stored clock whose party strictly exceeds the grown live party (a crashed incarnation's identity is never absorbed elsewhere; own aliases are kept in step by `slice`). Is this branch a deliberate safety net for the documented shared-bookmark misuse (84-91)? If so, the comment should say so; if not, it may be dissolvable. Recommendation: state the rationale at the site.
- "Seam" crate-wide (finding 5): keep as the crate's established term or replace with "boundary"/"layer" everywhere including `SessionStats`? Recommendation: replace, following the `mint` purge precedent, in one sweep.
- The birthday-floor sentence (finding 34): restore the "Off-model note" label per the recorded ruling, or delete it? Recommendation: restore the label; the accident bound already carries the acceptance.
- Do you want a session-end observer hook (finding 38) and an offline bookmark inspector (finding 20)? Both are small, both are public API. Recommendation: the hook yes (rumors-tracing needs it for span status); the inspector when an operator tool first needs it.
- Two reconciliation drivers (finding 15): unify into one `Reconciliation` with an admission parameter, or keep `bootstrap_reconcile` for the readability of the bootstrap path on its own? Recommendation: unify; the asymmetric `.stats(..)` is the first drift.
- `Bookmarked` has no sibling `tests.rs` (only `format/tests.rs`); its token state machine (staged/last/slice interplay) is exercised only through `Peer` in the integration suites. Do you want a direct property test of `Bookmarked` (reclaim/slice/record/write sequences against a model), or is public-API coverage the intended stance? Recommendation: a small model-based proptest in `src/bookmark/tests.rs`, since finding 25's refactor is exactly where such a test earns its keep.
- `PayloadDepthLimit::new(0)`: ciborium's recursion check fails when the counter is already zero, so a zero limit refuses every send with `EncodeError::Depth` and fails every non-converged session at ingress. Consistent with the stated semantics; should `new`'s doc name the degenerate value? Recommendation: one sentence.
- gossip.rs:710-711 returns `guarded.party.is_some()` from the pre-session `send_if_modified` (waking watchers when a party is taken or forked), while `bookmark_update`'s closure (551-552) argues a party-only change moves no observable frontier and owes no wake. Both are harmless under `Changes`' frontier compare; which rule is intended? Recommendation: adopt 551-552's rule at both sites and say so once.

## Dropped

- [39g] AGENTS.md pointer in format/tests.rs:490-492: owner-ratified cross-reference (66d782e5) placed so tamper sweeps find the sanctioned exception; deliberate and documented.
- [24] reconciliation.rs:246 `Protocol::V2` link and observe/tests.rs:58 "observed V2 session": accurate names of the live public variant (the test asserts `protocol: Protocol::V2`), not residue; folded into finding 12's owner-gated vocabulary question.
- [39i] reconciliation.rs:202 "Why 17?" and 214 "twiddle its thumbs": Finch's own register (77334965); taste, no cost named; the grammar item at 233-235 is kept in finding 14.
- [21] "inline comment inverts the predicate": overstated; the comment omits the party-changed arm rather than stating the complement; kept as "incomplete" in finding 26.
- [41] duplicate of finding 18 (epilogue marker docs and first-byte sweep).
- [45] duplicate of finding 22 (`load` once per Peer), whose construction is carried forward.
- [37], [56] PartyGuard items: duplicates of finding 17; [56]'s turbofish folded into finding 15.
- [35] duplicate of finding 19 (test helper braces and doc).
- [48] duplicate of finding 47 (`try_from_arc` round trip), framed as per-send cost; the cost statement is carried forward.
- [47] duplicate of finding 44 (`Message` visibility), whose `try_new` doc correction is carried forward.
- [23], [24] duplicates of finding 12 (V1 residue).
- [39b] duplicate of finding 2 (glued imports).
- [18] severity medium: lowered to low; the epilogue comparison is a whole-array `!=`, so the untested first byte cannot currently diverge.
- [20], [21] severity medium: lowered to low; private prose with the correct statement one grep away.
- Correctness lens open question on `!party.covers` retention, perfapi open question on `PayloadDepthLimit::new(0)`, and the 710-711 versus 551-552 wake rule: not findings (no contract breached); carried as open questions.
- perfapi's `SessionStats` "Two deliberate boundaries" count: outside this partition (src/tree/mirror/streaming/stats.rs); left for that partition's reviewer.
- perfapi's "no bench covers the send path with a non-unit payload": an instrument proposal, not a defect in this partition; noted under finding 47's optional meter.
