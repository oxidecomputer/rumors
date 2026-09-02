# Partition api-core: Public surface core: Peer, Rumors, observers, Batch, Snapshot, errors, tutorial

## Partition summary

This partition is the replica's public face. `Peer` (src/peer.rs) is the `!Clone` identity anchor: the `Network`, the window and run-budget settings, the `watch`-guarded `Inner { party, tree }`, the bookmark mutex, the type-erased payload codec, and the observation attachment. `Bootstrap` and `BookmarkedBootstrap` (src/peer/bootstrap.rs) are the type-state builders behind `Peer::bootstrap`, and `Joined` their outcome. `Rumors` (src/rumors.rs) is the cloneable working handle: a `Peer` plus an `Extant` token whose count lets `try_into_peer` reclaim the anchor; every operation forwards to a `pub(crate)` method on `Peer`. The three set observers, `UnorderedMessages`, `CausalMessages`, and `Changes` (src/rumors/), share one `Channel` type that materializes the `watch` wait as an owned boxed future. `Batch` (src/batch.rs) queues `Action`s and commits them in one `send_if_modified` critical section; `Snapshot` (src/snapshot.rs) is a `Network` plus a structure-shared `Tree<T>`. `Network` reserves its all-zero bootstrap sentinel structurally, `tags.rs` is the CBOR tag table, `Protocol` has one variant, `Error<B>` is a flat taxonomy generic only for its bookmark arm, and `tutorial.rs` is a docs-only page. I read all fifteen partition files in full with line numbers (4039 lines; src/peer/bootstrap/tests.rs, 146 lines, is the only test code), plus the out-of-partition context the findings depend on (src/peer/gossip.rs, src/tree.rs, src/message.rs, src/bookmark.rs, src/observe.rs, src/tree/mirror/handshake.rs, the cited tests, `before`'s `Version`/`Rank`/`Ranked`, and the vendored tokio 1.52.3, bytes 1.11.1, and thiserror-impl 2.0.18 sources at the versions Cargo.lock pins).

The code is in good shape. The `Peer`/`Rumors` XOR is argued at the types rather than in prose, the type-state builder lets each `join` declare only its own outcomes, `Batch` is closed by the borrow checker (two `compile_fail` doctests pin both escape routes), the observers state their checkpoint discipline at the fields and pin it at both boundaries on both faces, every `must_use` names the concrete loss, and the error module opens with a per-variant recovery table at exactly the altitude a caller needs. I checked the contract claims I could reach against the code and found them accurate, with the exceptions below.

The findings cluster in four places. First, two removal commits did not sweep their own remainders: the V1 retirement (368da2a5) left the vocabulary of "selecting" a `Protocol` in the error table, an error message string, and two method docs, plus a `Default` derive nothing calls; and the commit that made `Snapshot::iter` opaque (8dc0596ed) left a `pub use crate::tree::Iter` and an `IntoIterator` impl that name a type no public path reaches. Second, the `Bootstrap` page and its plumbing tests were written for a three-setting builder and not re-read when `observe` (40b1e96a) and `payload_depth_limit` (4356e197) landed. Third, `#[derive]` on generic wrappers imports bounds the representations never use, so `Error<B>` is not `std::error::Error` for a legal non-`Debug` bookmark and `Snapshot<T>` refuses to clone a non-`Clone` payload; the crate already states the counter-principle in two hand-written impls. Fourth, two small concurrency pieces carry more machinery than their invariants need: the three observers each re-spell the same two `Channel` transitions (and `Changes` implements its state machine twice), and `Extant` keeps two counters of one quantity. One verification gap stands out: the `claimed` CAS and the subscribe-before-shed ordering in `try_into_peer` are exercised by no committed test with two concurrent reuniters. The remainder is prose mechanics and small idiom, listed as nits.

Findings are numbered in path-then-line order. One finding (api-core-10) anchors in src/message.rs, outside this partition's file list; the perfapi lens filed it here because `Batch::send` is its only call site in the partition, and the message.rs partition's finalizer may hold a duplicate.

## Findings

### api-core-1: `Batch::redact_all` accepts only `&Version` items although callers usually hold owned versions
- Where: src/batch.rs:112-115 (related: src/rumors.rs:279-283, src/peer.rs:668-671)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: assessed (read)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (212c6914 introduced the signature without discussing the item type)
- Owner-gated: yes: widens a public signature

The bulk redaction entry takes `I: IntoIterator<Item = &'v Version>`, but the natural sources of versions to redact yield them owned: the observers' items are `(Version, Arc<T>)`, and a `Vec<Version>` moved into a task must be re-borrowed with `.iter()`. `Version::clone` is an O(1) refcount bump and `Path::for_leaf` needs only a borrow, so `Item: Borrow<Version>` (the `HashSet::remove<Q>` idiom) accepts both forms at no cost and keeps the existing `&evens` doctest compiling.

Evidence:

    112	    pub fn redact_all<'v, I>(&mut self, versions: I)
    113	    where
    114	        I: IntoIterator<Item = &'v Version>,
    115	    {

Resolution: On both `redact_all`s, `I: IntoIterator, I::Item: Borrow<Version>`, calling `self.redact(version.borrow())`. Acceptance: the rumors.rs:263-278 doctest passes unchanged, and a second example `rumors.redact_all(evens)` over an owned `Vec<Version>` compiles.

### api-core-2: `Batch::commit`'s no-party arm degrades to a silent drop in release builds
- Where: src/batch.rs:133-139 (related: src/peer/gossip.rs:696-711, src/peer/gossip.rs:825-828, src/peer/gossip.rs:1384-1404, src/peer.rs:177-180, .cargo/mutants.toml:15-24)
- Class / severity / confidence: correctness / low / medium
- Provenance: assessed (read)
- Seen by: correctness (structure and perfapi raised it as an open question); refutation: confirmed; history: no rationale found; the mutants policy of record points the other way
- Owner-gated: no

The arm is argued unreachable, and I agree with the argument: `Inner.party` is `None` only between `inner.party.take()` in `Peer::retire` (gossip.rs:696-702) and `PartyGuard::drop`'s restore (gossip.rs:1384-1404), and `retire` consumes the `Peer` while the `Peer`/`Rumors` XOR keeps any `Batch`-creating handle from coexisting with it. But the code expresses that proof as a debug-only assert followed by `return false`: were the arm ever reached, a closure that returned `Ok` would have its whole batch discarded with no error, no observer wake, and no panic. The doctrine's sanctioned response to programmer error is a panic carrying the proof; a silent no-op is the one behavior never sanctioned, and the crate's own mutants policy (step 2 of the disposition ladder) says a structurally necessary but unreachable branch asserts the impossibility at the site. tokio's `send_if_modified` (watch.rs:1180-1194) catches the closure's panic, drops the write lock, and resumes unwinding, so an `expect` cannot poison the channel.

Evidence:

    133	            // The party is present on every reachable handle: `retire`
    134	            // consumes the `Peer`, and the `Peer`/`Rumors` XOR keeps a
    135	            // retiring set's handles from coexisting with it.
    136	            let Some(party) = inner.party.as_ref() else {
    137	                debug_assert!(false, "no party to tick in a `Batch` commit");
    138	                return false;
    139	            };

Resolution: Replace the `let ... else` with `.expect("a Batch commits through a live Peer or Rumors handle; the Peer/Rumors XOR keeps one from coexisting with a retirement's in-flight party")`. The structural alternative, making `Inner.party` total by holding the in-flight retirement party elsewhere (it would also delete the `None => inner.party = Some(party)` arm at gossip.rs:825-828), is a larger design change; see the open questions. Acceptance: no `debug_assert!(false, ..)`-plus-fallback pattern remains in `Batch::commit`; the panic message states the XOR argument; `just test` stays green (the arm is unreachable, so no test changes).
Construction: In src/tests.rs, where `Inner`'s fields are `pub(crate)`: `let peer = Peer::<u64>::seed(); peer.inner.send_modify(|i| i.party = None); peer.send(1).unwrap(); assert_eq!(peer.snapshot().len(), 0);`. At HEAD this passes in release (the send reports `Ok` and commits nothing) and panics in debug; after the fix it panics in both.

### api-core-3: Paragraph rewrap residue in doc comments
- Where: src/error.rs:6 (related: src/peer.rs:332, src/peer.rs:334, src/lib.rs:187, src/tutorial.rs:112, src/rumors/changes.rs:18, src/peer/bootstrap/tests.rs:6, src/rumors.rs:255, src/batch.rs:49, src/batch.rs:55-56)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (awk `length()` on each cited line)
- Seen by: prose; refutation: confirmed; history: no rationale found (each line blames to a mechanical edit that did not rewrap)
- Owner-gated: no

Several doc paragraphs carry one over-long or one stub-short line mid-paragraph, the signature of an edited sentence left unwrapped (rustfmt does not rewrap comments). Measured widths: error.rs:6 is 105 columns, peer.rs:332 is 92, peer.rs:334 is 86, lib.rs:187 is 92, tutorial.rs:112 is 90, changes.rs:18 is 97, bootstrap/tests.rs:6 is 96; rumors.rs:255 is a 44-column line mid-paragraph, and batch.rs:49 is a 14-column line holding `peer's`.

Evidence:

    6	//! [`Error::Mirror`], for matching and bug reports. Every session `Err` poisons its link (discard it and

    48	    /// a receiver would reject or misread — one nesting deeper than the
    49	    /// peer's
    50	    /// [`payload_depth_limit`](crate::Peer::payload_depth_limit), one

Resolution: Rewrap the listed paragraphs to the surrounding width. Acceptance: no doc line in the partition exceeds the paragraph width except table rows and bare link-reference lines, and no mid-paragraph stub lines remain.

### api-core-4: Prose describes selecting a `Protocol`, an operation the API no longer offers
- Where: src/error.rs:14 (related: src/error.rs:76, src/error.rs:179, src/error.rs:201, src/protocol.rs:1, src/peer.rs:512, src/peer.rs:604; out of partition: src/tree/mirror/handshake.rs:180, src/tree/mirror/handshake.rs:204, src/peer/gossip.rs:723)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep: no `fn protocol` or `.protocol(` anywhere in src, tests, examples, or benches; `Protocol` has one variant at protocol.rs:15-18; the only public carriers are `Error::VersionMismatch.local_protocol` and `observe::SessionInfo.protocol`)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: deliberate-but-expired (every site was accurate under the `.protocol()` builders; 368da2a5 removed them and re-touched peer.rs:604 without rewording, and its prose sweep pattern did not include "select")
- Owner-gated: no (the enum itself stays, per the retirement note's decision 2)

Seven sites in the partition describe the protocol as something the user selects: the module doc, the error table's remedy row, the `VersionMismatch` display string (the text a user reads at runtime), two `Error` variant docs, `target_message_size`'s "default protocol", and `payload_depth_limit`'s analogy. No selector exists, so a user following the remedy row has nothing to do. AGENTS.md's hard rule is that nothing in the tree refers to code that no longer exists.

Evidence:

    14	//! | [`Error::VersionMismatch`] | unchanged | select the same [`Protocol`] at both ends; if both already do, the selected protocol's wire version differs across the two releases: align crate versions |

    76	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]

    1	//! Selectable wire reconciliation protocols.

    603	    /// is therefore a fleet-coordinated configuration event, like
    604	    /// changing the selected [`Protocol`](crate::Protocol), never a
    605	    /// per-peer tuning parameter.

Resolution: protocol.rs:1 becomes "The wire protocol version a session speaks."; error.rs:14 becomes "the two releases speak different wire versions: align crate versions"; error.rs:76 becomes "peer speaks rumors protocol version {remote_version}; this release speaks {local_protocol:?}"; error.rs:179 and 201 say "this release's dialect"; peer.rs:512 says "When a session supplies a subtree the counterparty lacks"; peer.rs:604 says "like upgrading to a release that speaks a new [`Protocol`] version". Apply the same wording to the out-of-partition twins at handshake.rs:180 and 204 and gossip.rs:723. Acceptance: `grep -rn -i select src/protocol.rs src/error.rs src/peer.rs` returns no line pairing selection with `Protocol` or a dialect, and no doc or message implies a protocol choice the API does not offer.

### api-core-5: Derived `Debug`/`Clone`/`PartialEq` on generic wrappers demand bounds their representations never use; `Error<B>` is not `std::error::Error` for a non-`Debug` bookmark
- Where: src/error.rs:61-63 (related: src/snapshot.rs:16-20, src/peer/gossip.rs:93-94, src/peer/gossip.rs:149-150, src/peer/bootstrap.rs:365-366, src/tree.rs:90, src/tree.rs:137-150, src/bookmark.rs:27-30, src/peer.rs:182-184, src/peer/bootstrap.rs:81-84, tests/api_send_bounds.rs:28-38)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (read the derive sites; read thiserror-impl 2.0.18 src/expand.rs:202-206, the version Cargo.lock pins, which inserts `Self: Debug` and `Self: Display` into the generated `impl Error`'s where-clause whenever the type has type parameters; derive-added per-parameter bounds are language-defined; the compile-time assertions below were not compiled)
- Seen by: structure, correctness, perfapi; refutation: confirmed; history: no rationale found (the derives predate the crate's own stated principle at peer.rs:182-183 and bootstrap.rs:81-84)
- Owner-gated: no (every change loosens a bound; the alternative of a `Debug` supertrait on `BookmarkError` would be gated)

`#[derive(Debug)]` on `Error<B: BookmarkError>` adds `B: Debug` although no variant holds a `B` (only `BookmarkIo<B::Error>`, whose `Debug` the trait already guarantees through `B::Error: std::error::Error`), and thiserror's generated `impl std::error::Error` carries `where Self: Debug`. `Bookmark` and `BookmarkError` never require `Debug` (bookmark.rs:27-30, 92), and bootstrap.rs:286-287 records that as deliberate. So for a legal `B: !Debug`, `Rumors<T, B>::gossip`'s error is neither `Debug` nor `std::error::Error`: `.unwrap()`, `.expect()`, and `?` into `Box<dyn Error>` stop compiling. The same derive pattern gives `Snapshot<T>` `T: Clone`, `T: Debug`, and `T: PartialEq` bounds on a documented cheap structure-sharing value whose `Tree<T>` implements `Clone` and `PartialEq` unbounded by hand (tree.rs:137, 146) and whose `Message` `Debug` prints hex bytes, never the payload (message.rs:531-538); `Tree<T>`'s own `#[derive(Debug, Eq)]` (tree.rs:90) is the upstream source of the `T: Debug` and `T: Eq` bounds, and its `Debug` prints the whole node structure recursively where `Peer` and `Rumors` print a summary. `Retire<T, B>`, `Unbookmarked<T, B>`, and `Joined<T, B>` derive `Debug` over a `Peer<T, B>` whose `Debug` is hand-written to be independent of `T: Debug` (peer.rs:182-193). Every in-tree bookmark happens to be `Debug`, so no committed test exercises the failing case.

Evidence:

    61	#[non_exhaustive]
    62	#[derive(Debug, thiserror::Error)]
    63	pub enum Error<B: BookmarkError = NoBookmark> {

    16	#[derive(Clone, Debug, PartialEq, Eq)]
    17	pub struct Snapshot<T> {

    27	pub trait BookmarkError {
    28	    /// What a [`load`](Bookmark::load) or [`store`](Bookmark::store)
    29	    /// reports when it fails.
    30	    type Error: std::error::Error + Send + Sync + 'static;

    81	// A manual, unbounded impl: the payload type is phantom (the builder
    82	// holds configuration only), so the `T: Clone` bound `derive` would add
    83	// has nothing to constrain.

Resolution: Replace the derives with representation-bounded manual impls: `impl<B: BookmarkError> fmt::Debug for Error<B>` (every field is `Debug` under the enum's existing bound); `impl<T> Clone`, `Debug`, `PartialEq`, and `Eq` for `Snapshot<T>` delegating to `Tree`'s unbounded impls, with `Debug` in the summary form `Peer` uses (network, latest, len, `finish_non_exhaustive`); and `impl<T, B: BookmarkError> Debug for Retire<T, B>` and `Unbookmarked<T, B>`. `Joined<T, B>` legitimately holds `bookmark: B`, so keep `B: Debug` there and drop only `T: Debug`. The tree partition owns `Tree<T>`'s derive at tree.rs:90. Add compile-time assertions to tests/api_send_bounds.rs. Acceptance: a committed test in tests/api_send_bounds.rs with `struct Opaque(u64)` (Serialize, Deserialize, Eq; no Debug or Clone) and `struct MuteBookmark;` (no Debug) asserts `Error<MuteBookmark>: std::error::Error`, `Snapshot<Opaque>: Clone + Debug`, and `Retire<Opaque, MuteBookmark>: Debug`, and `format!("{:?}", snapshot)` is bounded regardless of set size.
Construction: `fn require_error<E: std::error::Error>() {}` and `fn require_clone_debug<T: Clone + std::fmt::Debug>() {}`, then `require_error::<rumors::Error<MuteBookmark>>(); require_clone_debug::<rumors::Snapshot<Opaque>>();`. Both fail to compile at HEAD (derive-added bounds unsatisfied) and compile once the impls are manual.

### api-core-6: Three `Error` variants carry undocumented fields while their siblings document every field
- Where: src/error.rs:73-80 (related: src/error.rs:233, src/tree/mirror/handshake.rs:178-194)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read the whole enum, error.rs:61-290)
- Seen by: prose; refutation: confirmed; history: no rationale found (the three mirror `handshake::Error`'s equally undocumented fields)
- Owner-gated: no

`MagicMismatch.remote_magic`, `VersionMismatch.local_protocol` and `remote_version`, and `IntentInvalid.byte` have no field docs, while `NetworkMismatch`, `PayloadDepthMismatch`, `PreambleMalformed`, `PreambleTruncated`, `HandOffMalformed`, and `BootstrapHistoryConflict` document each field. A reader matching on the enum expects one treatment per field (what the six magic bytes are, which side each version belongs to).

Evidence:

    73	    MagicMismatch { remote_magic: [u8; 6] },

    77	    VersionMismatch {
    78	        local_protocol: Protocol,
    79	        remote_version: u64,
    80	    },

    233	    IntentInvalid { byte: u8 },

Resolution: One-line field docs: the first six bytes the peer sent; the protocol version this release speaks; the version number in the peer's preamble; the byte received where an intent was expected. Acceptance: every public field of `Error` has a doc comment.

### api-core-7: `Error::widen` is a variant-by-variant identity map forced by the flat generic shape
- Where: src/error.rs:316-366 (related: src/error.rs:292-314, src/error.rs:57-60)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed (with a correction: the finding's variant tally was wrong, and no tally belongs in the write-up); history: no rationale found (`widen` arrived in a WIP commit; the 2026-08-19 error-taxonomy ruling concerns naming, not shape)
- Owner-gated: yes: the alternative reshapes the public error type

`Error<B>` is generic in `B` solely for the `Bookmark(BookmarkIo<B::Error>)` arm (its own doc at 57-60 says so), so re-tagging an `Error<NoBookmark>` as `Error<B>` must enumerate every variant, all but one of them a verbatim copy, plus the `match never {}` elimination. `From<handshake::Error>` (292-314) is a second variant-by-variant map into the same enum. The compiler keeps both exhaustive, so this is maintenance weight, not a hazard; the steelman for the flat shape is real (users match one level deep, and the module's remedy table is the payoff).

Evidence:

    322	    pub(crate) fn widen<B: BookmarkError>(self) -> Error<B> {
    323	        match self {
    324	            Error::Io(error) => Error::Io(error),
    325	            Error::MagicMismatch { remote_magic } => Error::MagicMismatch { remote_magic },

    360	            Error::Bookmark(error) => match error {
    361	                BookmarkIo::Io(never) => match never {},
    362	                BookmarkIo::Format(error) => Error::Bookmark(BookmarkIo::Format(error)),
    363	            },

Resolution: Design proposal for the owner: give the wire layer a non-generic session error (the bookmark-independent variants) with `Error<B>` wrapping it beside `Bookmark`, or a `From<SessionError> for Error<B>`; `widen` then collapses to two arms or disappears and `From<handshake::Error>` targets the non-generic type. If the flat public shape is preferred, record that decision in `widen`'s doc and close the question. Acceptance: either `widen` is gone or reduced to a match over at most two arms with the public error-matching examples still compiling, or `widen`'s doc records the flat-shape decision.

### api-core-8: "all demanded once, at peer construction" is true of the serde and `Eq` bounds only
- Where: src/lib.rs:240-242 (related: src/rumors.rs:167, 210, 246, 281, 339, 364, 374, 384, 395, 494, 623; src/snapshot.rs:92, 109, 158; src/batch.rs:27, 69; src/peer.rs:195; src/message.rs:513)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read every where-clause in rumors.rs, snapshot.rs, batch.rs, and the observers' impl headers)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (the sentence's own commit, a5a16f43, scopes the claim to serde and `Eq`)
- Owner-gated: no

`Serialize`, `DeserializeOwned`, and `Eq` are demanded once (peer.rs:195, bootstrap.rs:252), but `T: Send + Sync + 'static` recurs on every typed read and write (`send`, `redact`, `send_all`, `redact_all`, `batch`, the observer accessors, `gossip`, `gossip_when`, `Snapshot::get`/`iter`/`range`, and the `Batch` struct itself) because `Message::arc::<T>` downcasts `Arc<dyn Any + Send + Sync>` and `Tree::act` needs `T: Send + Sync`. Generic code over `Rumors<T>` must restate them to call `send`, which is exactly the question a library user asks before writing such code.

Evidence:

    240	//! Your message type `T` needs [`serde::Serialize`],
    241	//! [`serde::de::DeserializeOwned`], [`Eq`], [`Send`], [`Sync`], and
    242	//! `'static`, all demanded once, at peer construction. Payloads are

Resolution: State which bounds recur: the serde and `Eq` obligations are demanded once at construction; `Send + Sync + 'static` accompany every typed read and write, so generic code over `Rumors<T>` carries them. Acceptance: a reader writing `fn f<T>(r: &Rumors<T>) { r.send(..) }` finds the required bounds named in the section.

### api-core-9: `static_assertions` is a runtime dependency used only under `cfg(test)`, and the crate-attribute comments misstate their mechanisms
- Where: src/lib.rs:295-302 (related: Cargo.toml:128, Cargo.toml:144-145, src/tree/typed/height.rs:175-176, src/tree/typed/height/tests.rs:8-10)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep: the crate's only `static_assertions` use is in src/tree/typed/height/tests.rs, declared behind `#[cfg(test)] mod tests;`; Cargo.toml:128 lists it under `[dependencies]`, not `[dev-dependencies]`; the vendored static_assertions 1.1.0 `assert_eq_size!` expands to an `#[allow(... unsafe_code ...)] unsafe { .. }` block)
- Seen by: prose (the comment wording; the dependency placement was its open question); refutation: confirmed; history: no rationale found (both comments date from fceb55f98 with no message body)
- Owner-gated: no

A released library pulls `static_assertions` into every downstream build for macros only its own test module expands. The comment above the `forbid(unsafe_code)` gate gestures at the right mechanism (the macro's `unsafe` block would trip `forbid`) but does not name the crate, has a number disagreement, and says "allow it only in tests" where the attribute lifts a `forbid`. The `large_futures` comment describes denying a lint as "check to make sure it's not an issue".

Evidence:

    295	// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
    296	#![cfg_attr(not(test), forbid(unsafe_code))]

    300	// Programmer error in recursive async traits can create large futures, so we
    301	// check to make sure it's not an issue
    302	#![deny(clippy::large_futures)]

    128	static_assertions = { workspace = true }

Resolution: Move `static_assertions` to `[dev-dependencies]`. Reword 295 to name the mechanism: "`static_assertions`' size macros expand to an `unsafe` block, and only the tests use them, so the `forbid` is lifted under `cfg(test)`." Reword 300-301: "Recursive async traits can produce very large futures without warning; deny the lint so a size regression is a build error." Acceptance: `cargo tree -e normal` (or `just` equivalent) shows no `static_assertions` in the normal dependency graph; `just check` and `just test` stay green; both comments name the crate or lint action they govern.

### api-core-10: Every stored message retains its encoding `Vec`'s slack capacity plus a `bytes::Shared` header
- Where: src/message.rs:393-396 (out of this partition's file list; related: src/message.rs:285-290, src/message.rs:341, src/batch.rs:71, src/lib.rs:14-16, tests/encode_alloc.rs:1-10)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (read bytes 1.11.1 src/bytes.rs:960-993, the version Cargo.lock pins: `From<Vec<u8>>` takes the allocation-free boxed-slice path only when `len == cap`, and otherwise allocates a `Box<Shared>` and retains the vector's full `cap`; read `to_vec` and both `Bytes::from(Vec)` sites; the slack figure is arithmetic on the doubling sequence, and no meter was run)
- Seen by: perfapi; refutation: confirmed (correcting the reader's bytes version from 1.12.1 to the locked 1.11.1; the code is identical); history: no rationale found (no note or meter covers message residency; tests/encode_alloc.rs meters the frame writer)
- Owner-gated: no

The per-send path (`Batch::send` at batch.rs:71, then `PayloadCodec::message`, then `Message::try_from_arc`) serializes into a `Vec` that starts at `Vec::new()` and grows by doubling as ciborium writes piecewise, then stores it with `Bytes::from(serialized)`. Nearly every live message therefore carries its encoding's power-of-two slack plus a header for its whole lifetime: at the design record `m = 172` (peer.rs:431) that is a 256-byte capacity, so 84 bytes of slack plus a header of roughly 24 bytes, about 60 percent overhead on the serialized cache the crate keeps per message so gossip can supply cached bytes. The wire path copies the cache with `serialize_bytes(&self.serialized)` (message.rs:565-568) rather than cloning the `Bytes`, so the header is a pure resident cost today. Denominator: per live message, resident bytes, plus one extra allocation per send. Sign: the memory term is fixed (strictly less after the fix); the CPU term (one shrinking realloc versus one header allocation) is roughly neutral. The crate's headline claim is that memory scales with the live set (lib.rs:14-16), and this is redundant work with a fixed sign, so under the doctrine it defaults to construct-and-measure.

Evidence:

    285	fn to_vec<T: Serialize>(value: &T) -> Vec<u8> {
    286	    let mut buf = Vec::new();

    393	        Ok(Message {
    394	            serialized: Bytes::from(serialized),
    395	            message: arc,
    396	        })

Resolution: At message.rs:394 and 341 write `Bytes::from(serialized.into_boxed_slice())`: `into_boxed_slice` shrinks the allocation (usually in place) and the boxed-slice path uses the promotable vtable with no header allocation and no slack. Land a `stats_alloc` meter in the style of tests/encode_alloc.rs around one `Rumors::send` (or a `Batch::commit` of N sends) pinning bytes retained per message to the payload `Arc` plus the exact encoding length plus tree nodes, committing the current bad number first per the metering practice. Acceptance: the committed meter shows zero retained slack and one fewer allocation per send than the parent commit's baseline; `benches/in_memory` `batch_insert` is neutral or better.

### api-core-11: `Network::from_rng` loops without bound on a degenerate RNG, and its doc calls the handled case impossible
- Where: src/network.rs:57-71 (related: src/peer.rs:210-213)
- Class / severity / confidence: correctness / nit / medium
- Provenance: assessed (read)
- Seen by: correctness (the loop), prose (the wording); refutation: confirmed; history: no rationale found (byte-identical to its origin in 0f4461932)
- Owner-gated: no

The retry is unbounded. Through the hidden `Peer::seed_rng<R: RngCore + ?Sized>` a caller-supplied RNG that fills zeros (a stub, an exhausted or mis-seeded generator) hangs peer construction forever rather than failing; the 2^-128 argument holds for a uniform RNG only. A degenerate RNG is programmer error, for which the sanctioned response is a diagnosable panic, never a hang. The doc's "cryptographically impossible" argues from likelihood about a case the loop correctly handles. Gating `seed_rng` (api-core-12) shrinks the input space to the crate's own tests, which is the cheaper fix if taken.

Evidence:

    59	    /// Re-draws in the (cryptographically impossible, `2^-128`) event of the
    60	    /// all-zero value, keeping [`BOOTSTRAP`](Self::BOOTSTRAP) reserved as the
    61	    /// unambiguous bootstrap sentinel.
    62	    pub(crate) fn from_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
    63	        loop {
    64	            let mut bytes = [0u8; 16];
    65	            rng.fill_bytes(&mut bytes);
    66	            let network = Network(bytes);
    67	            if !network.is_bootstrap() {
    68	                return network;
    69	            }
    70	        }
    71	    }

Resolution: Bound the retry to a small constant and `panic!` with a message naming the RNG as the fault, or document on `seed_rng` that a non-uniform RNG may not terminate; reword 59 to "in the negligible (2^-128) event". Acceptance: `from_rng` has no unbounded loop, or `seed_rng`'s doc states the RNG requirement; the doc no longer calls a handled case impossible.

### api-core-12: Hidden test-only surface is gated inconsistently, and `Snapshot::warm_caches` has no caller
- Where: src/peer.rs:210-213 (related: src/peer.rs:710-716, src/rumors.rs:410-416, src/snapshot.rs:163-169, src/peer.rs:480-483, src/peer.rs:726-728, Cargo.toml:142, Cargo.toml:145)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep: every `seed_rng` caller is under tests/; every `warm_caches` caller is a bench on `Rumors` or a `Tree` in src/tree/tests.rs, none on a `Snapshot`; Cargo.toml:145 gives tests and benches the `test-internals` feature through the self dev-dependency)
- Seen by: structure; refutation: confirmed (raising severity: the ungated `seed_rng` puts `rand::RngCore` in a shipped public signature, so a `rand` major bump becomes semver-visible); history: no rationale found (both predate the cfg-gating convention first applied in cb69fc951 and stated in db2718d46)
- Owner-gated: no

`Peer::seed_rng` and the three `warm_caches` are `#[doc(hidden)] pub` but compiled into every build, while the equally test-only `sync_window_floor` and `dangerously_alias_party` carry `#[cfg(any(test, feature = "test-internals"))]`. Hidden-but-shipped serves neither reader, and `seed_rng` makes `rand` (Cargo.toml:142) a public dependency. `Snapshot::warm_caches` has no caller at all: the benches warm through `Rumors::warm_caches`.

Evidence:

    210	    /// Like [`seed`](Self::seed), but draws the universe's [`Network`]
    211	    /// identifier from a caller-supplied RNG instead of [`OsRng`].
    212	    #[doc(hidden)]
    213	    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {

    480	    #[cfg(any(test, feature = "test-internals"))]
    481	    #[doc(hidden)]
    482	    #[must_use]
    483	    pub fn sync_window_floor(mut self) -> Self {

Resolution: Gate `seed_rng`, `Peer::warm_caches`, and `Rumors::warm_caches` with the same `#[cfg(any(test, feature = "test-internals"))]` as their neighbours, and delete `Snapshot::warm_caches`. If deterministic seeding is meant for downstream users, un-hide `seed_rng` and document it instead. Acceptance: `grep -rn 'fn warm_caches' src/` lists peer.rs and rumors.rs under cfg gates; a default-features `cargo doc` exposes neither `seed_rng` nor `warm_caches`; `just bench-build` and `just gate` stay green.

### api-core-13: `Peer::bookmark`'s first sentence promises a persist that a pristine seed skips
- Where: src/peer.rs:245-246 (related: src/peer.rs:260-263, src/peer/gossip.rs:372-378)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `bookmark_inner`: the `pristine` check returns before `bookmark_record()` runs)
- Seen by: prose; refutation: confirmed; history: no rationale found (the pristine skip is deliberate and stated at 260-263 and gossip.rs:368-371; the summary line and its exception paragraph were added in one hunk and never reconciled)
- Owner-gated: no

The summary line, the one that appears in the module listing, says attaching persists the identity before returning; the fourth paragraph and the code say a pristine seed is attached without touching storage.

Evidence:

    245	    /// Attach `bookmark` to this [`Peer`], persisting its identity before
    246	    /// returning.

    260	    /// A pristine [`seed`](Peer::seed), with nothing sent and no identity yet
    261	    /// donated or absorbed, has nothing worth persisting, so this touches
    262	    /// storage only once the peer *knows* something: any content, or any
    263	    /// identity beyond the undivided seed.

    376	        if pristine {
    377	            return Ok(peer);
    378	        }

Resolution: First sentence: "Attach `bookmark` to this [`Peer`], persisting its identity before returning unless the peer is a pristine seed with nothing yet to record." Keep 260-263 as the explanation. Acceptance: the summary line and paragraph 260-263 agree; `Bootstrap::bookmark` (bootstrap.rs:194-196) already states the joined-peer case correctly and needs no change.

### api-core-14: Hand-maintained variant counts remote from their enums
- Where: src/peer.rs:285 (related: src/peer/bootstrap.rs:334-335, src/peer/gossip.rs:94-134, src/peer/bootstrap.rs:366-404)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read both enums: `Retire` has Retired, Declined, Recovered, Uncertain; `Joined` has Joined, Bailed, Unbookmarked, Failed)
- Seen by: prose; refutation: confirmed; history: no rationale found (both tallies came from 2026-07-24 style passes)
- Owner-gated: no

"the four [`Retire`] outcomes" and "Each of the four ways that can end is a [`Joined`] variant" count enums defined in other files. Both are correct today and both rot on the next variant; the doctrine says state the structure, not the tally.

Evidence:

    285	    /// four [`Retire`] outcomes; in brief, a session reconciles content

    334	    /// received identity before anything is handed back. Each of the four
    335	    /// ways that can end is a [`Joined`] variant; the bookmark comes back

Resolution: "each [`Retire`] outcome"; "Each way that can end is a [`Joined`] variant". Acceptance: no numeral counts an enum's variants in the partition's prose.

### api-core-15: `sync_memory_budget`'s public doc cites test files and internals, and states the wire-buffer bound twice
- Where: src/peer.rs:308-465 (related: src/peer.rs:313, 318, 320-326, 353-357, 394, 417, 428, 445, 457, 527; src/lib.rs:322; src/tree/mirror/streaming/window/tests.rs:278; src/conformance.rs:10-12)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: `default_crossover_matches_the_solve` is a private test fn at src/tree/mirror/streaming/window/tests.rs:278; `mod tree` is private at lib.rs:322; the STREAM_COUNT x target_message_size bound appears at 320-326 and again at 353-357)
- Seen by: prose; refutation: confirmed; history: already-known for the derivation's placement (execution ledger 3a, ruled 2026-07-23, and option (c), ruled 2026-07-24: the closed form and the tradeoff table live in this rustdoc); the three sub-items below are not covered by either ruling
- Owner-gated: no for the three sub-items; relocating the sizing guide would reopen the rulings and is listed under open questions instead

The contract portion of this doc is right; three things in it are not at public altitude. (a) It prices memory "by the storage backend's own cost function" and "under the in-memory backend", and `target_message_size` repeats "storage backend" at 527, though `mod tree` is private and src/conformance.rs:10-12 itself says the storage boundary is crate-internal. (b) It cites `tests/dispute_wire.rs`, `tests/tradeoff_probe.rs`, `tests/window_operator.rs`, `tests/window_knee.rs`, and the private test fn `default_crossover_matches_the_solve` by name: repo-internal navigation a docs.rs reader cannot follow. (c) The encoded-wire-buffer bound is stated in full in the opening paragraph and again under "# What this does not bound".

Evidence:

    312	    /// what costs memory — kilobytes per disputed subtree in flight,
    313	    /// priced by the storage backend's own cost function — and

    393	    /// (calibrated by deterministic byte counts,
    394	    /// `tests/dispute_wire.rs`).

    428	    ///   and pinned by `default_crossover_matches_the_solve`;

    320	    /// This setting does not govern encoded wire messages in hand: the
    321	    /// wire schedule bounds those, at most one run per stream per
    322	    /// direction, so up to

    353	    /// - **Encoded wire messages in hand**: the run buffers stated
    354	    ///   above, priced by

Resolution: Replace "storage backend" and "in-memory backend" with "per disputed subtree in flight" (here and at 527); replace each test-file and test-fn citation with the stable claim ("the crate's tests pin the envelope and the crossover") or drop it; state the wire-buffer bound once, in the "# What this does not bound" list, and have the opening paragraph point there. Acceptance: no `tests/*.rs` path or test fn name appears in public rustdoc in the partition; "backend" appears in no public doc; the bound is stated once.

### api-core-16: The `Peer` forwarding layer has three asymmetries, and `Batch::commit`'s doc describes one caller of five
- Where: src/peer.rs:627-708 (related: src/rumors.rs:406-408, src/rumors.rs:372-377, src/batch.rs:121-127, src/tests.rs:629, src/tests.rs:648)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: `fn changes` exists only in rumors.rs; `Peer::messages_since` at peer.rs:689 is the target of `Rumors::unordered_messages_since`; `Batch::new` plus `commit` appears five times in peer.rs, four of them closure-free)
- Seen by: structure; refutation: confirmed; history: no rationale found (586911cfd renamed the public and one internal method but left `messages_since`; ce27df86 wrote "Runs iff the caller's closure returned `Ok`" in the same commit that shaped the closure-free callers)
- Owner-gated: no

`peer.rs` owns the local API as undocumented `pub(crate)` methods that `Rumors` forwards to one-for-one. Three exceptions: `Rumors::changes` constructs `Changes::subscribe(&self.peer.inner)` itself instead of going through `Peer` like its five sibling observers; `Peer::messages_since` is the target of the public `unordered_messages_since`, so the internal name diverges from the public one it implements; and `Peer::send`/`send_all` re-spell `Batch::new; op; commit` instead of `self.batch(|b| b.send(m))`, while `Batch::commit`'s doc says it runs iff the caller's closure returned `Ok` and that `Rumors::batch` owns that decision, which is untrue of the four direct callers. A forwarding layer earns its keep by being uniform; the `commit` doc is a present-tense claim the code contradicts.

Evidence:

    627	    pub(crate) fn send(&self, message: T) -> Result<(), EncodeError>
    628	    where
    629	        T: Send + Sync + 'static,
    630	    {
    631	        let mut batch = Batch::new(&self.inner, self.codec);
    632	        batch.send(message)?;
    633	        batch.commit();
    634	        Ok(())
    635	    }

    689	    pub(crate) fn messages_since(&self, since: Version) -> UnorderedMessages<T>

    123	    /// Observers and concurrent gossip sessions see all of it land at
    124	    /// once, in at most one observer wakeup. Runs iff the caller's
    125	    /// closure returned `Ok`
    126	    /// ([`Rumors::batch`](crate::Rumors::batch) owns that decision).

Resolution: Route `Rumors::changes` through a `Peer::changes`; rename `Peer::messages_since` to `unordered_messages_since`; express `Peer::send` as `self.batch(|batch| batch.send(message))` and `send_all` likewise (either leave the infallible `redact`/`redact_all` explicit or add a private `commit_with`); restate `Batch::commit`'s doc as: commits everything queued in one critical section, reached from `Rumors::batch` on `Ok` and from the one-shot `send`/`redact` families. Acceptance: `grep -n 'Batch::new' src/peer.rs` shows one site or a single private helper; `grep -rn 'fn messages_since' src/` is empty; `Rumors::changes` forwards to `self.peer`; batch.rs:121-127 no longer claims a single caller.

### api-core-17: First sentences mix imperative and third-person moods; the doctrine of record fixes third person
- Where: src/peer.rs:710-712 (related: src/snapshot.rs:163, src/rumors.rs:410, src/batch.rs:44, 76, 84, 107, src/snapshot.rs:23, 84, 97, 114, src/network.rs:57, 84, src/rumors.rs:133)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the first sentences; ~/.claude/writing-style.md:122-124 reads "Every doc comment opens with one complete, standalone sentence that survives extraction into a single-line module listing: verb-first, present tense, third person")
- Seen by: prose (proposing the opposite direction); refutation: confirmed the inconsistency and noted std's third-person convention; history: deliberate-and-holds for third person, so the finding is reframed: the imperative majority is the deviation
- Owner-gated: yes: a crate-wide prose sweep whose direction contradicts the current majority

`Batch`, `Snapshot`, and `Network` open in the third person ("Queues", "Looks up", "Iterates", "Forces", "Draws"), which the writing doctrine mandates; `Peer`, `Rumors`, `Bootstrap`, and the observers open in the imperative ("Send a message", "Attach", "Bound", "Force this set's tree"). The same sentence appears in both moods at peer.rs:710 and snapshot.rs:163. One mood reads as one voice in the module listing.

Evidence:

    710	    /// Force this set's tree to compute its lazy structural memos (observable

    163	    /// Forces this set's tree to compute its lazy structural memos (observable

    44	    /// Queues a message for this batch's commit.

    133	    /// Send a message, committing it immediately.

Resolution: Owner ruling, then a sweep: per the doctrine, align the imperative sites to third person (the larger set); if the imperative is preferred for this crate, amend the doctrine and align the three third-person files. At minimum make peer.rs:710, rumors.rs:410, and snapshot.rs:163 identical. Acceptance: first sentences across the partition share one mood.

### api-core-18: The `Bootstrap` page enumerates a three-setting builder that has four
- Where: src/peer/bootstrap.rs:33-38 (related: src/peer/bootstrap.rs:141-142, src/peer/bootstrap.rs:161-162, src/peer/bootstrap.rs:171-172, src/peer/bootstrap.rs:269, src/peer.rs:236-240)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read the four setting methods at 133, 153, 166, 180 and `bookmark` at 212; `git merge-base --is-ancestor` per the prose lens confirms the roster predates `observe`)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (the roster was accurate at c5f1210a3; 40b1e96a added `observe` without touching any roster line; 4356e197 maintained the roster for `payload_depth_limit` but nothing else; 55e347382's "rather than a fourth setting" was arithmetic on three)
- Owner-gated: no

Three statements on the public page are stale against the builder's current methods. The type doc enumerates three settings and omits `observe`; `target_message_size` is called "the one setting with immediate effect on the bootstrap session" while `payload_depth_limit` ("The join session decodes the provider's supplied records before a [`Peer`] exists, so the bound is selected here") and `observe` ("starting with the bootstrap session itself") both say they take effect there too; and `BookmarkedBootstrap` is justified as "A distinct type, rather than a fourth setting" when a fourth setting already exists. `Peer::bootstrap` (peer.rs:236-237) carries the same partial list. A user reading 141-142 and then 161-162 gets two contradictory answers to which settings affect the join session.

Evidence:

    33	/// Every setting here is the new peer's own, selected one session
    34	/// early: [`sync_memory_budget`](Self::sync_memory_budget),
    35	/// [`target_message_size`](Self::target_message_size), and
    36	/// [`payload_depth_limit`](Self::payload_depth_limit) each state what they
    37	/// change about the bootstrap session itself, and the joined peer keeps

    141	    /// This is the one setting with immediate effect on the bootstrap
    142	    /// session, the session that transfers the provider's entire set as

    269	/// type, rather than a fourth setting, lets each state's `join` declare only

    236	    /// builder's settings ([`Bootstrap::sync_memory_budget`],
    237	    /// [`Bootstrap::target_message_size`]) are the peer-to-be's own,

Resolution: Rewrite 33-38 without the roster ("Every session setting here is the new peer's own, selected one session early; each method states what it changes about the bootstrap session itself"). Replace 141-142 with the structural fact: the settings with a wire or handshake effect (run sizing, the greeting's depth-limit field, observation) reach the join session; `sync_memory_budget` alone has nothing to bound there. Replace "rather than a fourth setting" with "rather than another setting". Trim peer.rs:236-240 to point at the builder's page. Acceptance: no ordinal or method roster remains on the `Bootstrap`/`BookmarkedBootstrap` pages or in `Peer::bootstrap`; the `target_message_size`, `payload_depth_limit`, and `observe` docs agree about which settings affect the join session.

### api-core-19: `Bootstrap`'s `Debug` omits the `observe` setting
- Where: src/peer/bootstrap.rs:96-104 (related: src/peer/bootstrap.rs:73-75, src/observe.rs:218-222)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the impl; `Attachment` implements `Debug` at observe.rs:218, printing `attached: bool`)
- Seen by: none of the four lenses; raised by the refutation pass as new; history: same cause as api-core-18 (40b1e96a added the field without touching the impl)
- Owner-gated: no

The hand-written `Debug` prints `window`, `run_budget`, and `payload_depth_limit` but not `observe`, the same three-of-four staleness as the page prose. `Attachment` is `Debug`, so the field can be printed; `finish()` rather than `finish_non_exhaustive()` presents the three as the whole configuration.

Evidence:

    96	/// The configuration only; the payload type parameter carries no state.
    97	impl<T> std::fmt::Debug for Bootstrap<T> {
    98	    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    99	        f.debug_struct("Bootstrap")
   100	            .field("window", &self.window)
   101	            .field("run_budget", &self.run_budget)
   102	            .field("payload_depth_limit", &self.payload_depth_limit)
   103	            .finish()
   104	    }
   105	}

Resolution: Add `.field("observe", &self.observe)`. Acceptance: `format!("{:?}", Peer::<u64>::bootstrap().observe(handler))` shows `attached: true`.

### api-core-20: Bootstrap plumbing testdocs claim every knob; the bodies check two of four
- Where: src/peer/bootstrap/tests.rs:6-8 (related: src/peer/bootstrap/tests.rs:44-52, src/peer/bootstrap/tests.rs:54-66, src/peer/bootstrap/tests.rs:77-91, tests/payload_depth.rs:146-148, tests/observe.rs:278-281, src/message.rs:248, src/observe.rs:213-216)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read the whole file: every assertion is on `window` or `run_budget`; `payload_depth_limit` and `observe` appear in no assertion; `PayloadCodec::limit()` exists at message.rs:248; `Attachment` derives only `Clone, Default`, no `PartialEq`)
- Seen by: prose, correctness; refutation: confirmed; history: deliberate-but-expired (accurate at f11949147 for three knobs, all asserted; 40b1e96a and 4356e197 added knobs without touching this file; 368da2a5 removed the protocol assertions and kept the quantifier)
- Owner-gated: no

The module doc says "every builder knob reaches the builder's state, and every stored choice reaches the joined peer unchanged"; `knobs_store_the_selected_values` says "Each knob stores exactly the selected value"; `joined_peer_retains_the_configuration` says "The joined peer retains every builder choice". Each body asserts only `window` and `run_budget`, so a regression in `payload_depth_limit` or `observe` plumbing passes a suite whose doc says it is covered. `defaults_match_the_seed_configuration` claims the builder matches "the same budget and run target [`Peer::seed`] starts with" but compares against constants, not against a `Peer::seed()`, so a seed that drifted from the constants would not fail it. AGENTS.md: an inaccurate testdoc is a bug in the test.

Evidence:

    6	//! the session bytes in `tests/bootstrap_snapshot.rs`. This suite pins the plumbing those tests
    7	//! rest on: every builder knob reaches the builder's state, and every
    8	//! stored choice reaches the joined peer unchanged.

    54	/// Each knob stores exactly the selected value, through the same
    55	/// constructors as the matching [`Peer`] methods.

    65	    assert_eq!(budget_bytes(config.window), CUSTOM_BUDGET);
    66	    assert_eq!(config.run_budget, RunBudget::from_bytes(CUSTOM_TARGET));

    77	/// The joined peer retains every builder choice for its later sessions.

Resolution: Extend the bodies: assert `config.payload_depth_limit` and `peer.codec.limit()` against a non-default `PayloadDepthLimit`, and assert the attachment reaches the joined peer (a `pub(crate)` `Attachment::is_attached()` or the `Debug` form, since `Attachment` has no `PartialEq`); in `defaults_match_the_seed_configuration`, read the expected values off `Peer::<u64>::seed()`'s `window`, `run_budget`, and `codec`. Alternatively narrow every quantifier to the two sizing knobs and point at tests/payload_depth.rs and tests/observe.rs for the rest. Acceptance: each testdoc's quantifier matches the fields its body asserts; the defaults test reads its expected values off a seeded `Peer`.

### api-core-21: `Protocol` derives `Default` for a call site that no longer exists
- Where: src/protocol.rs:13-18 (related: .agent-notes/2026-09-01-v1-retirement/README.md:153-160)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (grep: `Protocol::default()` appears nowhere in src, tests, benches, or examples; `SessionInfo`, the only struct holding a `Protocol`, does not derive `Default`)
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (the derive served `Protocol::default()` in `Peer::seed_rng` and `Bootstrap::new`; 368da2a5 deleted both call sites and left the derive; decision 2 of the retirement note keeps the enum public and `#[non_exhaustive]`)
- Owner-gated: yes: removes a public trait impl

The `Default` derive and its `#[default]` marker are machinery that outlived the builder default they served. The enum itself stays by ruling; only the derive is dead.

Evidence:

    13	#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    14	#[non_exhaustive]
    15	pub enum Protocol {
    16	    /// Bounded-memory reconciliation over multiplexed logical streams.
    17	    #[default]
    18	    V2 = 2,

Resolution: Remove `Default` from the derive list and the `#[default]` attribute. Acceptance: the build is clean and `grep -rn 'Protocol::default' src tests` stays empty.

### api-core-22: `Extant` keeps two counters of one quantity; the `watch::Sender` clone count alone carries the protocol
- Where: src/rumors.rs:39-60 (related: src/rumors.rs:93-105, src/rumors.rs:108-131)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read the vendored tokio 1.52.3 watch.rs at the version Cargo.lock pins: `impl<T> Clone for Sender<T>` bumps `ref_count_tx` (201-209); `Drop for Sender` sets closed and notifies waiters when the last clone drops (1458-1465); `changed_impl` registers `notified()` before checking state (989-999), so no wake is lost; `maybe_changed` returns `Err` iff the version is unchanged and the closed bit is set (961-981); `subscribe` marks the current version seen (1387-1394). The redesign was not compiled or run.)
- Seen by: structure; refutation: confirmed; history: no rationale found (the two-counter design arrived whole in cb69fc951; its predecessor 36df73797 used the watch channel's own close as the quiescence signal, so the single-counter form has precedent in this tree)
- Owner-gated: no

`Extant` counts extant handles with an `Arc<()>` strong count and separately wakes reuniters through a cloned `watch::Sender<()>`, whose clone count is also the number of extant handles (each `Rumors` holds one `Extant`, and `#[derive(Clone)]` clones both fields). The redundancy is what forces the `Option` around the token (so `Drop` can shed it before the wake), the hand-written `Drop`, and the strong-count re-check loop in `try_into_peer_inner`. tokio's `watch` already emits the needed event: `Receiver::changed()` returns `Err` exactly when every `Sender` clone has dropped and the current value is seen, and nothing here ever sends a value, so the close is the one event a receiver can observe. tokio's `Sender::drop` decrements its count and closes the channel atomically with respect to receivers, so the ordering hazard the field docs describe cannot arise. Every piece exists to reconcile two counters that must agree; with one counter they dissolve.

Evidence:

    39	#[derive(Clone)]
    40	struct Extant {
    41	    /// The extancy token. An `Option` only so [`Drop`] can shed it *before*
    42	    /// waking waiters on `drops`: a reuniter woken by that send must already
    43	    /// observe the decremented strong count. Always `Some` outside `Drop`.
    44	    token: Option<Arc<()>>,

    54	impl Drop for Extant {
    55	    fn drop(&mut self) {
    56	        // Shed the token first, then wake: see the field docs above.
    57	        self.token = None;
    58	        self.drops.send_replace(());
    59	    }
    60	}

Resolution: `struct Extant { alive: watch::Sender<()>, claimed: Arc<AtomicBool> }` with `#[derive(Clone)]` and no `Drop`; `Rumors::new` builds `alive: watch::Sender::new(())`. `try_into_peer_inner` becomes: subscribe a receiver, clone `claimed`, drop the `Extant`, then `while alive.changed().await.is_ok() {}` (the `Err` means every sender is gone, and the closed state is monotone because a new `Sender` needs a live `Rumors` to clone from), then the existing `compare_exchange` claim. Keep the monotonicity and exactly-once prose; drop the token and wake-ordering prose. Land api-core-25's tests first so the redesign is exercised. Acceptance: `Extant` has two fields and no `Drop` impl; `Arc<()>` and `strong_count` no longer appear in rumors.rs; the reunion tests, including the concurrent-reuniter cases api-core-25 adds, pass.

### api-core-23: `Rumors` re-spells `Peer`'s field list and `Debug` body
- Where: src/rumors.rs:62-77 (related: src/rumors.rs:81-90, src/peer.rs:184-193, src/peer/gossip.rs:349-366, src/peer/gossip.rs:387-395)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified (read all sites)
- Seen by: structure (the field-literal clone); refutation: confirmed, and raised the `Debug` duplication as new; history: no rationale found
- Owner-gated: no

`Rumors::clone` rebuilds a `Peer` by listing all seven fields with the bookmark `Arc` shared; `Peer::bookmark_inner` does the same twice (to attach `B`, and to hand a `NoBookmark` peer back on failure). All three are the one operation "same peer, different or shared bookmark handle", and the field knowledge of `Peer` lives in rumors.rs. Separately, `Rumors`'s `Debug` re-spells `Peer`'s `Debug` body field for field under a different struct name. The struct literals are compiler-checked, so the cost is repetition and two views that can drift.

Evidence:

    64	        Self {
    65	            peer: Peer {
    66	                network: self.peer.network,
    67	                window: self.peer.window,
    68	                run_budget: self.peer.run_budget,
    69	                inner: self.peer.inner.clone(),
    70	                bookmark: Arc::clone(&self.peer.bookmark),
    71	                codec: self.peer.codec,
    72	                observe: self.peer.observe.clone(),
    73	            },

    81	impl<T, B: BookmarkError> std::fmt::Debug for Rumors<T, B> {
    82	    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    83	        let inner = self.peer.inner.borrow();
    84	        f.debug_struct("Rumors")
    85	            .field("network", &self.peer.network)
    86	            .field("latest", inner.tree.latest())
    87	            .field("len", &inner.tree.len())
    88	            .finish_non_exhaustive()

Resolution: Add `pub(crate) fn with_bookmark<B2: BookmarkError>(&self, bookmark: Arc<Mutex<Bookmarked<B2>>>) -> Peer<T, B2>` in peer.rs, documented as the crate-internal duplication the public `!Clone` deliberately withholds, and use it at rumors.rs:65-73 and both gossip.rs sites; give `Peer` a private `fn debug_fields(&self, s: &mut fmt::DebugStruct)` that both `Debug` impls call. Acceptance: `grep -c 'run_budget:' src/rumors.rs src/peer/gossip.rs` drops to the from-scratch construction in `bootstrap_inner`; the two `Debug` impls share one body; tests green.

### api-core-24: Terminology drifts in public prose, including a ghost of the retired `Broadcast` type
- Where: src/rumors.rs:93 (related: src/rumors.rs:33, src/rumors.rs:358-359, 369, 379, 389, src/rumors/unordered.rs:9, src/rumors/unordered.rs:12-14, src/rumors/causal.rs:15, src/peer.rs:303, src/rumors.rs:345-346, src/snapshot.rs:34-35)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S'pub struct Broadcast' -- src` ends at cb69fc951, the commit that renamed the type away; 7be20b412 swept the phrase from `Extant`'s doc and missed `Rumors::new`'s; unordered.rs:12-14 read as cited)
- Seen by: prose; refutation: confirmed; history: no rationale found; the `Broadcast` ghost edges into AGENTS.md's rule against deleted API names in prose
- Owner-gated: no

(a) `Rumors::new`'s doc says "a fresh broadcast generation" where rumors.rs:33 says "a [`Rumors`] generation"; `Broadcast` was the type `Rumors` replaced. (b) The observer accessors say "every message sent to this [`Rumors`]" (358, 369, 379, 389; also unordered.rs:9 and causal.rs:15), though gossip-learned messages are observed too and only unordered.rs:12-14 says so. (c) The three `network()` accessors open with three different sentences for one fact (peer.rs:303 "The globally unique identifier for this network of gossiping [`Peer`]s."; rumors.rs:345-346 and snapshot.rs:34-35 "The identifier shared by every peer that descends from the same [`seed`]"); the doctrine's parallel-prose rule wants siblings to share one skeleton.

Evidence:

    93	    /// Assemble the first handle of a fresh broadcast generation around `peer`,

    33	/// One handle's share of a [`Rumors`] generation's existence.

    358	    /// Monitor every message sent to this [`Rumors`], in arbitrary
    359	    /// (*non-causal*) order.

    12	/// This enumerates every message not causally contained in the starting
    13	/// checkpoint, then every message learned afterwards: by local
    14	/// [`send`](crate::Rumors::send), by gossip, through any handle. Once the

Resolution: (a) "a fresh [`Rumors`] generation"; (b) "Monitor every message live in this [`Rumors`], however it arrived" at the four accessors and the two observer type docs; (c) one sentence for all three `network()` accessors. Acceptance: `grep -rn broadcast src` is empty; the six observer sentences and the three accessors use the crate's established terms.

### api-core-25: `try_into_peer`'s exactly-once claim among concurrent reuniters is untested
- Where: src/rumors.rs:116-130 (related: src/rumors.rs:45-48, src/rumors.rs:54-60, src/rumors.rs:110-115, tests/disruption.rs:830-837, tests/listen.rs:309-314, tests/api_send_bounds.rs:73-79, tests/lifecycle.rs (an existing `noop_waker` harness))
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of every `try_into_peer` call across tests/ and src/: each is a sole-handle reunite or a sequential per-set reunite awaiting clones dropped by spawned tasks; `grep -rn 'join!(.*try_into_peer'` is empty; no harness polls two reuniters on clones of one set; the constructed test was not run)
- Seen by: correctness; refutation: confirmed; history: no rationale found (the `claimed` CAS and the subscribe-before-shed ordering arrived in cb69fc951 with no concurrent-reuniter test; neither prior review packet mentions `Extant`)
- Owner-gated: no

The `claimed` CAS exists only for two or more reuniters observing quiescence concurrently, and the subscribe-before-shed ordering only for a reuniter that parks and is woken by a later drop. No committed test builds either shape, so deleting `claimed` or moving `drops.subscribe()` below `drop(extant)` would fail nothing deterministically. Each `Rumors` clone carries its own `Peer` value (rumors.rs:62-77), so without the CAS two reuniters would each receive a `Peer` for one identity, the linearity violation the `Peer`/`Rumors` XOR (lib.rs:113-126) exists to exclude. The guard doctrine asks that a guard name a concrete failure the committed tests catch; here they do not.

Evidence:

   116	        loop {
   117	            // Monotone once zero: creating a token takes a live `Rumors` to
   118	            // clone, and every reuniter has already shed its own.
   119	            if token.strong_count() == 0 {
   120	                // Exactly one reuniter wins the claim; the Peer/Rumors
   121	                // XOR is restored the instant this swap succeeds.
   122	                return claimed
   123	                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
   124	                    .is_ok()
   125	                    .then_some(peer);
   126	            }

Resolution: Add a point suite (tests/reunite.rs) driven with a noop waker, as tests/lifecycle.rs already does: (1) three clones, two reuniters polled to `Pending`, drop the third clone, poll both to completion, assert exactly one `Some`; (2) one reuniter parked, then the last clone dropped, assert it resolves `Some`; (3) two parked, drop one reuniter future, drop the last clone, assert the survivor gets `Some`. Then a proptest over N clones and K >= 1 reuniters with a shuffled drop/poll order asserting exactly one `Some` and every other reuniter `None`. Acceptance: the new tests are committed and fail when `claimed` is removed (two `Some`) and when `drops.subscribe()` is moved below `drop(extant)` (a parked reuniter stays `Pending` after the last drop).
Construction: `let a = Peer::<u64>::seed().into_rumors(); let b = a.clone(); let c = a.clone(); let mut f1 = pin!(a.try_into_peer()); let mut f2 = pin!(b.try_into_peer());` poll both with `futures::task::noop_waker_ref()` and assert `Pending`; `drop(c)`; poll both to `Ready`; assert `f1_result.is_some() ^ f2_result.is_some()`.

### api-core-26: The `_since` observer constructors state no same-universe precondition on `since`
- Where: src/rumors.rs:369-377 (related: src/rumors.rs:389-398, src/rumors/unordered.rs:111-113, src/rumors/causal.rs:106-108, src/network.rs:14-19)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read; a `Version` carries no `Network`, so nothing can check it)
- Seen by: correctness; refutation: confirmed; history: no rationale found (the same-network clause has lived only on `checkpoint()` since the `listen_from` era)
- Owner-gated: no

A checkpoint `Version` from another seed makes the containment filter meaningless and silently skips messages; network.rs:16-19 already records that independent universes can be "coincidentally and transiently compatible", which is exactly how a foreign checkpoint would filter live messages out. The precondition appears only on the producer side (`checkpoint()`: "another replica of the same network") and not where the value is consumed. AGENTS.md: never let two independently seeded universes interact; where the type system cannot enforce that, the contract at the consuming call is the only guard.

Evidence:

   369	    /// Monitor every message sent to this [`Rumors`] which is not already
   370	    /// causally contained in `since`, then everything learned afterwards, in
   371	    /// arbitrary (*non-causal*) order.
   372	    pub fn unordered_messages_since(&self, since: Version) -> UnorderedMessages<T>

Resolution: Add one sentence to both `_since` docs: `since` must be a checkpoint or frontier observed in this same [`Network`] (compare [`Rumors::network`]); a version from another universe is undetectable here and resumes incorrectly. Acceptance: both `_since` methods name the precondition and point at `network()`.

### api-core-27: The session promise is stated in full on two pages
- Where: src/rumors.rs:451-481 (related: src/link.rs:279-321; the anchor is already the link target at src/lib.rs:228, src/peer.rs:289, src/peer/bootstrap.rs:239, src/rumors.rs:462, src/rumors.rs:543)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (read both sections side by side; grep for `what-a-session-promises` lists the five link sites)
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds for repeating hazards (~/.claude/writing-style.md:219-230: repeat the hazard at every surface, "linking to one canonical explanation"), so the finding is reframed from "pick one home" to "make one the summary"
- Owner-gated: no

`Rumors::gossip` restates the whole `Ok`/`Err`/cancellation contract with its three qualified exceptions, and `Link`'s "# What a session promises" states the same in full. They agree today; with two full normative statements, the next edit to one is the drift. Every other page in the crate that needs the promise links to `Link`'s anchor, so `Link` is already the canonical home the doctrine asks for.

Evidence:

   457	    /// On `Err`, the replica is unchanged and the link is poisoned:
   458	    /// discard it and reconnect. This is enforced, not advisory, since
   459	    /// every subsequent session on the link fails fast with
   460	    /// [`Error::LinkPoisoned`] rather than misreading its mid-frame
   461	    /// control stream. Cancellation counts as `Err` ([what a session
   462	    /// promises](crate::link::Link#what-a-session-promises)).
   463	    /// "Unchanged" has three qualified exceptions:

Resolution: Cut `Rumors::gossip` to the `Ok` arm plus a two-line `Err`-and-cancellation summary that links to the anchor, leaving the three-exception list on `Link` alone. Acceptance: one page carries the three-exception list; the other links to it.

### api-core-28: `# Cancellation` where the crate's hazard heading is `# Cancel safety`
- Where: src/rumors.rs:563 (related: src/link.rs:252, src/bookmark.rs:326, src/link/routed.rs:277, src/link/routed/endpoint.rs:295, and four streaming-tree sites)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn '# Cancel' src`: nine sites, one outlier, and the outlier's body at 565-571 is a cancel-safety statement)
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (the heading predates the crate's adoption of `# Cancel safety`)
- Owner-gated: no

Hazards get uniform named sections so a reader can scan for them; `gossip_when` is the one section spelled differently.

Evidence:

   563	    /// # Cancellation

   252	    /// # Cancel safety

Resolution: Rename to `# Cancel safety`. Acceptance: `grep -rn '# Cancel' src` shows one spelling.

### api-core-29: `CausalMessages` copies each staged version's bytes into a fresh `Vec` for the tiebreak key
- Where: src/rumors/causal.rs:99-102 (related: src/rumors/causal.rs:54-63, crates/before/src/version.rs:91-95, crates/before/src/codec/bits.rs:91-97, crates/before/src/version/rank.rs:882, crates/before/src/version/ranked.rs:372-383, tests/causal.rs:146-153)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read; `Version` is `#[derive(Clone)]` over a refcounted `Bytes`, so its clone is O(1) and `as_bytes` is a borrow; `Rank: Ord`; not measured)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (9c73d7b4 chose the `(Rank, bytes)` key and documents the order, not the copy; `Version::clone` was already O(1) then)
- Owner-gated: no

Per staged leaf, `ingest` allocates a `Vec<u8>` copy of `version.as_bytes()` as the rank tiebreak in the `BTreeMap<(Rank, Vec<u8>), Leaf>` key, while the `Leaf` already owns the version and cloning it is a refcount bump. Denominator: one heap allocation and a version-length copy per message delivered through `CausalMessages`, plus a `Vec` header in every staged key. Sign: fixed (a strict deletion of a copy; the order is unchanged). Materializing `Rank` once per leaf stays justified: `Ranked::cmp` is a rank co-sweep per comparison.

Evidence:

    99	        while let Some((_, leaf)) = walk.next() {
   100	            let version = leaf.version();
   101	            staged.insert((version.rank(), version.as_bytes().to_vec()), leaf);
   102	        }

    63	    staged: BTreeMap<(Rank, Vec<u8>), Leaf>,

Resolution: A private `struct StageKey { rank: Rank, version: Version }` whose `Ord` compares `(rank, version.as_bytes())` lexicographically and whose `Eq` is derived (`Version`'s `Eq` is canonical byte equality, so the two agree); key `staged` by it, cloning the leaf's `Version`. `before` needs no change. Acceptance: a `stats_alloc` region around one ingest pass shows one fewer allocation per delivered message; tests/causal.rs's `(rank, bytes)` ordering assertion still passes.

### api-core-30: Small `Copy` enums omit `Hash`, and `Protocol` omits `Ord`
- Where: src/rumors/changes.rs:55-56 (related: src/peer/gossip.rs:186-187, src/peer/gossip.rs:207-208, src/protocol.rs:13, src/network.rs:24, src/message.rs:71)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (read the derive lines)
- Seen by: perfapi; refutation: confirmed; history: no rationale found
- Owner-gated: yes: adds public trait impls

`TryTick`, `Led`, and `Gossip` derive `Debug, Clone, Copy, PartialEq, Eq` but not `Hash`; `Protocol` derives neither `Hash` nor `PartialOrd`/`Ord` though a wire version is naturally ordered. `Network` and `PayloadDepthLimit` carry the full set. Keying per-session metrics by `Led` in a `HashMap` is the concrete use.

Evidence:

    55	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    56	pub enum TryTick {

Resolution: Add `Hash` to the four derives and `PartialOrd, Ord` to `Protocol`. Acceptance: `fn key<K: Hash + Eq>() {}` instantiates for all four in tests/api_send_bounds.rs.

### api-core-31: Import hygiene: `crate::Inner` spelled at seven sites, a qualified constant in a return type, scattered import groups
- Where: src/rumors/unordered.rs:63-71 (related: src/rumors/unordered.rs:84, 97, src/rumors/causal.rs:67, 88, src/rumors/changes.rs:68, src/snapshot.rs:80, src/peer/bootstrap.rs:5-23, src/peer.rs:26-28, src/network.rs:7-11, src/snapshot.rs:1-2, src/rumors.rs:9-19, src/batch.rs:8)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `crate::Inner` across src; `ls rustfmt.toml .rustfmt.toml` finds no config; import blocks read in full)
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

The observers spell `crate::Inner<T>` in full at seven sites rather than importing `Inner` as batch.rs:8 does; `Snapshot::hash` returns `[u8; crate::MERKLE_HASH_LEN]`; and several files interleave import groups (bootstrap.rs:5-22 splits `std` across tokio and puts `serde` last with no blank line before the doc comment at 23; peer.rs:26-28 likewise before `mod bootstrap;`; network.rs:7-11 spells four `use serde::...` lines and abuts the type doc; snapshot.rs:1-2 and rumors.rs:9-19 put crate imports before `std`). rustfmt does not reorder across blank-line groups, so `just fmt` leaves these as they are.

Evidence:

    63	type WaitForChange<T> =
    64	    Pin<Box<dyn Future<Output = (bool, watch::Receiver<crate::Inner<T>>)> + Send>>;

    69	pub(super) enum Channel<T> {
    70	    /// The channel is in hand.
    71	    Ready(watch::Receiver<crate::Inner<T>>),

Resolution: `use crate::Inner;` in the three observer files and `use crate::MERKLE_HASH_LEN;` in snapshot.rs; regroup imports as std / external / crate with a blank line between groups and before the first item. Optionally adopt `group_imports = "StdExternalCrate"` in a rustfmt.toml so `just fmt-check` enforces it. Acceptance: `grep -rn 'crate::Inner' src/rumors/` is empty; `grep -n 'crate::MERKLE_HASH_LEN' src/snapshot.rs` is empty; `just fmt-check` clean.

### api-core-32: Five doc examples discard `send`'s `Result`, and rustdoc's injected `#![allow(unused)]` hides it
- Where: src/rumors/unordered.rs:139 (related: src/rumors/unordered.rs:156, src/snapshot.rs:140, 142, 143, src/lib.rs (no `#![doc(test(attr(..)))]`), justfile:122-123)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified for the sites (grep: five bare `.send("..");` statements in doc examples; every other example uses `?` with an `Ok::<(), EncodeError>(())` tail); the rustdoc mechanism is assessed from rustdoc's documented doctest preamble, not run
- Seen by: perfapi; refutation: reframed (the gate's `RUSTDOCFLAGS` is not the reason: rustdoc prepends `#![allow(unused)]` to every doctest of a crate that declares no `#![doc(test(attr(..)))]`, and a command-line `-D warnings` does not override a source-level `allow`); history: deliberate-but-expired (the bare statement was the documented idiom when `send` committed on drop; ce27df86 and 6e4b6eea changed `send` without touching these examples)
- Owner-gated: no

Examples teach; these teach readers to ignore the admission error the crate surfaces at the author on purpose. They survive because the doctest preamble allows `unused_must_use`.

Evidence:

   139	    /// rumors.send("one".to_string());

   140	    /// rumors.send("first".to_string());
   141	    /// let then = rumors.snapshot().latest().clone();
   142	    /// rumors.send("second".to_string());
   143	    /// rumors.send("third".to_string());

Resolution: Use `?` with the `# Ok::<(), rumors::EncodeError>(())` tail at the five sites (the unordered.rs example runs inside a `block_on`; return the `Result` from the async block as rumors.rs:582-614 does). To enforce the convention, add `#![doc(test(attr(deny(unused_must_use))))]` to lib.rs rather than flags to the `doctest` recipe. Acceptance: `grep -rn '\.send("' src | grep -v '?;'` returns nothing; `just doctest` passes with the crate attribute in place.

### api-core-33: Three observers re-spell one `Channel` state machine; `Changes` implements it twice
- Where: src/rumors/unordered.rs:197-226 (related: src/rumors/unordered.rs:40, 69-74; src/rumors/causal.rs:41, 165-174, 187-193; src/rumors/changes.rs:48, 75-104, 112-122, 133-145, 156-162)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read all three `poll_next` bodies; grep: `channel state present` at four sites, `matched Ready above` at three; `next_inner` is defined at changes.rs:76 and called only at changes.rs:117)
- Seen by: structure, correctness, perfapi; refutation: confirmed; history: no rationale found for the duplication (`Channel` was shared at aa321d2ba but its transitions stayed inline; cd7c09db3 rewrote both message observers without consolidating and did not touch changes.rs); `next_inner` is deliberate-but-expired (it was the engine of a `Changes<T, Blocking>: Iterator` impl deleted at 83edcd944)
- Owner-gated: no

The `Waiting` arm (poll the boxed wait, restore `Ready`, end on `closed`) and the enter-owned-wait block (`take()` the channel, `unreachable!` on the impossible variant, `Box::pin` the `changed()` future) are copied into all three `poll_next` bodies, each with its own `expect("channel state present")` and `unreachable!("matched Ready above")`. `Changes` additionally carries a second, `async fn` implementation of its whole state machine (`next_inner`) whose only caller is `try_next`, whose doc claims to share the `Stream` state machine, and which awaits `rx.changed()` by borrow where `poll_next` materializes the owned wait; its siblings implement `try_next` as `self.next().now_or_never()`. A fix to the wait discipline must land in three places plus `next_inner`; the seven `expect`/`unreachable!` sites exist only because each observer re-implements the transition around an `Option` it owns solely to `take()` from; and each `poll_next` should read as its domain step, not channel plumbing.

Evidence:

   197	            match this.channel.as_mut().expect("channel state present") {
   198	                Channel::Waiting(wait) => match wait.as_mut().poll(cx) {
   199	                    Poll::Pending => return Poll::Pending,
   200	                    Poll::Ready((closed, rx)) => {
   201	                        this.channel = Some(Channel::Ready(rx));
   202	                        if closed {
   203	                            return Poll::Ready(None);
   204	                        }
   205	                    }
   206	                },

   220	                    let Some(Channel::Ready(mut rx)) = this.channel.take() else {
   221	                        unreachable!("matched Ready above");
   222	                    };
   223	                    this.channel = Some(Channel::Waiting(Box::pin(async move {
   224	                        let closed = rx.changed().await.is_err();
   225	                        (closed, rx)
   226	                    })));

    75	    /// Await the next coalesced change, sharing the [`Stream`] state machine.
    76	    pub(crate) async fn next_inner(&mut self) -> Option<()>

Resolution: Move the transitions onto the shared type: make `Channel<T>` own its optionality and give it `fn poll_receiver(&mut self, cx) -> Poll<Option<&mut watch::Receiver<Inner<T>>>>` (drives a `Waiting`, restores `Ready`, yields `None` once closed) and `fn wait(&mut self)` (moves `Ready(rx)` into the boxed `changed()` future); each observer's field becomes a plain `Channel<T>` and each `poll_next` becomes `let Some(rx) = ready!(this.channel.poll_receiver(cx)) else { return Poll::Ready(None) };`, the domain step, and `this.channel.wait()`. Delete `Changes::next_inner` and write its `try_next` as the siblings do. Host `Channel` in rumors.rs or a `channel` submodule rather than `pub(super)` inside unordered.rs. Acceptance: `grep -c 'channel state present' src/rumors/` and `grep -c 'matched Ready above' src/rumors/` both return 0; `next_inner` is gone; the observer suites and the doctests in unordered.rs and changes.rs stay green.

### api-core-34: `Snapshot`'s `IntoIterator` names an `Iter` no public path reaches, while `iter()` hides the same type
- Where: src/snapshot.rs:4-7 (related: src/snapshot.rs:105-112, src/snapshot.rs:172-179, src/lib.rs:317, src/lib.rs:322, src/lib.rs:347, src/tree.rs:167-176)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (lib.rs:317 `mod snapshot;` and 322 `mod tree;` are private; the re-export block at 328-349 exports only `Snapshot` from snapshot and `MERKLE_HASH_LEN`/`SessionStats` from tree; no external use of `snapshot::Iter` or `rumors::Iter` in tests, examples, or benches)
- Seen by: structure, prose, perfapi; refutation: confirmed; history: deliberate-but-expired (cb69fc951 re-exported `Iter` at the root; 8dc0596ed removed that re-export and made `Snapshot::iter` return an opaque type "to move engine internals out of the user-facing prose", leaving this `pub use`, its doc, and the `IntoIterator` impl as unswept remainder)
- Owner-gated: yes: either adds a public type name or removes a public trait impl

`snapshot.rs` re-exports `crate::tree::Iter` with a doc calling it "re-exported from the tree internals", but no public path reaches it, so the doc describes a re-export nobody outside the crate can see and names internals. Meanwhile `impl IntoIterator for &Snapshot<T>` sets `type IntoIter = Iter<'a, T>` (a type users can spell only as `<&Snapshot<T> as IntoIterator>::IntoIter`) while `Snapshot::iter` returns `impl DoubleEndedIterator + ExactSizeIterator + Send + Sync`: one iterator presented two ways, and a caller who wants to store it in a struct field can do so through neither.

Evidence:

    4	/// The iterator of [`Snapshot::iter`], re-exported from the tree internals:
    5	/// every live message as `(&Version, Arc<T>)`, unspecified order,
    6	/// exact-size and double-ended.
    7	pub use crate::tree::Iter;

   172	impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
   173	    type Item = (&'a Version, Arc<T>);
   174	    type IntoIter = Iter<'a, T>;

Resolution: History already chose opacity, so the consistent completion is to delete the `pub use` with its doc and the `IntoIterator` impl (callers write `snapshot.iter()`). The alternative, exporting `Iter` at the crate root and returning it from `iter()` per std's naming convention, reverses 8dc0596ed and is the owner's to weigh. Acceptance: either `IntoIterator for &Snapshot` and snapshot.rs:4-7 are gone and the doctests using `snapshot.iter()` still compile, or `rumors::Iter` appears in the public index and `Snapshot::iter` names it; in both cases no public doc says "tree internals".

### api-core-35: `Snapshot` equality includes the causal ceiling, undocumented, while `hash` documents excluding it
- Where: src/snapshot.rs:16-20 (related: src/snapshot.rs:69-82, src/tree.rs:131-135, src/tree.rs:146-150, tests/routed_link.rs:250, .agent-notes/2026-07-23-review-link-transport/review-link-transport-branch.md:299-301)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read `Root::eq`: it compares `ceiling` as well as `root`; `Tree::eq` delegates to it; the hash docs at 73-79 say the hash excludes the frontier; the one test use of `==` is tests/routed_link.rs:250 after convergence)
- Seen by: correctness; refutation: confirmed; history: no rationale found for the semantics; link-transport review R13 relies on `Snapshot: Eq` and characterizes it as "set equality", which is not what the code computes, so the review's assumption strengthens the documentation half
- Owner-gated: yes: defines or removes a public trait's semantics, and removal reopens R13

Two snapshots of the same live set at different frontiers compare unequal under `==` (the ceiling advances on redactions and survives a tree emptying) while `hash()` compares equal, and nothing on the type says what `==` means. The hash doc goes out of its way to say the frontier is excluded, which makes the silence about `==` conspicuous.

Evidence:

    16	#[derive(Clone, Debug, PartialEq, Eq)]
    17	pub struct Snapshot<T> {

   131	impl PartialEq for Root {
   132	    fn eq(&self, other: &Self) -> bool {
   133	        self.ceiling == other.ceiling && self.root == other.root
   134	    }
   135	}

    77	    /// covers the live set only, not the causal frontier: two replicas at
    78	    /// different points in causal time can share a hash — compare
    79	    /// [`latest`](Self::latest) for history.

Resolution: Owner decision: either document `==` on the type as "same network, same live set, and same frontier (`latest`)", or drop `PartialEq`/`Eq` and rewrite tests/routed_link.rs:250 as `hash()` plus `latest()` equality. Acceptance: the type docs state what equality means, or the impl is gone and R13's test is re-expressed.

### api-core-36: `Snapshot::earliest` has no public-tier test
- Where: src/snapshot.rs:54-56 (related: src/tree/tests.rs:458-466)
- Class / severity / confidence: verification-gap / nit / high
- Provenance: verified (grep: `earliest` appears in tests/ only in prose in tests/hop_trace.rs; no call in tests/ or src/tests.rs)
- Seen by: correctness; refutation: confirmed; history: no rationale found
- Owner-gated: no

The method is a pure delegate, so drift risk is small, but its two-clause contract (a floor every live version contains; `None` iff empty) is pinned only at the tree layer, and it is the one `Snapshot` accessor with zero public exercise. Differential and exhaustive suites exercise the public API so the public wiring cannot drift unnoticed.

Evidence:

    54	    pub fn earliest(&self) -> Option<&Version> {
    55	        self.tree.earliest()
    56	    }

Resolution: Add to an existing single-peer proptest: `snapshot.earliest().is_none() == snapshot.is_empty()`, and every `iter()` version `>= earliest`. Acceptance: a tests/ assertion calls `Snapshot::earliest` and its testdoc states the contract.
Construction: In tests/single_peer.rs's batch proptest, after each commit, `let s = peer.snapshot(); prop_assert_eq!(s.earliest().is_none(), s.is_empty()); if let Some(e) = s.earliest() { for (v, _) in s.iter() { prop_assert!(e <= v); } }`.

### api-core-37: `Snapshot` docs advise sorting by `Version`, which has no `Ord`
- Where: src/snapshot.rs:101-104 (related: src/snapshot.rs:129-132, crates/before/src/version.rs:1729-1760, crates/before/src/version/ranked.rs:372-383, src/rumors/causal.rs:54-63, tests/single_peer.rs:85-87)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `before` for `impl Ord for`: only `Ranked<'_>`, `Rank`, `Base`, and test oracles; `causal_cmp_impls!` at version.rs:1758 gives `Version` only `PartialOrd` via the causal partial order)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (inaccurate from birth: `Version` has never had `Ord`)
- Owner-gated: no

Both `iter` and `range` tell the reader to "Sort by the yielded [`Version`]s if your application needs an ordering consistent with causality." `Version` implements only the causal partial order, so `versions.sort()` does not compile and `sort_by(|a, b| a.partial_cmp(b).unwrap())` panics on any two concurrent versions (tests/single_peer.rs:85-87 gets away with it only for a lone peer's versions). The total order the crate itself uses is `Version::ranked()` (`before::Ranked: Ord`), and neither site names it.

Evidence:

   101	    /// Order is unspecified, and in particular does *not* follow the causal
   102	    /// order: a message may be yielded before another that causally precedes
   103	    /// it. Sort by the yielded [`Version`]s if your application needs an
   104	    /// ordering consistent with causality.

Resolution: Name the order at both sites: "sort by [`Version::ranked`] (a total order extending causality: the order [`CausalMessages`] delivers in)", with a two-line example (`items.sort_by(|a, b| a.0.ranked().cmp(&b.0.ranked()))`). Acceptance: a doctest at the `iter` site sorts a three-message snapshot containing two concurrent sends and asserts the causal predecessor comes first.

## Positives

- `Bootstrap`/`BookmarkedBootstrap` argue the type-state split at the type (bootstrap.rs:269-273: each state's `join` declares only its own outcomes), and every manual impl carries its reason: the unbounded `Clone` (81-83) because the payload is phantom, the `Debug` showing the bookmark by `type_name` (286-287) because `Bookmark` does not require `Debug`.
- `Batch` is a closed scope handle enforced by the borrow checker: the two `compile_fail` doctests (rumors.rs:320-336) pin both escape routes, and `commit` reads the changed flag straight from `Tree::act` with the no-root-hash-read invariant metered in src/tests.rs:644-655 (with a liveness leg at 626-635).
- `try_into_peer_inner` (rumors.rs:108-131) gets the hard ordering right and says why at the fields: subscribe before shedding, shed before waking. api-core-22 dissolves the need for the second rule; the reasoning as written is correct.
- The observers state the checkpoint discipline once at the field (causal.rs:46-53), mirror it in `UnorderedMessages`, and pin it on both faces at both boundaries; `folding_delivered_versions_can_lose_a_message` in tests/listen.rs is a committed demonstration that the tempting wrong implementation fails. `CausalMessages`' ordering promise rests on a stated theorem (`before::Rank` strictly monotone in the causal order, rank.rs:152-156) plus a two-line cross-pass argument.
- `Network` reserves its all-zero bootstrap sentinel structurally (`from_rng` redraws) and keeps the serde form to one opaque byte string with the rationale beside it (network.rs:27-29).
- error.rs opens with a per-variant table of replica state and the action beyond reconnecting (10-28), exactly the altitude a caller matching on `Error` needs; the "counterparty bug: report it" rows keep the conformance-detector framing of the model of record. `Error::Epilogue`'s doc (124-159) is a clear statement of the two-generals residue and what the exchange protects. `Error::widen` closes the uninhabited arm with `match never {}`, so adding a variant cannot mis-widen silently.
- `Snapshot`'s cheapness claim is mechanically true (a `Root` clone is a `Bytes` refcount bump plus an `Arc` bump; `len` is a stored count; hash and bounds are `OnceLock` memos), and `Snapshot::hash`'s claim that the frontier is excluded is true at the mechanism (`root_hash` hashes nodes only; the ceiling rides outside `Root`).
- Every `must_use` names the concrete loss: `Bootstrap`/`BookmarkedBootstrap` (does nothing until `join`), `Joined` (a peer or the bookmark), `Retire` (leaks the identity), `Unbookmarked` (strands the identity), `gossip_when` (the driver does nothing until polled).
- `Peer` and `Rumors` `Debug` impls print a bounded summary independent of `T: Debug`, the right default for a replica that may hold millions of messages.
- changes.rs:36-43 ("This signal alone does not make a gossip driver") names the concrete deadlock a naive loop produces and points at `gossip_when`: a hazard section that earns its space. rumors.rs:196-207 explains a deliberate API omission (`send` returns no `Version`) with the intended pattern and the batching argument.
- tutorial.rs: every step is a complete program with its exact output, the final program is the cumulative one, and step 5's "one driver per end" is called out as the thing to remember. The `Peer` lifecycle example (peer.rs:47-116) covers all `Retire` outcomes and the mutual-bootstrap bail in one runnable block.
- benches/in_memory.rs covers every in-memory public operation in this partition, and tests/api_send_bounds.rs pins the `Send`/`Sync` contract of the handle types and every async entry point at compile time; api-core-5's acceptance test extends exactly that shape.

## Open questions for Finch

- `Inner.party: Option<Party>`: the `None` state exists only while `Peer::retire` holds the party in flight, yet it forces two unreachable arms (batch.rs:136-139, api-core-2; gossip.rs:825-828, "keeps the arm total without a panic path") that each need a proof, and the two arms chose different shapes (silent `return false` versus adopt-the-donation). Holding the in-flight party in the retire future and keeping `Inner.party` total would delete both. I did not size the change against `PartyGuard` and the bookmark critical sections. Recommendation: rule once for both arms now (api-core-2's `expect` is the cheap consistent fix), and schedule the dissolution as a design item.
- `sync_memory_budget`'s sizing guide (peer.rs:371-465 plus the included table): the rulings of 2026-07-23 and 2026-07-24 placed the derivation in this rustdoc. The prose lens's altitude case for moving "# Choosing a budget" to a docs-only `sizing` page beside `reconciliation`, leaving the method with its ~40-line contract and a link, is worth reconsidering now that the crate has docs-only pages; api-core-15 fixes only what the rulings did not cover. Recommendation: move it; the rulings were about where the derivation lives relative to code, not about which rustdoc page.
- `Protocol` after the V1 retirement: decision 2 keeps the enum public and `#[non_exhaustive]` as wire vocabulary, which closes the lenses' question. The retirement note's status line (README.md:7-8) says the `Protocol` enum "went with" the builders, which the tree contradicts; agent notes are exempt from the prose rules, but a reader following that line will be misled. Recommendation: one-line correction in the note.
- batch.rs:22-26 is your own wording (c927ba04, a GitHub web edit): "concurrent gossip rounds" where the crate elsewhere says sessions, and "holds no lock" in two consecutive sentences. Not swept as a finding because it is your voice; do you want it aligned to "sessions" and the repetition folded?
- lib.rs:45-54 states scaling claims marked "derived" ("run stale in proportion to roughly the square of the bandwidth shortfall", "falling roughly as the inverse square root of the backlog"). I found no committed instrument or pinned measurement in tests/ holding them. Is there one, or should the doc state the qualitative behavior and leave the derivation to a design note?
- Lock scope of `Batch::commit`: the whole `Tree::act` runs inside `send_if_modified`'s write lock, so `snapshot()`, the `Debug` impls, and observer `borrow_and_update` calls block for the commit's duration; a 10^5-message `send_all` is O(N log N) under the lock. Inherent to the design (insert paths derive from post-tick versions read under the lock), so no finding; is a measured figure for the large-batch case wanted before persistent storage lands?
- api-core-17 (first-sentence mood) and api-core-34/35 (`Iter`, `==`) each need a ruling before work starts; my recommendations are in the findings (third person per the doctrine; delete the `IntoIterator` impl; document `==` rather than drop it).

## Dropped

- Structure [30]/[36] (`Changes::next_inner` duplicate) and [41] (enter-the-owned-wait block): duplicates of api-core-33.
- Prose [11], correctness [28], perfapi [39] (Protocol selection ghosts): duplicates of api-core-4; the `Default` derive half of structure [2] is api-core-21.
- Prose [15], perfapi [37] (unnameable `Iter`): duplicates of api-core-34.
- Structure [4] and correctness [26] parts (a) and (b) (`Snapshot` derive bounds and O(n) `Debug`): merged into api-core-5; part (c) (ceiling-inclusive `==`) is api-core-35.
- Correctness [29] (bootstrap plumbing module doc): duplicate of api-core-20.
- Prose [22] part (b) (batch.rs "concurrent gossip rounds", doubled "holds no lock"): owner-authored wording (c927ba04); moved to open questions rather than swept. Part (d) (`cryptographically impossible`) merged into api-core-11.
- Prose [21]'s `static_assertions` comment: merged into api-core-9 with the dependency-placement question the prose lens raised.
- Prose [12]'s proposal to relocate the sizing guide: reopens the 2026-07-23/24 rulings without new evidence; moved to open questions, with the three unruled sub-items kept as api-core-15.
- Prose [16]'s "pick one home" framing: the writing doctrine (writing-style.md:219-230) mandates repeating hazards with a link to one canonical explanation; kept as api-core-27 in the doctrine-compatible form (one canonical, one summary).
- Prose [19]'s direction (align third-person sites to the imperative): the doctrine (writing-style.md:122-124) fixes third person; kept as api-core-17 with the direction reversed and marked owner-gated.
- Perfapi [42]'s mechanism (`-D warnings` missing from the doctest recipe): refuted by the refutation pass (rustdoc's injected `#![allow(unused)]` is why the warnings never fire); kept as api-core-32 with the crate-attribute fix.
- Structure [6]'s variant tally ("eighteen"): the enum has a different count and no tally belongs in the write-up; kept as api-core-7 without the number.
- Lens open question on `Protocol` remaining a one-variant enum: already ruled (retirement note decision 2); recorded under open questions only for the note's own inaccuracy.
- Lens open question on whether `Bootstrap::observe` is retained by the joined peer's later sessions: the correctness lens did not read the callers and neither did I beyond tests/observe.rs:275-289; api-core-20 asks for a direct retention assertion, which settles it.
- `try_into_peer` sitting in the `impl<T, B: Bookmark>` block (rumors.rs:427) though its body needs only `BookmarkError`: below the bar; a `Peer<T, B>` with a non-`Bookmark` `B` is unconstructible, so the stricter bound costs nothing.
- perfapi's wire-ingress mirror of api-core-10 (`Message::from_wire` storing sliced run buffers): out of this partition; belongs to the streaming decoder's reviewer.
