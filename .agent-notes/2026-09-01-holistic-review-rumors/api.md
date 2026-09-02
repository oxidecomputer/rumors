# API surprises and expected features

This document collects every finalized finding of the rumors review whose primary class is api-surprise or feature-gap: the places where the public API of the `rumors` crate at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 surprised a competent user, and the things such a user expects and does not find. Ids take the form `<partition or sweep key>-<n>`; the full record for each, with its lens history and refutation notes, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. Severities are the finalizers' own and are reproduced unchanged. As they applied the scale in these two classes, *medium* marks a public contract or signature that fails a legal use, or a capability the docs promise and the crate does not ship (a derive-added bound a legal type cannot meet, a type no user can name, a trait shape every implementor pays for, a validation suite the module doc announces); *low* marks a surprise with a workaround or a gap a user can fill from outside (a missing accessor or counter, an unstated rule, a hidden but reachable item, a harness that reimplements what the crate could offer); *nit* marks a convenience or idiom gap. No entry in these classes is high. Provenance is stated per entry: *verified* means the finalizer ran a command or checked the claim mechanically (grep, a read of the rendered rustdoc, a read of pinned library sources); *assessed* means the claim rests on reading; *demonstrated* would mean a constructed test was run against this commit, and no entry here carries that mark, because the two witness passes built constructions for correctness, verification-gap, test-quality, and one simplification finding, and none targeted an entry in these two classes. Every entry in this document is owner-gated by nature: the crate is pre-release, and each resolution adds to, narrows, or renames public API, so adopting any of them is Finch's decision. The finalizers' own `Owner-gated` bullet is nonetheless kept verbatim, because a handful of entries are marked not gated (test-scaffold, example, and bound-loosening changes) and the reasons ride with the bullet. Each entry also carries a `Recommendation` bullet, which is this document's verdict: *adopt* (the change is small, widening, or restores a promise the docs already make), *consider* (a design choice with more than one defensible answer; the bullet says which this document recommends), or *decline* (with the reason). Every quoted evidence line below was checked mechanically against the file at the commit and every flagged attribution re-read by hand; one entry's evidence line numbers were off by one and are corrected in place (conformance-1).

## The fresh-eyes application

The fresh-eyes sweep read the crate as a first-time user would (README, crate docs, tutorial, every public item's rustdoc) and wrote a 527-line application against those docs alone; its artifacts sit under `sweeps/fresh-eyes/` in the review's scratch directory. The application's module doc states what it exercised:

    //! Exercises: seeding, a bookmarked bootstrap over the in-memory link,
    //! change-driven gossip on both ends while both sides send and redact,
    //! the three set observers, snapshot lookups, a file-backed `Bookmark`
    //! restored across a simulated crash, a routed TCP session, and the
    //! `try_into_peer` / `retire` lifecycle end.

It compiled on its first `cargo check` except for two deliberate probes, and the finalizer confirmed from the log that those were the only two `error[` lines. The first probe asked for the iterator type behind `Snapshot`:

    error[E0603]: module `snapshot` is private
       --> src/main.rs:536:32
        |
    536 |     let _named: Option<rumors::snapshot::Iter<'static, Note>> = None;
        |                                ^^^^^^^^  ---- struct `Iter` is not publicly re-exported

That is fresh-eyes-4 below, and the same defect reached this review from three other reports (tree-core-5, inventory-1, api-audit-2). The second probe followed the docs' instruction to "select" a protocol:

    error[E0599]: no method named `protocol` found for struct `Peer<T, B>` in the current scope
       --> src/main.rs:538:38
        |
    538 |     let _peer = Peer::<Note>::seed().protocol(rumors::Protocol::V2);

That one is a documentation finding rather than an API one (fresh-eyes-2, with api-audit-5 and prose-hygiene-2 in the documentation document): the `.protocol()` builders were deliberately removed with the V1 protocol, and the prose that told users to select one was not swept.

What the application had to write by hand is as telling as what failed to compile. Its file-backed `Bookmark` implements `store` by draining the lent writer into a `Vec` before it can do the one thing a bookmark store does:

        let mut buf: Vec<u8> = Vec::new();
        {
            let sink: &mut (dyn AsyncWrite + Unpin + Send) = &mut buf;
            write(sink).await?;
        }
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, &buf)?;
        std::fs::rename(&tmp, &self.path)?;

Every `Bookmark` implementor in the tree does the same, and the crate itself hands `store` a `Vec` it has already encoded (session-bookmark-23). The application also implemented two traits, `BookmarkError` and `Bookmark`, to supply one associated type (api-audit-4), and a retry after `Joined::Bailed` would have needed a builder the outcome does not return (fresh-eyes-9). Its run log records what worked as the docs said: both drivers ended cleanly on stream end and on remote hang-up; concurrent one-shot `gossip` on both ends merged into one session with both sides reporting `Led::Local`; `bytes_sent` on one end equalled `bytes_received` on the other over TCP (69/69); and redactions issued on each side were honored on both within one session. The sweep's one runtime question, why the bookmark file stayed at 68 bytes across three sessions carrying the peer's own sends, was answered by reading (the writes ran, and the advanced version encoded to the same width) and is recorded under the fresh-eyes sweep's open questions rather than here.

## Highest-value items

1. `#[derive]` on the generic wrappers imports bounds their representations never use: `rumors::Error<B>` is `Debug`, and therefore `std::error::Error`, only when the bookmark type is `Debug`, so `?` into `Box<dyn Error>` stops compiling for a legal bookmark, and `Snapshot<T>` refuses to clone for a non-`Clone` payload although it is a structure-sharing view; the crate states the counter-rule in its own hand-written impls (api-core-5; the same defect from the sweep and the tree partition is api-audit-1 and tree-core-2).
2. `Snapshot::iter` returns an opaque type while `IntoIterator for &Snapshot<T>` names `tree::Iter`, which no public path reaches, so a user can hold the iterator only as `<&Snapshot<T> as IntoIterator>::IntoIter`; rustc's `unnameable_types` lint would make the class a compile error (tree-core-5; the same defect from three other reports is fresh-eyes-4, inventory-1, and api-audit-2).
3. `Bookmark::store` asks every implementor to accept a higher-ranked closure over a trait-object writer and to box a future, so that the crate can hand it a `Vec` it has already encoded; an owned byte parameter carries the same atomicity obligation (session-bookmark-23).
4. `BookmarkError` is a trait implemented by bookmarks, not by errors, whose only content is `type Error`; every public generic reads `B: BookmarkError`, and a user implements two traits to supply one associated type (api-audit-4).
5. The `conformance` module doc promises one validation suite per caller-implementable boundary and ships one, for `Link`; `Bookmark`'s commit-iff-`Ok` clause is exactly the kind a deployment gets wrong over a file or KV store, and the crate classifies its violation as unspecified corruption (conformance-1).
6. Methods on error types re-exported from `rumors::error` return types no user can name (`signal::StreamError`, `Signal`), one of them sharing its simple name with a public sibling, and two `Display` impls route through `Debug`, so a variant rename rewrites a user-visible message (remote-codec-30).
7. `warm_caches` and `seed_rng` are `#[doc(hidden)] pub` with no feature gate while their siblings `sync_window_floor` and `dangerously_alias_party` are gated on `test-internals`; hidden items are still semver surface, and `seed_rng` puts rand's `RngCore` on it (api-audit-15; the same sites from three other reports are benches-envelope-6, tests-lifecycle-10, and deps-5).
8. The rule deciding which public error enums are `#[non_exhaustive]` lives only in commit 1e458d69's message, so `EncodeError`, the admission taxonomy a caller matches on, is closed although it grows with enforcement, and `SendError` is closed while its incoming twin is open (session-bookmark-46; remote-codec-18, remote-capture-atlas-35, and remote-adapter-streams-22 file the rule and its other stragglers).
9. The public error taxonomy carries variants no public path produces (`GreetingError::Order`; `RemoteError::PayloadDepthMismatch` under `Error::Mirror`) and does not say which variants diagnose the local participant and which the peer, so a user filing a conformance bug cannot tell whose bug it is (remote-codec-28; remote-proxy-2, remote-proxy-3, and api-audit-13 file the rest of the pattern).
10. `Snapshot`'s `==` includes the causal ceiling, so two snapshots of the same live set at different frontiers compare unequal while `hash()` documents excluding the frontier, and nothing on the type says what equality means (api-core-35).
11. `SessionObserver` has no end-of-session hook carrying the outcome, so the shipped tracing adapter cannot set span status and a recorder cannot tag a capture as complete or aborted (session-bookmark-38).
12. A configured peer's settings cannot be read back, the effective (saturated) run budget has no getter, and `SessionStats` reports neither frame counts nor whether the window ever bound the session, so a user tuning `sync_memory_budget` or `target_message_size` gets no feedback from the crate's own instruments (api-audit-11; remote-codec-5, remote-adapter-streams-21, and materialized-2 are the other three readouts).

## Crate-wide patterns

- **Derived impls on generic wrappers bind `T` and `B` where the representation never inspects them.** The sites are `#[derive(Debug, thiserror::Error)]` on `Error<B>` (src/error.rs:61-63), `#[derive(Clone, Debug, PartialEq, Eq)]` on `Snapshot<T>` (src/snapshot.rs:16-17), `#[derive(Debug, Eq)]` on `Tree<T>` (src/tree.rs:90), and the `Debug` derives on `Retire<T, B>` (src/peer/gossip.rs:93-94), `Unbookmarked<T, B>` (src/peer/gossip.rs:149-150), and `Joined<T, B>` (src/peer/bootstrap.rs:365-366). The crate writes the rule down at src/peer/bootstrap.rs:81-84 and follows it for `Peer`, `Rumors`, and `Bootstrap`. Entries: api-core-5, api-audit-1, tree-core-2; materialized-17 asks the same question of the internal error enums, where the missing derives are `Clone`, `PartialEq`, and `Eq` rather than surplus bounds.
- **Public signatures name types with no public path, and nothing catches the class.** `tree::Iter` reaches users only through `IntoIterator for &Snapshot<T>` (src/snapshot.rs:172-174); `signal::StreamError` reaches them through `Stream::new` and `Signal` through `InvalidSignalPlacement::signal` (src/tree/mirror/streaming/remote/codec/signal.rs:41, :375). Four reports filed the `Iter` case independently (tree-core-5, fresh-eyes-4, inventory-1, api-audit-2); remote-codec-30 and api-audit-2 file the error-module cases. Every entry proposes the same committed check, rustc's allow-by-default `unnameable_types` lint enabled in src/lib.rs so the clippy leg's `-D warnings` fails the next occurrence. The idiom document records the same `Iter` defect once more as api-core-34.
- **The `#[non_exhaustive]` rule is recorded only in commit 1e458d69's message.** Open today: `DecodeErrorKind`, `ListingIssue`, the adapter's `DecodeError<E>`, `StreamError`, `AcceptError`, `GreetingError`, the proxy's `Error<E>`, `FrameDefect`, `RecordDefect`, `FormatError`, and `Error<B>`. Closed with no stated reason: `EncodeError` (src/message.rs:116-117), `SendError` (src/tree/mirror/streaming/remote/streams.rs:238-239), the adapter's `EncodeError<E>` (src/tree/mirror/streaming/remote/adapter/error.rs:43-44), `EncodeErrorKind` (src/tree/mirror/streaming/remote/codec/error.rs:64-65), `Origin`, `FramePart`, `DecodeLeafError`, `LeafRunError`, `StreamClass`, `DecodeSignalError`, `ScopeError`, `OpeningError`, `ReplyFrameError`, `HeadError`, and `MaterializedError` (src/tree/mirror/streaming/materialized/error.rs:1-8). Entries: session-bookmark-46, remote-codec-18, remote-capture-atlas-35, remote-adapter-streams-22, and the exhaustiveness half of materialized-17. The documentation and idiom documents carry the same rule from their angles (api-audit-10, link-17).
- **Hidden but ungated calibration hooks on the production handles.** `Peer::warm_caches` (src/peer.rs:713-716), `Rumors::warm_caches` (src/rumors.rs:413-416), `Snapshot::warm_caches` (src/snapshot.rs:166-169), and `Peer::seed_rng` (src/peer.rs:210-213) are `#[doc(hidden)] pub` with no `cfg`, while `sync_window_floor` (src/peer.rs:480-483) and `dangerously_alias_party` (src/peer.rs:726-728) are gated on `any(test, feature = "test-internals")`; every caller of the four is a bench or an integration test, and those already build with the feature (Cargo.toml:145). Entries: api-audit-15, benches-envelope-6, tests-lifecycle-10, deps-5. The vestigial and documentation documents file the same sites as inventory-4 and async-hazards-4, and open question 6 records that the reports disagree on gating versus documenting `seed_rng`.
- **Error variants no public path produces, and an origin the taxonomy does not state.** `GreetingError::Order` is constructed at src/tree/mirror/streaming/remote/codec/greeting.rs:153 and stripped at :277 before any caller sees it (remote-codec-28); `RemoteError::PayloadDepthMismatch` is lifted into `Error::PayloadDepthMismatch` at src/peer/gossip.rs:1423-1429, so its arm under `Error::Mirror` is dead (remote-proxy-2); `HandOffDefect::Undecodable` admits `Decode::Io`, which `decode_party` routes to `Error::Io` instead (mirror-common-15); three `Infallible` backend slots in `MirrorError` force `match never {}` arms (api-audit-13); and `RemoteError`'s variants are not classified by whether they diagnose this crate's own participant or the peer, with `TerminalQuery` raised on both sides (remote-proxy-3). From other documents: remote-adapter-streams-19 (simplification) deletes `ReplyFrameError`, a variant with no producer; remote-proxy-29 (vestigial) shows the decode-side `TerminalQuery` is unreachable from wire bytes; fresh-eyes-10 (documentation) is the module-doc misstatement of what reaches `Error::Mirror`.
- **What the crate configures and measures cannot be read back.** There is no getter for `sync_memory_budget`, `target_message_size`, or `payload_depth_limit`, and the `Peer` and `Rumors` `Debug` impls omit them (api-audit-11); the run budget saturates at a `MAX_RUN_BUDGET_BYTES` that is neither re-exported nor readable (remote-codec-5); `SessionStats` has no frame counts (remote-adapter-streams-21) and no window-stall signal (materialized-2); the routed link's dialer never sees the `Token` the acceptor reports (link-21), and router evictions leave no count (link-16). `SessionStats` is `#[non_exhaustive]`, so its additions are non-breaking.
- **`Rumors::send` returns no `Version`, by recorded decision, and the crate's own tests recover it six ways.** The shapes are `created_version` (tests/common/action.rs:47-64), `Peer::insert_one` (tests/common/peer.rs:77-87), a by-payload scan in `shape::pool` and tests/gossip_snapshot.rs, `.iter().map(|(v, _)| v.clone()).next()` (tests/retire_redaction.rs:25-30, tests/single_peer.rs:216-221), `find_map` by payload (tests/retire.rs:194-198, tests/single_peer.rs:357-364), and an inline `range(causally::since(&pre))` (tests/pairwise.rs, tests/single_peer.rs:20-27). The rationale at src/rumors.rs:190-207 holds for the state-machine half; its batching half does not bind the single-message method. Entries: tests-common-2, tests-lifecycle-31.
- **`pub` on items nothing outside the crate can reach, documented in library-user voice.** `Message` and eight of its methods (src/message.rs:47-51; session-bookmark-44), and the codec's error constructors and schedule helpers `Origin::{direction, stream}`, `CodecEncodeError::new`, `Stream::{new, at_height, height}`, and `Speaker::other` (api-audit-14). The modularity and idiom documents carry the crate-wide convention (inventory-10, module-graph-12, streaming-backend-window-2, tree-typed-1); `#![warn(unreachable_pub)]` is the committed check each proposes.
- **Bootstrap and retire sessions are reimplemented per test suite.** `tests/common/wire.rs` offers no driver that returns the newcomer as a `Peer`, hands back the mutual-bootstrap `None`, takes a configured builder, or retires one peer into another, so tests/hop_trace.rs, tests/bookmark_when.rs, tests/bootstrap.rs, tests/retire.rs, tests/retire_redaction.rs, tests/observe.rs, and the schedule executor each carry a copy, and two of the copies skip the control-drain assertion (tests-common-30, tests-observation-11; tests-lifecycle-3 in the simplification document is the same family seen from the suites).

## Crate root and public surface (lib, peer, rumors, batch, snapshot, network, tags, protocol, error, tutorial)

### api-core-1: `Batch::redact_all` accepts only `&Version` items although callers usually hold owned versions
- Where: src/batch.rs:112-115 (related: src/rumors.rs:279-283, src/peer.rs:668-671)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: assessed (read)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (212c6914 introduced the signature without discussing the item type)
- Owner-gated: yes: widens a public signature
- Recommendation: adopt: `I::Item: Borrow<Version>` is the std idiom, strictly widening, and the existing doctest keeps compiling.

The bulk redaction entry takes `I: IntoIterator<Item = &'v Version>`, but the natural sources of versions to redact yield them owned: the observers' items are `(Version, Arc<T>)`, and a `Vec<Version>` moved into a task must be re-borrowed with `.iter()`. `Version::clone` is an O(1) refcount bump and `Path::for_leaf` needs only a borrow, so `Item: Borrow<Version>` (the `HashSet::remove<Q>` idiom) accepts both forms at no cost and keeps the existing `&evens` doctest compiling.

Evidence:

    112	    pub fn redact_all<'v, I>(&mut self, versions: I)
    113	    where
    114	        I: IntoIterator<Item = &'v Version>,
    115	    {

Resolution: On both `redact_all`s, `I: IntoIterator, I::Item: Borrow<Version>`, calling `self.redact(version.borrow())`. Acceptance: the rumors.rs:263-278 doctest passes unchanged, and a second example `rumors.redact_all(evens)` over an owned `Vec<Version>` compiles.

### api-audit-13: `MirrorError`'s public shape carries meaning the user cannot read off it
- Where: src/error.rs:52-53 (related: src/error.rs:283-289, src/tree/mirror.rs:41-50, src/tree/mirror/streaming/remote/proxy/error.rs:31-41, src/tree/mirror/streaming/materialized/error.rs:1-8, src/tree/mirror/streaming/remote/adapter/error.rs:44-47, src/tree/mirror/streaming/remote/adapter/error.rs:59-62, src/peer/gossip.rs:1154, src/peer/gossip.rs:1165, src/peer/gossip.rs:1212, src/peer/gossip.rs:1224, src/peer/gossip.rs:1406-1431)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -rn 'Error::Mirror\|streaming_error\|MirrorError' src` outside tests shows `streaming_error` as the only constructor of `Error::Mirror` and the four `.map_err(streaming_error)` sites as its only callers; `.flip()` has no non-test caller; `error/type.MirrorError.html` shows the aliased enum's variants without a mapping sentence)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: changes the public error shape or its docs
- Recommendation: consider, as part of open question 4; the recommendation is a concrete public `MirrorError` (local violation or wire failure) constructed at `streaming_error`, which removes the `Infallible` arms and the dead `PayloadDepthMismatch` arm together (remote-proxy-2).

Three facts about the mirror taxonomy are invisible from its public docs.
The alias fixes `Client` to `MaterializedError` (the local participant: a
`Violation` it detected in the peer's replies) and `Server` to
`RemoteError` (the wire proxy), but neither the alias doc nor
`Error::Mirror` says so, and the variant docs speak of "position", an
internal vocabulary. Three backend-error slots are fixed to `Infallible`
(`MaterializedError::Backend`, `ReplyEncodeError::Backend`,
`ReplyDecodeError::Backend`), so an exhaustive match writes `Backend(never)
=> match never {}` arms for variants that cannot occur. And
`RemoteError::PayloadDepthMismatch` is rendered as a public variant while
every session path lifts it into `Error::PayloadDepthMismatch` before
returning, so a match on it under `Error::Mirror` is dead code.

Evidence:

    src/error.rs
    52	/// The concrete production mirror failure, retaining its detecting side.
    53	pub type MirrorError = mirror::Error<MaterializedError<Infallible>, RemoteError<Infallible>>;

    src/tree/mirror.rs
    43	pub enum Error<C, S> {
    44	    /// The protocol participant supplied in the client position failed.
    45	    #[error("mirror client failed")]
    46	    Client(#[source] C),
    47	    /// The protocol participant supplied in the server position failed.
    48	    #[error("mirror server failed")]
    49	    Server(#[source] S),

    src/peer/gossip.rs
    1419	    // The depth-limit mismatch is a configuration diagnosis, not a
    1420	    // reconciliation failure: surface it as its own top-level variant.
    1421	    // Only the proxy (the server side of every production handshake)
    1422	    // detects it; the materialized participant has no wire.
    1423	    if let tree::mirror::Error::Server(streaming_remote::Error::PayloadDepthMismatch {
    1424	        local,
    1425	        remote,
    1426	    }) = error
    1427	    {
    1428	        return Error::PayloadDepthMismatch { local, remote };
    1429	    }
    1430	    Error::Mirror(error)

Resolution: document at `MirrorError` which variant is the local
participant and which the wire proxy, in caller vocabulary. Then the owner
chooses: (a) keep the alias and add a sentence to
`RemoteError::PayloadDepthMismatch` saying the peer surface lifts it to
`Error::PayloadDepthMismatch`; or (b) present a concrete, non-generic
public `MirrorError` enum (local violation | wire failure) constructed at
`streaming_error`, keeping the generic sum crate-internal, which removes
the `Infallible` arms and the unreachable variant together. Acceptance:
the `MirrorError` docs state the mapping; either the unreachable variant's
doc says where it surfaces, or no publicly matchable variant is uninhabited
or unreachable.

### api-core-5: Derived `Debug`/`Clone`/`PartialEq` on generic wrappers demand bounds their representations never use; `Error<B>` is not `std::error::Error` for a non-`Debug` bookmark
- Where: src/error.rs:61-63 (related: src/snapshot.rs:16-20, src/peer/gossip.rs:93-94, src/peer/gossip.rs:149-150, src/peer/bootstrap.rs:365-366, src/tree.rs:90, src/tree.rs:137-150, src/bookmark.rs:27-30, src/peer.rs:182-184, src/peer/bootstrap.rs:81-84, tests/api_send_bounds.rs:28-38)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (read the derive sites; read thiserror-impl 2.0.18 src/expand.rs:202-206, the version Cargo.lock pins, which inserts `Self: Debug` and `Self: Display` into the generated `impl Error`'s where-clause whenever the type has type parameters; derive-added per-parameter bounds are language-defined; the compile-time assertions below were not compiled)
- Seen by: structure, correctness, perfapi; refutation: confirmed; history: no rationale found (the derives predate the crate's own stated principle at peer.rs:182-183 and bootstrap.rs:81-84)
- Owner-gated: no (every change loosens a bound; the alternative of a `Debug` supertrait on `BookmarkError` would be gated)
- Recommendation: adopt, as one change with api-audit-1 and tree-core-2 (the same defect filed from the sweep and from the tree partition): the construction fails to compile today for a legal bookmark, and the crate's own comment at src/peer/bootstrap.rs:81-84 states the rule to apply.

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

### api-audit-1: Derived trait impls on wrapper types demand bounds the wrapped values never need
- Where: src/error.rs:61-63 (related: src/snapshot.rs:16-17, src/tree.rs:90-91, src/tree.rs:137-150, src/peer/gossip.rs:93-94, src/peer/gossip.rs:149-150, src/peer/bootstrap.rs:365-366, src/peer.rs:182-193, src/rumors.rs:79-90, src/peer/bootstrap.rs:81-94, src/bookmark.rs:92)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (impl headers extracted from the rendered pages `error/enum.Error.html`, `struct.Snapshot.html`, `enum.Retire.html`, `enum.Joined.html`, `struct.Unbookmarked.html`, `struct.Peer.html` under `target/doc/rumors/`)
- Verification: confirmed; history: no-rationale-found (the derives date from WIP commits; the crate's own `Bootstrap` shows the manual-impl pattern with its rationale)
- Owner-gated: yes: relaxing impl bounds widens the public API, which the owner lands
- Recommendation: adopt, as one change with api-core-5 and tree-core-2; this entry's rendered impl headers are the acceptance oracle for all three.

`Error<B>`, `Retire<T, B>`, `Joined<T, B>`, `Unbookmarked<T, B>`, and
`Snapshot<T>` derive their impls, so the derive's implicit bounds leak into
the API: `rumors::Error<B>` is `Debug`, and therefore `std::error::Error`,
only when the user's bookmark type is `Debug`, a bound `Bookmark` neither
states nor needs (no `B` value is ever held, only `B::Error`); `Snapshot<T>`
is `Clone` only when `T: Clone`, although it is an `Arc`-sharing view whose
`Tree<T>` has an unbounded manual `Clone`. `Peer` and `Rumors` implement
`Debug` by hand for all `T`, and `Bootstrap` carries a comment explaining
exactly this pattern, so the outcome types undo a standard the crate set.

Evidence:

    src/error.rs
    61	#[non_exhaustive]
    62	#[derive(Debug, thiserror::Error)]
    63	pub enum Error<B: BookmarkError = NoBookmark> {

    src/snapshot.rs
    16	#[derive(Clone, Debug, PartialEq, Eq)]
    17	pub struct Snapshot<T> {

    src/tree.rs
    90	#[derive(Debug, Eq)]
    91	pub struct Tree<T> {
    ...
    137	impl<T> Clone for Tree<T> {

    src/peer/bootstrap.rs (the crate already states the rule)
    81	// A manual, unbounded impl: the payload type is phantom (the builder
    82	// holds configuration only), so the `T: Clone` bound `derive` would add
    83	// has nothing to constrain.
    84	impl<T> Clone for Bootstrap<T> {

    rendered impl headers (target/doc/rumors)
    impl<B: Debug + BookmarkError> Debug for Error<B> where B::Error: Debug
    impl<B: BookmarkError> Error for Error<B> where BookmarkIo<B::Error>: Error, Self: Debug + Display
    impl<T: Clone> Clone for Snapshot<T>
    impl<T: Debug> Debug for Snapshot<T>
    impl<T: Debug, B: Debug + BookmarkError> Debug for Retire<T, B>
    impl<T: Debug, B: Debug + BookmarkError> Debug for Joined<T, B>
    impl<T, B: BookmarkError> Debug for Peer<T, B>

Resolution: replace the derives with manual impls mirroring the wrapped
types' bounds: `impl<B: BookmarkError> Debug for Error<B> where
BookmarkIo<B::Error>: Debug` (thiserror's `Error` impl then holds for every
bookmark), the same for `Retire`, `Joined`, `Unbookmarked`; `impl<T> Clone
for Snapshot<T>` and `impl<T> Debug for Snapshot<T>` delegating to `Tree`,
which needs its own manual `Debug` (its derive also binds `T: Debug`
through the `PhantomData`). Add a compile test beside
`tests/api_send_bounds.rs` instantiating `Error<B>` as `Box<dyn
std::error::Error>` for a `Bookmark` whose type is not `Debug`, and cloning
a `Snapshot<P>` for a non-`Clone` payload. Acceptance: the rendered headers
read `impl<T, B: BookmarkError> Debug for Retire<T, B>`, `impl<B:
BookmarkError> Error for Error<B>` without `B: Debug`, and `impl<T> Clone
for Snapshot<T>`; the compile test is committed.

Construction: `struct NoDebugBookmark; impl BookmarkError for
NoDebugBookmark { type Error = std::convert::Infallible; } impl Bookmark
for NoDebugBookmark { /* as NoBookmark */ }` then `fn is_error<E:
std::error::Error>() {} is_error::<rumors::Error<NoDebugBookmark>>();`
fails to compile today. `#[derive(PartialEq, Eq, Serialize, Deserialize)]
struct P(u64); fn is_clone<T: Clone>() {}
is_clone::<rumors::Snapshot<P>>();` fails to compile today.

### api-audit-3: Payload bounds are re-demanded at every method although the crate docs promise 'demanded once'
- Where: src/lib.rs:240-242 (related: src/peer.rs:145, src/peer.rs:195, src/peer.rs:290-292, src/peer/bootstrap.rs:247-252, src/rumors.rs:27, src/rumors.rs:165-168, src/rumors.rs:208-210, src/rumors.rs:244-247, src/rumors.rs:279-282, src/rumors.rs:337-340, src/rumors.rs:362-365, src/rumors.rs:382-385, src/rumors.rs:489-494, src/rumors.rs:617-623, src/snapshot.rs:17, src/snapshot.rs:90-93, src/snapshot.rs:105-110, src/snapshot.rs:153-158, src/rumors/unordered.rs:167, src/rumors/unordered.rs:191, src/rumors/causal.rs:128, src/rumors/causal.rs:151, src/rumors/changes.rs:112-114, src/rumors/changes.rs:127)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read every `impl` block on `Peer`, `Rumors`, `Snapshot`, and the observers; the structs carry no `T` bound, and the only constructors, `Peer::seed` and the two `join`s, each require all six bounds)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes for the structural fix (struct bounds are API); the one-sentence doc correction is not
- Recommendation: adopt the structural fix: `T: Send + Sync + 'static` on the definitions of `Peer`, `Rumors`, `Snapshot`, and the observers makes src/lib.rs:240-242 true as written and deletes every per-method clause; no `Peer<T>` exists today for a `T` that fails them (open question 12).

The crate docs say the payload type's six bounds are demanded once at
construction. `Serialize + DeserializeOwned + Eq` are; `Send + Sync +
'static` are repeated as `where` clauses on `Peer::retire`, every `Rumors`
mutator and session entry, `Snapshot::{get, iter, range}`, and the observer
impls, because `Peer<T, B>`, `Rumors<T, B>`, and `Snapshot<T>` declare no
bound on `T`. The set varies by method (`send` wants `'static`, `redact`
does not), which a reader takes for a per-method requirement, and is not: no `Peer<T>`
exists for a `T` failing any of them.

Evidence:

    src/lib.rs
    240	//! Your message type `T` needs [`serde::Serialize`],
    241	//! [`serde::de::DeserializeOwned`], [`Eq`], [`Send`], [`Sync`], and
    242	//! `'static`, all demanded once, at peer construction. Payloads are

    src/peer.rs
    145	pub struct Peer<T, B: BookmarkError = NoBookmark> {
    ...
    290	    pub async fn retire<CR, CW, C, A>(self, link: &mut Link<CR, CW, C, A>) -> Retire<T, B>
    291	    where
    292	        T: Send + Sync + 'static,

    src/rumors.rs
    208	    pub fn redact(&self, version: &Version)
    209	    where
    210	        T: Send + Sync,

Resolution: minimal: reword lib.rs:240-242 so the serde and `Eq`
obligations are the ones demanded once, while `Send + Sync + 'static`
accompany the methods that move or share the payload. Structural: put `T:
Send + Sync + 'static` on the definitions of `Peer`, `Rumors`, `Snapshot`,
and the observers (`Batch` already carries `Send + Sync`) and delete the
per-method clauses, making the sentence true as written. Acceptance: either
the sentence matches the signatures, or `grep -n 'T: Send + Sync'
src/rumors.rs src/peer.rs src/snapshot.rs` returns only struct-level
bounds.

### deps-6: `pub use ::before;` puts a whole crate on the public surface beside an explicit re-export list, with the choice unrecorded at the site
- Where: src/lib.rs:328-330 (related: src/lib.rs:93, src/bookmark.rs:149, src/rumors.rs:422, src/peer.rs:728)
- Class / severity / confidence: api-surprise / low / medium
- Provenance: assessed (read lib.rs in full; grep of `pub fn|pub trait|pub struct|pub enum|pub type` mentioning `Clock|Party|Dominance|Ticks|Version` over non-test src with `pub(crate)`/`pub(super)` excluded; grep of `rumors::before` over tests/, benches/, examples/, crates/rumors-tracing/src (empty; the tests import `before::...` from the dev-dependency directly))
- Verification: reframed (severity lowered from medium; the rustdoc-rendering claim dropped, see Dropped); history: deliberate-and-holds with a thin record (8dc0596ed, 2026-06-11, "Also re-export `before` (and Version/causally from it) at the crate root"; the same commit added `pub use ::borsh;`, since removed with borsh, so the whole-crate pattern has already contracted once)
- Owner-gated: yes: public API shape
- Recommendation: consider; the recommendation is to keep the whole-crate re-export as the version-pinning path for `before` types in rumors' signatures, record that in a one-line comment at src/lib.rs:328, and keep the named re-exports at :330 as the convenience spellings, saying so (open question 13).

Line 328 re-exports all of `before` as `rumors::before`; line 330 re-exports three of its items by name. The whole-crate form makes every public path of `before` part of rumors' API, so a `before` breaking change is a rumors breaking change, and users have two spellings for the named items. The standard reason for such a re-export holds here: rumors' ungated public signatures name `Version` (Rumors, Batch, Snapshot, the checkpoints), `Ticks`, `causally`, and `Clock` (`BookmarkIo` returns `BTreeMap<Network, Vec<Clock>>`), and a user implementing `BookmarkIo` needs the exact `before` version rumors compiled against; `Party` reaches the ungated surface nowhere (both `dangerously_alias_party` exits are test-internals-gated). Nothing at the site says which consideration won, and the explicit list on the next line suggests the enumerated form was intended.

Evidence:

    src/lib.rs
       328	pub use ::before;
       329	pub use batch::Batch;
       330	pub use before::{Ticks, Version, causally};

    src/bookmark.rs
       149	    ) -> impl Future<Output = Result<BTreeMap<Network, Vec<Clock>>, BookmarkIo<Self::Error>>> + Send;

Resolution: Owner decision between (1) keeping the whole-crate re-export as the version-pinning path for `before` types in rumors' signatures, recorded in a one-line comment at line 328, and then either dropping the duplicate spellings at line 330 or saying why both exist; or (2) replacing line 328 with an enumerated `pub mod before { pub use ::before::{Clock, Ticks, Version, causally, ...}; }` covering the items reachable from rumors' ungated signatures and their methods' return types, with the crate docs' [`before`] link (lib.rs:93) retargeted. Acceptance: line 328 carries either a comment stating the choice or an enumerated module; the named re-exports at 330 are either unique spellings or explained.

### deps-5: rand's RngCore reaches the public surface through an ungated, doc-hidden constructor that only tests call
- Where: src/peer.rs:210-213 (related: src/peer.rs:8, src/peer.rs:207, src/peer.rs:480-481, src/peer.rs:726-727, src/rumors.rs:420-421, src/network.rs:62, design/rumors-frame-fuzz.md:78-79, eleven `tests/*.rs` call sites)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep of `seed_rng` over src/, tests/, benches/, examples/, crates/rumors-tracing, fuzz workspaces; grep of `pub use rand` over src/ (empty); read peer.rs:195-225, 475-490, 720-735, rumors.rs:415-425, network.rs:50-80)
- Verification: confirmed; history: deliberate-and-holds for the constructor itself (0f4461932 introduced it), no-rationale-found for its gating differing from its siblings
- Owner-gated: yes: any change to a `pub` item, hidden or not, is a public API decision
- Recommendation: consider, ruled with tests-lifecycle-10 (open question 6); gating keeps rand's `RngCore` off the shipped surface, and a `Network`-taking constructor is the dependency-free alternative if users' test suites need deterministic seeding.

`Peer::seed_rng<R: RngCore + ?Sized>` is `pub` and `#[doc(hidden)]`, takes rand 0.8's `RngCore` (rand is not re-exported, so a caller must depend on the same rand major), and outside `seed()` is called only from eleven test files. Every other test hook on `Peer` and `Rumors` is gated `#[cfg(any(test, feature = "test-internals"))]`; this one is not. The planned fuzz harness (design/rumors-frame-fuzz.md:78-79) pairs `Peer::seed_rng` with `sync_window_floor()`, which is gated, so that consumer must enable `test-internals` regardless and loses nothing if `seed_rng` is gated the same way.

Evidence:

    src/peer.rs
       210	    /// Like [`seed`](Self::seed), but draws the universe's [`Network`]
       211	    /// identifier from a caller-supplied RNG instead of [`OsRng`].
       212	    #[doc(hidden)]
       213	    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
       480	    #[cfg(any(test, feature = "test-internals"))]
       481	    #[doc(hidden)]
       726	    #[cfg(any(test, feature = "test-internals"))]
       727	    #[doc(hidden)]

    tests/retire_snapshot.rs
        41	    Peer::seed_rng(&mut SmallRng::seed_from_u64(0))

Resolution: Either gate it like its siblings (`#[cfg(any(test, feature = "test-internals"))]`, keeping or dropping `#[doc(hidden)]` to match them), which keeps rand off the shipped surface; or make it a documented constructor over a dependency-free input (`seed_with_network_bytes([u8; 16])`, rejecting the all-zero sentinel as `Network::from_rng` does). The test call sites adapt either way. Acceptance: either `seed_rng` compiles only under test/test-internals, or the seeded constructor's signature names no rand type.

### api-audit-15: `#[doc(hidden)]` bench hooks are not feature-gated like their siblings
- Where: src/peer.rs:713-716 (related: src/peer.rs:210-213, src/rumors.rs:413-416, src/snapshot.rs:166-169, src/peer.rs:480-483, src/peer.rs:726-728, Cargo.toml:144-145, justfile:129-131)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -rln 'warm_caches\|seed_rng' benches examples tests src crates` lists only `benches/*.rs`, `tests/*.rs`, and the definitions; `seed_rng`'s callers are ten `tests/*.rs` files; Cargo.toml:145 gives the self-referential dev-dependency `features = ["test-internals", "conformance"]`, and the justfile comment at 129-131 confirms the dev-dependency cycle enables those features on the lib for every test build; benches build with dev-dependencies too)
- Verification: confirmed; history: no-rationale-found (`warm_caches` dates from cb69fc951, before `test-internals` existed as a gate)
- Owner-gated: yes: removes items from default builds
- Recommendation: adopt, together with benches-envelope-6, tests-lifecycle-10, and deps-5 (open question 6): the four items' only callers already build with `test-internals`.

`Peer::warm_caches`, `Rumors::warm_caches`, `Snapshot::warm_caches`, and
`Peer::seed_rng` are `pub` and hidden but compiled into every build, while
the neighbouring test hooks (`sync_window_floor`,
`dangerously_alias_party`) are gated on `cfg(any(test, feature =
"test-internals"))`. The only callers are benches and integration tests,
which already build with the dev-dependency's `test-internals`, so the gate
costs nothing; hidden items are still callable and semver-relevant, and
the crate docs call `test-internals` "this crate's own test scaffolding".

Evidence:

    src/peer.rs
    713	    #[doc(hidden)]
    714	    pub fn warm_caches(&self) {
    715	        self.inner.borrow().tree.warm_caches();
    716	    }

    src/peer.rs (the sibling that is gated)
    480	    #[cfg(any(test, feature = "test-internals"))]
    481	    #[doc(hidden)]
    482	    #[must_use]
    483	    pub fn sync_window_floor(mut self) -> Self {

    Cargo.toml
    145	rumors = { workspace = true, features = ["test-internals", "conformance"] }

Resolution: add `#[cfg(any(test, feature = "test-internals"))]` to the
four items. Acceptance: `cargo doc -p rumors --no-deps` (no features)
exposes neither `warm_caches` nor `seed_rng`; `cargo bench --no-run` and
`cargo nextest run --no-run` still build.

### fresh-eyes-9: Joined::Bailed/Failed return the bookmark but drop the configuration
- Where: src/peer/bootstrap.rs:345-353 (related: src/peer/bootstrap.rs:44-46, src/peer/bootstrap.rs:247-250, src/peer/bootstrap.rs:279-283, src/peer/bootstrap.rs:379-382, src/peer/bootstrap.rs:398-403)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: verified (read: `let Self { config, bookmark } = self;` at 345, `config` consumed by `config.join(link)` at 346 and absent from both retry arms; `Bootstrap::join` takes `self` at 247-250; `Bootstrap<T>` has a hand-written `Clone` (comment at line 82); `BookmarkedBootstrap<T, B>` at 279 derives nothing)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes
- Recommendation: adopt returning the whole `BookmarkedBootstrap<T, B>` in `Bailed` and `Failed`: the builder's doc promises the retry, and pre-release the shape change is cheap (open question 11); the doc sentence is the fallback.

The plain builder's docs promise that after a bail or a failed session "a
clone of the same configuration retries", but `BookmarkedBootstrap::join`
consumes the builder, the type is not `Clone`, and the retry arms hand back
only the bookmark. A user who wants the promised retry must have cloned the
plain `Bootstrap<T>` before attaching the bookmark, which no page shows.

Evidence:

    src/peer/bootstrap.rs
    44	/// The builder is `Clone`: after a mutual-bootstrap bail
    45	/// ([`join`](Self::join)'s `Ok(None)`) or a failed session, a clone of
    46	/// the same configuration retries against another provider as-is.

    345	        let Self { config, bookmark } = self;
    346	        match config.join(link).await {
    347	            Ok(Some(peer)) => match peer.bookmark(bookmark).await {
    348	                Ok(peer) => Joined::Joined { peer },
    349	                Err(unbookmarked) => Joined::Unbookmarked(unbookmarked),
    350	            },
    351	            Ok(None) => Joined::Bailed { bookmark },
    352	            Err(error) => Joined::Failed { error, bookmark },
    353	        }

Resolution: Either return the whole `BookmarkedBootstrap<T, B>` in `Bailed`
and `Failed` (API change), or add one sentence to `Bootstrap::bookmark`
saying to clone the plain builder first when a retry is anticipated.
Acceptance: a retry after `Joined::Bailed` needs no state the outcome did
not return, or the docs say what to keep.

### api-core-30: Small `Copy` enums omit `Hash`, and `Protocol` omits `Ord`
- Where: src/rumors/changes.rs:55-56 (related: src/peer/gossip.rs:186-187, src/peer/gossip.rs:207-208, src/protocol.rs:13, src/network.rs:24, src/message.rs:71)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (read the derive lines)
- Seen by: perfapi; refutation: confirmed; history: no rationale found
- Owner-gated: yes: adds public trait impls
- Recommendation: adopt: four `Hash` derives and `PartialOrd, Ord` on `Protocol` cost nothing and match `Network` and `PayloadDepthLimit`; the `Ord` half has no observable effect while `Protocol` has one variant, so it is the cheaper half to defer if anything is.

`TryTick`, `Led`, and `Gossip` derive `Debug, Clone, Copy, PartialEq, Eq` but not `Hash`; `Protocol` derives neither `Hash` nor `PartialOrd`/`Ord` though a wire version is naturally ordered. `Network` and `PayloadDepthLimit` carry the full set. Keying per-session metrics by `Led` in a `HashMap` is the concrete use.

Evidence:

    55	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    56	pub enum TryTick {

Resolution: Add `Hash` to the four derives and `PartialOrd, Ord` to `Protocol`. Acceptance: `fn key<K: Hash + Eq>() {}` instantiates for all four in tests/api_send_bounds.rs.

### fresh-eyes-4: Snapshot's IntoIterator names an iterator type users cannot name
- Where: src/snapshot.rs:4-7 (related: src/snapshot.rs:105-107, src/snapshot.rs:172-179, src/lib.rs:317, src/lib.rs:347, src/tree.rs:176)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (the sweep's probe `check-1.log:77`: `error[E0603]: module `snapshot` is private`; read `mod snapshot;` at lib.rs:317 and `pub use snapshot::Snapshot;` at lib.rs:347, the only export; `pub struct Iter<'a, T>` at tree.rs:176 inside the private `mod tree`; no `[lints]` table in Cargo.toml and no `unnameable_types` attribute in src/lib.rs, so no committed check catches this class)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes (whether `Snapshot::iter` returns a named type and whether `Iter` joins the crate root are API decisions)
- Recommendation: adopt with tree-core-5 (open question 1); this is the entry that carries the compiler's own evidence (`check-1.log:77`).

`Iter` is re-exported into the private `snapshot` module only, so it is
reachable solely as `<&Snapshot<T> as IntoIterator>::IntoIter`; a user cannot
write the type to hold the iterator in a struct. The doc sentence is also
inaccurate on the visible surface: `Snapshot::iter` returns an `impl` type,
not `Iter`, and "re-exported from the tree internals" names a layer the API
does not expose.

Evidence:

    src/snapshot.rs
    4	/// The iterator of [`Snapshot::iter`], re-exported from the tree internals:
    5	/// every live message as `(&Version, Arc<T>)`, unspecified order,
    6	/// exact-size and double-ended.
    7	pub use crate::tree::Iter;

    105	    pub fn iter(
    106	        &self,
    107	    ) -> impl DoubleEndedIterator<Item = (&Version, Arc<T>)> + ExactSizeIterator + Send + Sync

    172	impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    173	    type Item = (&'a Version, Arc<T>);
    174	    type IntoIter = Iter<'a, T>;

    src/lib.rs
    317	mod snapshot;
    347	pub use snapshot::Snapshot;

Resolution: Owner's choice: (a) re-export `Iter` at the crate root
and let `Snapshot::iter` return it, so the doc sentence becomes true; or (b)
keep the `impl` return and make `IntoIter` a nameable public type. Either
way, add `#![warn(unnameable_types)]` to src/lib.rs so the gate's clippy leg
fails the next unnameable public type: the committed check for this hole.
Rewrite snapshot.rs:4-6 without "tree internals". Acceptance: a doctest or
integration test names the iterator type in a `let x: rumors::... =
snapshot.into_iter();` binding and compiles; `unnameable_types` is enabled
and the gate is clean.

### api-core-35: `Snapshot` equality includes the causal ceiling, undocumented, while `hash` documents excluding it
- Where: src/snapshot.rs:16-20 (related: src/snapshot.rs:69-82, src/tree.rs:131-135, src/tree.rs:146-150, tests/routed_link.rs:250, .agent-notes/2026-07-23-review-link-transport/review-link-transport-branch.md:299-301)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read `Root::eq`: it compares `ceiling` as well as `root`; `Tree::eq` delegates to it; the hash docs at 73-79 say the hash excludes the frontier; the one test use of `==` is tests/routed_link.rs:250 after convergence)
- Seen by: correctness; refutation: confirmed; history: no rationale found for the semantics; link-transport review R13 relies on `Snapshot: Eq` and characterizes it as "set equality", which is not what the code computes, so the review's assumption strengthens the documentation half
- Owner-gated: yes: defines or removes a public trait's semantics, and removal reopens R13
- Recommendation: consider; the recommendation is to document `==` as same network, same live set, and same frontier, because tests/routed_link.rs:250 and the link-transport ruling R13 already lean on the impl and the `hash` doc already teaches the reader the frontier distinction (open question 2).

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

### api-audit-2: Public signatures name types with no public path: Snapshot's IntoIter, Signal, and a second StreamError
- Where: src/snapshot.rs:172-174 (related: src/snapshot.rs:4-7, src/snapshot.rs:105-112, src/tree.rs:167-176, src/lib.rs:347, src/tree/mirror/streaming/remote/codec/signal.rs:40-47, src/tree/mirror/streaming/remote/codec/signal.rs:105-110, src/tree/mirror/streaming/remote/codec/signal.rs:192-200, src/tree/mirror/streaming/remote/codec/signal.rs:374-377, src/tree/mirror/streaming/remote/error.rs:11-18, src/tree/mirror/streaming/remote/streams.rs:264-266)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (`find target/doc/rumors -name '*Iter*'` returns nothing; every `href` in `struct.Snapshot.html` containing `Iter` points at `IntoIterator`/`DoubleEndedIterator`/`ExactSizeIterator` in std, none at an `Iter` page; `error/struct.Stream.html` renders `Result<Self, StreamError>` with `StreamError` as unlinked text; `error/` holds 36 pages and no `Signal` page)
- Verification: confirmed; history: no-rationale-found (the `Iter` re-export dates from ddc57045f, before `snapshot` became a private module)
- Owner-gated: yes: the fix exports a type or removes public methods
- Recommendation: adopt: the `Iter` half resolves with tree-core-5 (open question 1), the error-module half with remote-codec-30 and api-audit-14 (open question 4), and `unnameable_types` is the committed check for both.

`impl IntoIterator for &Snapshot<T>` sets `IntoIter = Iter<'a, T>`, but
`Iter` lives in the private `tree` module and its `pub use` sits in the
private `snapshot` module (`lib.rs` re-exports only `Snapshot`), so no user
can write the type to store the iterator or name it in a signature, the
doc comments written for it render nowhere, and `Snapshot::iter()` returns
an `impl` type, so the two faces of one iterator disagree. The same hole
exists twice in `rumors::error`: `InvalidSignalPlacement::signal()` returns
`Signal`, and `Stream::new()` returns `Result<Self, StreamError>` where
that `StreamError` is `signal::StreamError`, an unexported enum sharing its
name with the exported `rumors::error::StreamError` from `streams.rs`.

Evidence:

    src/snapshot.rs
    4	/// The iterator of [`Snapshot::iter`], re-exported from the tree internals:
    5	/// every live message as `(&Version, Arc<T>)`, unspecified order,
    6	/// exact-size and double-ended.
    7	pub use crate::tree::Iter;
    ...
    172	impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    173	    type Item = (&'a Version, Arc<T>);
    174	    type IntoIter = Iter<'a, T>;

    src/lib.rs
    347	pub use snapshot::Snapshot;

    src/tree/mirror/streaming/remote/codec/signal.rs
    41	    pub fn new(index: u8) -> Result<Self, StreamError> {
    ...
    107	pub enum StreamError {
    ...
    375	    pub fn signal(self) -> Signal {

    src/tree/mirror/streaming/remote/error.rs (neither Signal nor signal::StreamError is exported)
    11	pub use super::codec::{
    12	    DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind, DecodeLeafError,
    13	    DecodeSignalError, EncodeError as CodecEncodeError, EncodeErrorKind as CodecEncodeErrorKind,
    14	    FramePart, GreetingError, HeadError, InvalidSignalPlacement, LeafRunError, ListingIssue,
    15	    Origin, QueryOrderError, Speaker, Stream, StreamClass,
    16	};
    17	pub use super::proxy::Error as RemoteError;
    18	pub use super::streams::{AcceptError, ReplyFrameError, SendError, StreamError};

Resolution: export `Iter` at the crate root (for example `pub use
snapshot::{Snapshot, SnapshotIter}` with the alias at snapshot.rs:7) and
have `Snapshot::iter()` return it by name so both faces agree. In `error`,
make `Stream::new`, `Stream::at_height`, and `Stream::height` `pub(crate)`
(see api-audit-14) so `signal::StreamError` leaves the public surface, or
export it under a distinct name; make `InvalidSignalPlacement::signal`
`pub(crate)` or export `Signal`, `Flow`, and `End`. Then close the class
mechanically: enable rustc's allow-by-default `unnameable_types` lint in
`lib.rs` (`#![warn(unnameable_types)]`, denied by the clippy leg's `-D
warnings`) if the pinned toolchain carries it. Acceptance: every type in a
rendered public signature links to a page (`struct.Snapshot.html`'s `type
IntoIter` links to an `Iter` page; `error/struct.Stream.html` and
`error/struct.InvalidSignalPlacement.html` show no unlinked type), and the
lint is on.

### inventory-1: `tree::Iter` reaches the public API through `IntoIterator` but has no nameable path
- Where: src/snapshot.rs:172-179 (related: src/snapshot.rs:4-7, src/snapshot.rs:105-112, src/lib.rs:317, src/lib.rs:347, src/tree.rs:176)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (grep for `Iter` across lib.rs, snapshot.rs, tree.rs; lib.rs:317 declares `mod snapshot;` private and lib.rs:347 re-exports only `Snapshot`; blame at snapshot.rs:7)
- Verification: confirmed; history: no-rationale-found (snapshot.rs:7 dates from ddc57045, 2026-06-10, and no note or ruling mentions it)
- Owner-gated: yes: the fix either adds a public name or changes `Snapshot::iter`'s return type
- Recommendation: adopt with tree-core-5 (open question 1); this entry adds the blame (ddc57045, 2026-06-10) showing the re-export predates the module going private.

`impl IntoIterator for &Snapshot<T>` names `Iter<'a, T>` as its `IntoIter`, but the only re-export of `Iter` is `pub use crate::tree::Iter` inside the private `snapshot` module, and `Snapshot::iter` returns an opaque `impl DoubleEndedIterator + ExactSizeIterator` rather than the same type. A user can iterate `&snapshot` but can name the iterator only as `<&Snapshot<T> as IntoIterator>::IntoIter`, and the doc at snapshot.rs:4 describes a re-export no reachable path provides.

Evidence:

    4	/// The iterator of [`Snapshot::iter`], re-exported from the tree internals:
    5	/// every live message as `(&Version, Arc<T>)`, unspecified order,
    6	/// exact-size and double-ended.
    7	pub use crate::tree::Iter;

    105	    pub fn iter(
    106	        &self,
    107	    ) -> impl DoubleEndedIterator<Item = (&Version, Arc<T>)> + ExactSizeIterator + Send + Sync

    172	impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    173	    type Item = (&'a Version, Arc<T>);
    174	    type IntoIter = Iter<'a, T>;

    317	mod snapshot;
    ...
    347	pub use snapshot::Snapshot;

Resolution: Decide which face is the contract. Either re-export `Iter` from the crate root and have `Snapshot::iter` return `Iter<'_, T>` so both faces agree and the doc at snapshot.rs:4-7 names the reachable path, or keep the opaque `iter` and make the `IntoIterator` impl's `IntoIter` a type users can reach. rustc's allow-by-default `unnameable_types` lint mechanizes this class; enabling it in lib.rs catches recurrences. Acceptance: `rumors::Iter` (or another reachable path) compiles from an external crate and is the type `Snapshot::iter` returns, or `unnameable_types` is enabled and clean.

### deps-12: rumors' `[package]` alone in the workspace lacks `description` and `license`
- Where: Cargo.toml:83-87 (related: crates/before/Cargo.toml:5-6, crates/rumors-tracing/Cargo.toml:5-6, crates/suanpan/Cargo.toml:5-6, crates/surface-scan/Cargo.toml:5-6, crates/before-viz/Cargo.toml:5-6)
- Class / severity / confidence: feature-gap / nit / high
- Provenance: verified (read every member `[package]` table)
- Verification: confirmed and sharpened; history: no-rationale-found
- Owner-gated: yes: the description is published prose in Finch's name
- Recommendation: adopt: `license = "MPL-2.0"` is the workspace convention; the one-sentence description is Finch's prose to write (open question 19).

Every sibling crate carries `description` and `license = "MPL-2.0"` (surface-scan and before-viz additionally `publish = false`); rumors, the crate the workspace exists to ship, carries neither, and `cargo publish` refuses a crate without `license` and `description`. The license is therefore not an open decision but a workspace convention rumors has not joined.

Evidence:

    Cargo.toml
        83	[package]
        84	name = "rumors"
        85	version = "0.1.0"
        86	edition = "2024"
        87	readme = "README.md"

    crates/before/Cargo.toml
         5	description = "Interval Tree Clocks (Almeida, Baquero & Fonte, 2008): packed bit-stream storage, transient fixed-width working form, linear-typed API."
         6	license = "MPL-2.0"

Resolution: Add `license = "MPL-2.0"` and a one-sentence `description` (the crate doc's first sentence is the natural source); add `repository` when the crate's public home is settled. Acceptance: `cargo publish --dry-run -p rumors` passes the manifest checks.

### api-audit-11: Session settings cannot be read back
- Where: src/peer.rs:467-467 (related: src/peer.rs:549, src/peer.rs:611, src/peer.rs:182-193, src/rumors.rs:79-90, src/peer/bootstrap.rs:96-105)
- Class / severity / confidence: feature-gap / nit / medium
- Provenance: verified (`grep -n 'pub fn' src/peer.rs src/rumors.rs` shows no getter for any setting; `Bootstrap`'s manual `Debug` prints all three, `Peer`'s and `Rumors`' print none)
- Verification: reframed: the sweep also flagged the argument shapes (`usize` for the two byte budgets, a newtype for the depth limit) as inconsistent; the asymmetry has a reason the code shows: `PayloadDepthLimit` crosses the wire, is compared for equality in the handshake, and appears in `Error::PayloadDepthMismatch`, while the byte budgets are local-only and never leave the process, so bytes-as-`usize` is the std idiom there. The getter gap stands; history: no-rationale-found
- Owner-gated: yes: adds public methods
- Recommendation: adopt the `Debug` fields now (the builder already prints them); add getters, or one settings view, when a caller needs to assert them (open question 9).

No getter exists for `sync_memory_budget`, `target_message_size`, or
`payload_depth_limit` on `Peer`, `Rumors`, or the builders, and the `Peer`
and `Rumors` `Debug` impls omit them, so a configured peer's effective
settings cannot be logged or asserted without the caller carrying the
values it passed.

Evidence:

    src/peer.rs
    467	    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
    ...
    549	    pub fn target_message_size(mut self, bytes: usize) -> Self {
    ...
    611	    pub fn payload_depth_limit(mut self, limit: PayloadDepthLimit) -> Self {

    src/peer/bootstrap.rs (the builder already prints what the peer cannot)
    99	        f.debug_struct("Bootstrap")
    100	            .field("window", &self.window)
    101	            .field("run_budget", &self.run_budget)
    102	            .field("payload_depth_limit", &self.payload_depth_limit)

Resolution: add the three settings to `Peer`'s and `Rumors`' `Debug`
output at minimum; consider getters (or one settings view) if a caller
needs to assert them. Acceptance: a test reads each setting back from a
configured `Rumors`, or the `Debug` output shows them.

## Session and bookmark (peer/gossip, bookmark, reconciliation, observe, message)

### api-audit-4: `BookmarkError` is a misnamed supertrait implemented by bookmarks, not by errors
- Where: src/bookmark.rs:26-31 (related: src/bookmark.rs:48-49, src/bookmark.rs:92, src/bookmark.rs:144, src/bookmark.rs:199-201, src/peer.rs:145, src/rumors.rs:27, src/error.rs:63, src/peer/gossip.rs:94, src/peer/gossip.rs:150, src/peer/bootstrap.rs:366)
- Class / severity / confidence: api-surprise / low / medium
- Provenance: assessed (read bookmark.rs in full; every constructor path that yields a `Peer<T, B>` requires `B: Bookmark`: peer.rs:272-277, bootstrap.rs:212, and `seed` fixes `NoBookmark`)
- Verification: confirmed; history: no-rationale-found (`git log -S'pub trait BookmarkError'` attributes it to cbfc1571e, "Unify async/sync interfaces, parameterizing by defaulted type", whose message records no reason for the split)
- Owner-gated: yes: renames or removes a public trait
- Recommendation: adopt: fold `type Error` into `Bookmark` and delete `BookmarkError`; no rationale for the split exists in git or the notes, and the fresh-eyes application implemented two traits to supply one associated type (open question 3).

The trait named `BookmarkError` is implemented by bookmark types and
carries only `type Error`; its first sentence describes the associated
type, not the trait. Every public generic reads `B: BookmarkError`, telling
a reader that `B` is an error type. Since every constructor requires `B:
Bookmark`, the split adds nothing a `type Error` on `Bookmark` would not;
the crate-private `Persist` supertrait would carry its own `type Error`,
normalized to `Bookmark::Error` by the blanket impl.

Evidence:

    src/bookmark.rs
    26	/// The error a [`Bookmark`] reports when persistence fails.
    27	pub trait BookmarkError {
    28	    /// What a [`load`](Bookmark::load) or [`store`](Bookmark::store)
    29	    /// reports when it fails.
    30	    type Error: std::error::Error + Send + Sync + 'static;
    31	}
    ...
    92	pub trait Bookmark: BookmarkError {
    ...
    199	impl BookmarkError for NoBookmark {
    200	    type Error = std::convert::Infallible;
    201	}

    src/peer.rs
    145	pub struct Peer<T, B: BookmarkError = NoBookmark> {

Resolution: fold `type Error` into `Bookmark`, bound the structs and
`Error<B>` on `B: Bookmark`, and delete `BookmarkError`, so a user
implementing a bookmark implements one trait. If a reason for the split
exists, keep the trait under a name that describes it and rewrite its
first sentence to describe the trait rather than the associated type.
Acceptance: no public bound reads `B: BookmarkError`, or the trait's name
and first sentence describe what implements it.

### session-bookmark-23: `Bookmark::store` lends a writer through an HRTB closure and a boxed future, but the crate hands it an already-materialized `Vec`
- Where: src/bookmark.rs:121-123 (related: src/bookmark.rs:33-39, 180-189, 210-217; src/lib.rs:333; tests/common/flaky.rs:206-220; tests/bookmark_when.rs:111-124; tests/bookmark_transmit_window.rs:115-128)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (read every `impl Bookmark for` in the tree: `NoBookmark` plus three test implementors, each of which copies the lent bytes into a fresh `Vec`; `Persist::write` encodes to `bytes` first and the closure body is `w.write_all(&bytes)`; `git show 80898f19` records the introducing rationale; `83edcd94` deleted `src/sync.rs`, the blocking twin)
- Seen by: perfapi; refutation: confirmed (with a correction to the proposed signature: a `&[u8]` borrowed from a temporary cannot ride the returned future in the combinator shape the comment at 158-160 protects; the parameter should be owned); history: deliberate but expired (the closure shape was chosen when two faces, async and blocking, shared one engine and was justified as bracketing the serialize step for atomicity; the blocking face is gone, and even at introduction the async `write` pre-encoded the bytes, so the bracket never enclosed serialization)
- Owner-gated: yes: public trait signature
- Recommendation: adopt: an owned `Vec<u8>` parameter carries the identical atomicity obligation, every implementor in the tree (and the fresh-eyes application) already materializes the bytes, and `Serialized` goes with it; decide `load`'s shape once alongside (open question 3).

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

### session-bookmark-44: `pub struct Message` and eight `pub fn`s are unreachable outside the crate but documented in library-user voice; `try_new`'s doc misnames it as the send-path constructor
- Where: src/message.rs:47-51 (related: src/message.rs:320-364, 399-420, 465-528; src/lib.rs:310, 337; src/peer.rs:14; src/tree.rs:714-715; src/conformance.rs:18-19)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep: `mod message;` is private at lib.rs:310 and only `EncodeError` (lib.rs:337) plus `PayloadDepthLimit`/`DEFAULT_PAYLOAD_DEPTH_LIMIT` (peer.rs:14) are re-exported; `src/testing.rs` names no `Message`; `mod tree;` is private (lib.rs:322); every non-`tests.rs` caller of `Message::new` is under a `#[cfg(test)]` module (`tree::arb`, `conformance::backend`, `streaming::testing`, and the streaming `tests/` directories gated at streaming.rs:219, proxy.rs:49, codec.rs:242, adapter.rs:86); `try_new`, `from_slice`, `from_bytes`, `from_arc` have no callers outside test code; the production send path is `PayloadCodec::new` -> `serialize_payload` -> `Message::try_from_arc` at 213-221; no `unreachable_pub` lint is configured)
- Seen by: structure, perfapi; refutation: confirmed (both); history: deliberate but expired (`Message` was public until 94da12b5 purged the exports to restart the surface; 6e4b6eea noticed the module is private and chose documentation over visibility; no owner ruling on visibility is recorded)
- Owner-gated: yes: whether `Message` becomes public API or `pub(crate)`
- Recommendation: adopt `pub(crate)` with the docs recast at maintainer altitude and the `try_new` correction; `#![warn(unreachable_pub)]` is the committed check and a crate-wide decision (open question 14).

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

### session-bookmark-46: `EncodeError` is not `#[non_exhaustive]` while every other public error taxonomy in the partition is
- Where: src/message.rs:116-117 (related: src/bookmark/format.rs:87-89, 127-129, 185-187; src/error.rs:61-63)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep: `FrameDefect`, `RecordDefect`, `FormatError`, and `Error` carry `#[non_exhaustive]`; `EncodeError` does not; every downstream match in tests/payload_depth.rs and tests/single_peer.rs uses `matches!`, and the rumors.rs doctest does not match on it, so adding the attribute breaks nothing in the tree)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (the crate's criterion for `non_exhaustive`, 1e458d69, predates `EncodeError` by a day and the enum landed without a recorded classification under it)
- Owner-gated: yes: changes downstream matching (cheap pre-release)
- Recommendation: adopt: `EncodeError` is the admission taxonomy a caller matches on and grows with enforcement, the class 1e458d69 marks open, and no in-tree match breaks (open question 5).

`EncodeError` is the admission-failure taxonomy a caller matches on; an added admission rule (a payload size ceiling is a plausible one) would be a breaking change. Closed outcome sets in the partition (`Retire`, `Led`, `Gossip`, `Direction`, `Role`, `BookmarkIo`) reasonably stay exhaustive; `EncodeError` is a taxonomy that grows as enforcement grows, the class the crate's own criterion marks `non_exhaustive`.

Evidence:

    116	#[derive(Debug, thiserror::Error)]
    117	pub enum EncodeError {

    87	#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
    88	#[non_exhaustive]
    89	pub enum FrameDefect {

Resolution: Add `#[non_exhaustive]` to `EncodeError`. Acceptance: the attribute is present; the doctest at rumors.rs and the `matches!` tests still compile.

### session-bookmark-40: A second `Peer::observe` or `Bootstrap::observe` replaces the first, and neither doc says so
- Where: src/observe.rs:226-230 (related: src/peer.rs:486-510, src/peer/bootstrap.rs:172-183, src/peer/bootstrap.rs:321-324)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read `attach`; grep of the `Peer::observe` and `Bootstrap::observe` docs for "replace", "again", "twice", "last" finds nothing)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (the design record describes a single handler attached at construction and never addresses repeated attach)
- Owner-gated: no for the doc sentence; yes for the accumulate-and-fan-out alternative
- Recommendation: adopt the sentence on both builders; accumulate-and-fan-out is a design with no consumer yet.

`Attachment::attach` overwrites `self.handler`. The public builders document what an observer sees and how it follows the peer, but not that attaching twice keeps only the last. A user who attaches `rumors_tracing::TracingObserver` and then a session recorder gets only the recorder, with no error and no doc to consult; composing by hand requires fan-out at all three levels.

Evidence:

    226	impl Attachment {
    227	    /// Attach `observer`; later sessions ask it for session handlers.
    228	    pub(crate) fn attach(&mut self, observer: Arc<dyn Observer>) {
    229	        self.handler = Some(observer);
    230	    }

Resolution: One sentence on both builders: "Attaching again replaces the earlier handler; to feed several consumers, attach one observer that fans out." Owner-gated alternative: accumulate into a `Vec<Arc<dyn Observer>>` and fan sessions out in `begin`. Acceptance: the sentence is present on both builders, or a test in tests/observe.rs attaches two observers and asserts both see the session.

### session-bookmark-20: The bookmark format's decoder is crate-private, so an operator holding a bookmark file has no read-only way to inspect it
- Where: src/bookmark.rs:24 (related: src/bookmark/format.rs:423-426, src/bookmark.rs:77-82, src/peer/gossip.rs:153-156)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: verified (grep: `pub(crate) fn decode` at format.rs:423; `BOOKMARK_FORMAT_VERSION`, `FormatError`, `FrameDefect`, `RecordDefect` re-exported at bookmark.rs:24 and lib.rs:331-334)
- Seen by: perfapi; refutation: reframed (the error taxonomy does have a public producer, a live peer's session or attach failure; the remaining gap is an offline reader); history: no rationale found
- Owner-gated: yes: new public API
- Recommendation: consider; the format is owned, versioned, and never migrated, so a read-only `inspect` is the natural operator tool, but nothing needs it yet: add it when an operator tool first does (open question 9).

The crate owns, versions, and refuses to migrate the persistence format, and the trait doc (77-82) says a mis-filed bookmark from another universe is silent ("the foreign identities simply lie dormant"). An operator deciding whether a file is safe to delete, or is the right one to attach, has no way to ask the crate which networks and how many stranded identities it holds; the only decoder runs inside a live peer.

Evidence:

    24	pub use format::{BOOKMARK_FORMAT_VERSION, FormatError, FrameDefect, RecordDefect};

    423	pub(crate) fn decode(bytes: &[u8]) -> Result<BTreeMap<Network, Vec<Clock>>, FormatError> {

Resolution: Owner decision. Expose a read-only `inspect` over frame bytes returning the record or a summary (`Network` to count and per-clock own version), documented as diagnostic only. Acceptance: a doctest decodes the bytes a `Bookmark` implementor stored and lists networks; the format pins are unaffected.

### session-bookmark-38: `SessionObserver` has no end-of-session hook carrying the outcome; `Drop` is the only end signal and it cannot say whether the session converged or failed
- Where: src/observe.rs:82-105 (related: src/observe.rs:39-44, 71-74; src/peer/gossip.rs:897-904; crates/rumors-tracing/src/lib.rs:175-182)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: verified (read `SessionObserver` at 82-105: methods `elected` and `stream` only; `rumors-tracing`'s `SessionAdapter` holds a `Span` and grep finds no `impl Drop` in that crate, so it cannot record span status or the converged version)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (the design record specifies three levels plus `elected` and says nothing about a session-end signal)
- Owner-gated: yes: public trait addition
- Recommendation: adopt a defaulted `SessionObserver::finished(outcome)` fired once per session on every exit path: `rumors-tracing` needs it to set span status, and the existing `elected` default-method pattern is the template (open question 9).

The module doc names tracing adapters and session recorders as the hook's consumers and admits an aborted session "may have observed fewer items than crossed the wire" (43-44), yet a consumer learns the end only by implementing `Drop`, which carries no outcome. The shipped adapter cannot set span status (converged version, `Error::Epilogue`, a protocol violation), and a recorder cannot tag a capture as complete versus aborted.

Evidence:

    82	pub trait SessionObserver: Send + Sync {

    92	    fn elected(&self, role: Role) {

    104	    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>>;

Resolution: Owner decision. Add a defaulted `fn finished(&self, outcome: ...)` to `SessionObserver`, invoked from `gossip_inner` and `bootstrap_erased` on every exit path via `SessionHandle`, with a rumors-blind payload (an outcome enum plus the public `Gossiped` or an error kind), following the existing `elected` default-method pattern. Acceptance: `rumors-tracing` sets span status from the hook; `tests/observe.rs` asserts the hook fires exactly once per session on both `Ok` and an injected failure.

## Link

### link-6: `SessionState` has public getters but no path to read them without consuming the link
- Where: src/link.rs:363-376 (related: src/link.rs:328, src/link.rs:470-478, src/link.rs:491-499)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -n 'fn session' src/link.rs` is empty; `grep -rn '\.session\b'` outside src/link shows every public-tier read going through `parts.session` after `into_parts`, e.g. proxy/work/tests.rs:48, conformance/link.rs:1004, testing/transport.rs:533)
- Seen by: perfapi (35); refutation: confirmed; history: no rationale (ee67e4cb added the getters when sealing the fields; a `Link::session()` accessor was never raised)
- Owner-gated: yes: public API addition
- Recommendation: adopt, unless link-9 (simplification) dissolves `LinkParts` and publishes `Link`'s fields, which would make the getter unnecessary; a public type with public getters needs a public way to borrow it either way.

`epoch()` and `poisoned()` are `pub`, but `Link::session` is `pub(crate)` and the only public route to a `SessionState` is `into_parts`, which consumes the link. A connection manager that wants check-then-act (re-link instead of firing a session that fails with `Error::LinkPoisoned`) must dismantle and reassemble the link to ask. A public getter on a public type implies a public way to hold a reference to that type. link-9, if adopted, dissolves this.

Evidence:

    365	    pub fn epoch(&self) -> u8 {
    366	        self.epoch
    367	    }

    374	    pub fn poisoned(&self) -> bool {
    375	        self.poisoned
    376	    }

    328	    pub(crate) session: SessionState,

Resolution: add `pub fn session(&self) -> &SessionState` on `Link` in the unbounded impl block beside `into_parts`. No wire or behavior change. Acceptance: a unit test reads `link.session().poisoned()` on a fresh link (false) and after an interrupted session (true) without calling `into_parts`.

### link-16: Router evictions are unobservable except through the affected link's next error
- Where: src/link/routed/endpoint.rs:52-55 (related: src/link/routed/router.rs:159-163, src/link/routed/router.rs:218-223, src/link/routed/router.rs:239-242, src/link/routed/router.rs:252-257)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: assessed (read every eviction and drop site in router.rs; none counts, logs, or calls out)
- Seen by: perfapi (45); refutation: confirmed; history: no rationale (no note considers router observability; `src/observe.rs` has no router hook)
- Owner-gated: yes: new public surface
- Recommendation: consider; once link-28 (correctness) lands, the admitted-connection eviction class disappears and the remaining evictions are conformance-bug signals worth counting, but a counter no known-bad scenario moves is decoration, so add only the counters `pending_header_bound_evicts_oldest` and `queue_overflow_evicts_the_link` can assert on (open question 9).

The `pending_headers` doc concedes that eviction of an idle recovered connection is silent and advises sizing generously; a stream-queue overflow and a pending-header eviction likewise leave no trace except a later transport error on one link. An operator cannot size `pending_headers` from data, detect a nonconforming peer, or tell a pool-corruption pattern from ordinary churn. "Size generously" is a guess standing in for a measurement. If link-28's structural fix lands, the admitted-connection eviction class disappears and the remaining evictions are conformance-bug signals worth counting.

Evidence:

    52	    /// expects. Eviction of an idle recovered connection is silent: no
    53	    /// invalidation reaches the dialer's pool, and the next stream
    54	    /// drawn on the dead entry fails, or hangs to the caller's session
    55	    /// timeout. Size generously.

Resolution: owner decision on shape. Smallest: a `RouterStats` of atomic counters (headers read, `LINK` accepted, `LINK` rejected, `STREAM` routed, unknown token, queue-overflow evictions, pending-header evictions) shared between the router and `Endpoint::stats(&self)`. Acceptance: `pending_header_bound_evicts_oldest` and `queue_overflow_evicts_the_link` assert the corresponding counter moved from 0 to 1; a counter that no known-bad scenario moves is decoration and is not added.

### link-21: The dialing side of a routed link cannot observe its `Token`
- Where: src/link/routed/endpoint.rs:253-279 (related: src/link/routed/endpoint.rs:124-133, src/link/routed/header.rs:87-98, src/link/routed/stream.rs:24-35, src/link/routed/stream.rs:66-84)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (read endpoint.rs and stream.rs in full: `link` returns `Ok(Link::new(...))` with no token; neither `StreamConnector` nor `StreamAcceptor` has an accessor)
- Seen by: perfapi (36); refutation: confirmed; history: no rationale (the design record fixes that tokens are "observed through LinkInfo" and never discusses the dialer's view)
- Owner-gated: yes: public API change
- Recommendation: adopt the tuple return `(LinkInfo<D::Addr>, RoutedLink<D>)` from `Endpoint::link`: it mirrors `Incoming::accept`, keeps the two ends symmetric, and is the smaller surface (open question 9).

`Endpoint::link` draws the token and returns only the `RoutedLink<D>`; the accepting side receives `LinkInfo { peer, token }`. The token is the one identifier both processes share for a link, so the dialer cannot log it, correlate a poisoned link with the peer's view, or tell which of several links to the same peer failed. The two ends of one object are asymmetric for no stated reason.

Evidence:

    253	    pub async fn link(&self, peer: D::Addr) -> Result<RoutedLink<D>, LinkError> {
    254	        // Register before the header goes out: the peer's reverse
    255	        // dials can only follow its read of the header, so they always
    256	        // find the token routable.
    257	        let (token, registration, streams) = router::register(&self.inner.table);

    131	    /// The link's routing identity, unique per link on this endpoint.
    132	    pub token: Token,

Resolution: return `(LinkInfo<D::Addr>, RoutedLink<D>)` from `Endpoint::link`, mirroring `Incoming::accept` (the `peer` field echoes the argument), or add `pub fn token(&self) -> Token` on `StreamConnector`/`StreamAcceptor`. The tuple return keeps the ends symmetric and is the smaller surface. Acceptance: `establishment_connects_control_and_streams` asserts the dialer-side token equals `info.token` on the accepting side.

## Conformance

### conformance-1: The module doc promises a suite per caller-implementable boundary; `Bookmark` has none
- Where: src/conformance.rs:3-8 (related: src/bookmark.rs:92-124, src/conformance/link.rs:10-26)
- Class / severity / confidence: feature-gap / medium / high
- Provenance: assessed (read; `pub trait Bookmark` and its `store` contract read at src/bookmark.rs:92-124; grep for a `conformance` mention in bookmark.rs or a bookmark submodule under conformance returns nothing)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (a3da46c4 made `conformance` a namespace "leaving room for future suites over other caller-built boundaries"; nothing considers or declines a Bookmark suite)
- Owner-gated: yes: a new public module, or a narrowing of a public doc sentence
- Recommendation: adopt: ship `conformance::bookmark::check(factory)` with the negative control and a paragraph naming what the suite cannot see (crash atomicity); the commit-iff-`Ok` clause is the one a deployment gets wrong over a file or KV store, and the crate names the consequence as unspecified corruption (open question 10).

The public module doc states a universal rule (one validation submodule per caller-implementable boundary) and ships one suite, for `Link`. `Bookmark` is application-supplied storage whose `store` contract carries clauses a black-box suite can check exactly the way the link suite checks its clauses: `load` is `Ok(None)` before any store, store-then-load round-trips the bytes, a second store replaces the first, and a `write` closure that returns `Err` after a partial write leaves the prior record readable rather than a torn frame. The crate classifies a violation as unspecified corruption (identity reclaimed below a transmitted frontier), and a deployment has no way to validate its commit semantics before that can happen. Prose speaks what is: either the rule is met or it is not stated.

Evidence:

         3	//! Each caller-implementable boundary gets one submodule carrying its
         4	//! validation suite. [`link`] checks that a caller-built
         5	//! [`Link`](crate::link::Link) transport delivers the stream independence,
         6	//! flow control, half-close, and cancellation tolerance the [link
         7	//! module](crate::link) requires of every implementation. Available from a
         8	//! dev-dependency with the `conformance` cargo feature enabled.

    src/bookmark.rs:
       106	    /// The crate serializes the framed record by calling `write` with a lent
       107	    /// writer. The implementor **must commit the written bytes atomically iff
       108	    /// `write` returns `Ok`** and must report an error rather than leave a
       109	    /// partial frame where the next [`load`](Self::load) could read it.

Anchor correction: the four quoted `Bookmark::store` doc lines sit at src/bookmark.rs:106-109 at this commit; the quoted numbers here and in the finalized record are corrected from 107-110, and the `Where` anchors are unchanged.

Resolution: Either add `conformance::bookmark::check(factory)` taking a factory of fresh, empty `Bookmark` instances and probing the clauses above, with a negative control (a bookmark that commits a partial frame on `Err`) asserted to fail, and a "what the suite cannot see" paragraph naming crash atomicity; or reword src/conformance.rs:3-4 so it claims only what ships. Acceptance: a `conformance::bookmark` module exists with the checks and control described, or the module doc no longer states one submodule per caller-implementable boundary.

### conformance-6: A hang under `check` names no check
- Where: src/conformance/link.rs:147-157 (related: src/conformance/link.rs:21-26, tests/tcp_link.rs:54-61, tests/routed_link.rs:94-101)
- Class / severity / confidence: feature-gap / low / high
- Provenance: assessed (read; both in-tree consumers wrap the whole suite in one `timeout` whose expiry message is "conformance suite ran past its liveness bound")
- Seen by: perfapi; refutation: confirmed; history: no rationale found (nothing in the 2026-07-23 review considers attributing a hang to a check)
- Owner-gated: yes for a progress hook or `check_with` variant (new API); the doc fix is not gated
- Recommendation: adopt the doc sentence on `check` now (the focused `check_*` functions exist for exactly this); add a progress-reporting `check_with` only if a transport author asks (open question 10).

The module doc says the liveness clauses fail as hangs that only the caller's timeout can bound, and `check` runs seven probes sequentially under that one timeout. When it fires the caller learns that the suite hung, not which clause: both in-tree consumers report exactly that. A transport author then re-runs the focused checks by hand to bisect. The focused `check_*` functions exist and are public, but nothing tells the reader that attributing a hang is what they are for.

Evidence:

       147	/// Run the whole conformance suite against fresh pairs from `pair`.
       148	///
       149	/// Each check consumes one fresh pair (`pair` is called once per check),
       150	/// and every focused check probes both directions of its pair, so the
       151	/// suite validates an asymmetric implementation on each side's connector
       152	/// and acceptor. See the [module docs](self) for executor and timeout
       153	/// requirements.
       154	///
       155	/// # Panics
       156	///
       157	/// On the first violated contract clause, with a description of the clause.

Resolution: Minimal and non-breaking: add to `check`'s docs that a hang under the caller's timeout does not identify the check, and that the focused `check_*` functions exist so each can run under its own timeout. Owner option: a progress hook (`check_with(pair, on_check: impl FnMut(&'static str))`) reporting each check's name as it starts, so the consumer's expiry message can name the check it entered last. Acceptance: `check`'s rustdoc tells the reader how to attribute a hang, or a progress-reporting entry point exists and `tests/tcp_link.rs` and `tests/routed_link.rs` use it.

### conformance-37: The cancellation probe requires two connects to complete while the peer's acceptor is unpolled, a precondition the link contract does not state
- Where: src/conformance/link.rs:867-872 (related: src/conformance/link.rs:884-887, src/conformance/link.rs:855-859, src/conformance/link.rs:28-55, src/conformance/link.rs:805-815, src/link.rs:51-54, src/link.rs:534-539, src/link.rs:563-564, src/link.rs:598-605, src/link/routed/router.rs:105, src/link/routed/router.rs:249, src/tree/mirror/streaming/remote/proxy/work.rs:212-220, src/tree/mirror/streaming/remote/streams.rs:694-706)
- Class / severity / confidence: api-surprise / low / medium
- Provenance: verified (read the probe's ordering at 861-887; `grep -n -i 'backlog\|rendezvous\|unaccepted' src/link.rs` hits only the `MEMORY_STREAM_BACKLOG` block at 534-539, so the contract section at 24-68 has no such clause; both in-tree connectors queue `STREAM_COUNT` unaccepted opens, `mpsc::channel(MEMORY_STREAM_BACKLOG)` at src/link.rs:563-564 with `MEMORY_STREAM_BACKLOG = STREAM_COUNT` and `mpsc::channel(STREAM_COUNT)` at router.rs:105 and :249; the protocol's own acceptor is polled beside the session in the biased `select!` at work.rs:212-220 and loops on `accept_one` at streams.rs:694-706)
- Seen by: correctness (the lens ran after the other three; see the partition summary); refutation: confirmed (the refutation prefers resolution (2): the protocol never needs the backlog); history: no rationale found (the connect-before-signal ordering is deliberate and argued inline at link.rs:855-859, from cbc4a0aa and a3da46c4, and the first-real-accept bridge came from the link-transport review; nothing considers the precondition that ordering imposes on the transport, and the routed-link record assumes a backlog exists without elevating it to a clause)
- Owner-gated: yes: whether the precondition becomes a contract clause in src/link.rs or a stated requirement of the suite
- Recommendation: consider: state the precondition in the suite (the module's "What the suite cannot see" section and `check_accept_cancellation`'s rustdoc) rather than adding a contract clause, since the protocol's own acceptor loop never needs a backlog; promote it to a clause only if transports are to be held to what the in-tree instantiations already provide (open question 10 is the neighbouring conformance decision).

`probe_cancellation` connects `CANCELLED_DELIVERIES` streams and only then signals the receiving half, which awaits that signal before its first `accept`, so both opens must complete while the peer's acceptor is unpolled: the transport has to hold at least two opened, unaccepted streams. The contract's Concurrency clause forbids serializing an open behind *unrelated* stream progress and says nothing about the peer's acceptance of the same stream, and the memory link's backlog comment says the protocol never relies on one ("the accept loop drains continuously"), which the protocol's code bears out: its acceptor is polled beside the session in the biased `select!` at work.rs:212-220 and loops at streams.rs:694-706. A transport whose `connect` completes only when the peer accepts (a rendezvous over a zero-capacity channel) satisfies every stated clause and hangs this check as an unattributed liveness failure. No committed transport exercises the case: the memory and routed connectors each queue `STREAM_COUNT` unaccepted opens, and TCP has the kernel's listen backlog. A public suite's preconditions must be the contract's clauses, and the module keeps a "What the suite cannot see" section for statements of exactly this kind; what the suite additionally *requires* belongs beside it.

Evidence:

       867	        let mut streams = Vec::with_capacity(CANCELLED_DELIVERIES);
       868	        for _ in 0..CANCELLED_DELIVERIES {
       869	            let (tx, _) = connector.connect().await.expect("contract: connect");
       870	            streams.push(tx);
       871	        }
       872	        let _ = connected.send(());

    src/conformance/link.rs:
       884	    let receive = async {
       885	        in_flight
       886	            .await
       887	            .expect("the sending half signals after connecting");

    src/link.rs:
        51	//! - **Concurrency.** Up to [`STREAM_COUNT`] streams per direction may be
        52	//!   open at once, while [`Connector::connect`] calls arrive sparsely and
        53	//!   mid-session. An instantiation must not require a full complement of
        54	//!   streams, nor serialize an open behind unrelated stream progress.

    src/link.rs:
       536	/// A session opens at most [`STREAM_COUNT`] streams, sessions are
       537	/// serialized, and the accept loop drains continuously, so this bound is
       538	/// never the limiting factor for an honest peer.

Resolution: Owner decision between (1) adding a clause to the contract in src/link.rs stating that `connect` completes without the peer polling `accept`, for at least `STREAM_COUNT` outstanding opens per direction (the bound both in-tree connectors already provide), cited from the probe; and (2) stating the probe's precondition in the module docs' requirements section and in `check_accept_cancellation`'s rustdoc, so a transport author knows why an otherwise conforming rendezvous connect hangs this check. The refutation and history passes both point at (2), because the protocol's acceptor loop never needs the backlog, so the requirement is the probe's alone. Either way, the sentence at src/link.rs:536-538 and the new statement should agree on whether the protocol itself relies on a backlog. Acceptance: either src/link.rs's contract section carries a clause the probe's precondition follows from, or the conformance module docs name the precondition beside the other liveness caveats; `memory_link_conforms`, `tests/tcp_link.rs`, and `tests/routed_link.rs` unchanged.

## Tree core

### tree-core-2: `#[derive(Debug, Eq)]` on `Tree<T>` and the four derives on `Snapshot<T>` impose `T` bounds no impl can use
- Where: src/tree.rs:90-91 (related: src/tree.rs:93-98, src/tree.rs:137-156, src/snapshot.rs:12-17, src/message.rs:533-539, src/tree/typed/untyped.rs:684-695, src/tree/typed/height.rs:16-21, src/peer.rs:182-195, src/rumors.rs:92 and 354)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: assessed (read; the bound each derive adds is language-defined: `#[derive(Clone)]` on `Foo<T>` expands to `impl<T: Clone> Clone for Foo<T>`, which `height.rs:16-21` states in the crate's own words)
- Seen by: perfapi (45), correctness (41); refutation: confirmed; history: no-rationale-found (300e2298 deliberately hand-wrote `PartialEq` for `Tree` to avoid a `T` bound and left `Debug`/`Eq` derived; `Snapshot`'s derives arrived in a WIP commit, 36df73797, and were never revisited)
- Owner-gated: yes: relaxing the bounds on public trait impls of `Snapshot<T>` is a public-API change, even though it is strictly widening
- Recommendation: adopt, as one change with api-core-5 and api-audit-1; the tree partition owns the `Tree<T>` derive at src/tree.rs:90 that is the upstream source of `Snapshot`'s `T: Debug` and `T: Eq` bounds.

`Tree<T>` hand-writes `Clone`, `PartialEq`, and `Default` with no bound on `T` (137-156) and holds `PhantomData<fn() -> T>` so that "auto-traits never descend into `T`" (96-97), yet `#[derive(Debug, Eq)]` adds `impl<T: Debug> Debug` and `impl<T: Eq> Eq`, so `Tree<T>: PartialEq` for every `T` while `Tree<T>: Eq` only for `T: Eq`. Neither derived impl can inspect `T`: `Message`'s `Debug` prints only the serialized hex and `Node` equality is pointer-or-hash. The user-visible consequence lands in `Snapshot<T>`, whose `#[derive(Clone, Debug, PartialEq, Eq)]` makes `Snapshot<T>: Clone` require `T: Clone`, although `Rumors::snapshot` (an impl with no `T` bounds, rumors.rs:92) hands one out for any payload `Peer::seed` accepts (`Serialize + DeserializeOwned + Eq + Send + Sync + 'static`, no `Clone`) and its doc promises cloning "shares structure with the live set rather than copying it". This is the types-first principle and the crate's own rule at `height.rs:16-21`: derived impls carry inherited `T: Trait` bounds, which is why the phantom is `fn() -> T`.

Evidence:

    src/tree.rs
    90    #[derive(Debug, Eq)]
    91    pub struct Tree<T> {
    ...
    95        /// Storage is erased ([`Message`] holds `dyn Any`); the facade's `T`
    96        /// names the type its faces downcast to (as `fn() -> T`, so
    97        /// auto-traits never descend into `T`).
    98        payload: PhantomData<fn() -> T>,

    src/snapshot.rs
    16    #[derive(Clone, Debug, PartialEq, Eq)]
    17    pub struct Snapshot<T> {

    src/tree/typed/height.rs
    16    // Hand-rolled trait impls below rather than `#[derive(...)]` because each
    17    // derived impl would carry an inherited `T: Trait` bound (e.g.
    18    // `#[derive(Clone)]` expands to `impl<T: Clone> Clone for S<T>`), which

Resolution: Replace the derives with bound-free impls: `impl<T> Debug for Tree<T>` (format `root` only) and `impl<T> Eq for Tree<T> {}`; the same four for `Snapshot<T>` (`Clone`, `Debug`, `PartialEq`, `Eq`, each without a `T` bound). Add a compile test (a doctest or a `tests/` unit) that clones and compares a `Snapshot<NotClone>` where `NotClone: Serialize + DeserializeOwned + Eq` only. Acceptance: the compile test builds; `cargo doc` shows `impl<T> Clone for Snapshot<T>` with no `T: Clone`; `Tree<T>` implements the same trait set for every `T`.

### tree-core-5: `Iter` is unnameable from outside the crate yet is the public `IntoIterator::IntoIter` of `&Snapshot<T>`
- Where: src/tree.rs:174-176 (related: src/snapshot.rs:7, 105-112, 172-178; src/lib.rs:317, 322, 347)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read `src/lib.rs:304-349`: `mod snapshot` and `mod tree` are private and `Iter` appears in no `pub use`; `snapshot.rs:7` re-exports it only inside the private module; `snapshot.rs:172-174` names it as `type IntoIter = Iter<'a, T>` while `Snapshot::iter` at 105-107 returns an opaque `impl`)
- Seen by: perfapi (49); refutation: confirmed; history: deliberate-and-holds for the opacity (8dc0596ed made `Snapshot::iter` opaque and dropped `Iter` from the root to hide engine internals) but that same commit left `IntoIterator for &Snapshot` naming the now-hidden type; the finding brings that new evidence and is kept, owner-gated
- Owner-gated: yes: either resolution changes the public API (re-export `Iter`, or make `into_iter`'s type opaque, or re-export and return the concrete type from `iter`)
- Recommendation: adopt option (a): re-export `Iter`, return it by name from `Snapshot::iter`, keep `typed::Iter` private inside it (which meets 8dc0596ed's goal), and add `FusedIterator`, `Debug`, and `Clone` where the inner iterator supports them; fresh-eyes-4, inventory-1, and api-audit-2 file the same defect and resolve with it (open question 1).

`Snapshot::iter` returns an opaque `impl DoubleEndedIterator + ExactSizeIterator + Send + Sync` while `(&snapshot).into_iter()` returns the concrete `Iter<'a, T>`, which no downstream path can name (`rumors::Iter` does not exist). A user who wants to hold the iterator in a struct must write `<&'a Snapshot<T> as IntoIterator>::IntoIter`. The two entry points to one traversal have different, and in one case unnameable, types; std's convention (`Vec::iter` returns `slice::Iter`, and `IntoIterator for &Vec` uses the same type) is the idiom the design at ddc57045f originally followed. `Iter` also implements none of `Clone`, `Debug`, or `FusedIterator`.

Evidence:

    src/tree.rs
    174    /// An [`ExactSizeIterator`] (the live-message count is known up front) and a
    175    /// [`DoubleEndedIterator`].
    176    pub struct Iter<'a, T>(typed::Iter<'a>, PhantomData<fn() -> T>);

    src/snapshot.rs
    105        pub fn iter(
    106            &self,
    107        ) -> impl DoubleEndedIterator<Item = (&Version, Arc<T>)> + ExactSizeIterator + Send + Sync
    ...
    172    impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    173        type Item = (&'a Version, Arc<T>);
    174        type IntoIter = Iter<'a, T>;

    src/lib.rs
    317    mod snapshot;
    ...
    347    pub use snapshot::Snapshot;

Resolution: Owner decision between (a) re-exporting `Iter` at the crate root, having `Snapshot::iter` return `Iter<'_, T>` concretely, and adding `Debug`/`Clone` where `typed::Iter` supports them and `FusedIterator` if `typed::Iter` keeps returning `None` after exhaustion; or (b) keeping opacity everywhere by boxing or wrapping `into_iter`'s type so no public signature names a hidden type. (a) matches std and 8dc0596ed's goal can be met by keeping `typed::Iter` private inside the wrapper. Acceptance: `Snapshot::iter` and `(&snapshot).into_iter()` have the same type; if that type is public, `cargo doc` lists it and a doctest stores it in a struct field by name.

### tree-core-6: `latest`/`earliest` read as a symmetric pair but bound different sets, and the tree-level docs do not say so
- Where: src/tree.rs:219-227 (related: src/snapshot.rs:40-56)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: verified (read both bodies: `latest` returns `Root::ceiling`, `earliest` the live leaves' floor; `snapshot.rs:52-53` carries the "unlike `latest`" disclaimer)
- Seen by: perfapi (55); refutation: confirmed; history: deliberate-and-holds (02bf7e346 introduced the pair as designed; the asymmetry is documented on `Snapshot` but not on `Tree`)
- Owner-gated: yes: any rename or new accessor is a public-API decision
- Recommendation: adopt the tree-level doc statement now; the rename (`frontier`) is worth doing before the first release if at all (open question 16).

`latest` is the causal ceiling, advanced by every send and redaction and, after any redaction, stamping no live message (`get(latest())` is `None`); `earliest` is the floor of the live leaves. Paired names promise paired semantics, and `Snapshot`'s doc has to disclaim the asymmetry. The tree-level docs ("Returns the latest version for the tree") do not state it at all, so a maintainer reading `Tree` alone gets no warning. The design is deliberate and documented at the public face, so this is a nit; the recorded rationale lives only in the commit message.

Evidence:

    219        /// Returns the latest version for the tree.
    220        pub fn latest(&self) -> &Version {
    221            &self.root.ceiling
    222        }
    223
    224        /// Returns the earliest version present in the tree.
    225        pub fn earliest(&self) -> Option<&Version> {
    226            self.root.root.as_ref().map(Node::floor)
    227        }

Resolution: At minimum, state at 219 and 224 what each bounds (the causal ceiling of every action; the floor of the live leaves) so the tree-level docs match `Snapshot`'s. Owner option: rename the ceiling accessor (`frontier()`) so `latest`/`earliest` can be a symmetric live pair, or expose the live ceiling under a distinct name. Acceptance: `Tree`'s two docs state the asymmetry; if renamed, `Snapshot`'s docs no longer need an "unlike `latest`" clause.

## Tree typed

### tree-typed-17: `Node<Root>::get` takes `&[u8]`, and the untyped walk spends two arms on lengths the type could exclude
- Where: src/tree/typed/node.rs:370-374 (related: src/tree/typed/untyped.rs:365-390, src/tree.rs:289-293)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: assessed (read: the sole caller, tree.rs:289-293, holds a `[u8; 32]` from `Path::for_leaf`)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no (crate-private signature)
- Recommendation: adopt: the sole caller already holds the `[u8; 32]`, and the two length arms in the untyped descent become an invariant the type states.

The doc promises a lookup by full 32-byte path and the only caller has one, yet the parameter is a slice, and the untyped descent handles a leftover tail at a leaf (`path.is_empty().then_some`) and exhaustion at a branch (`split_first()?`) that a `&Path` or `&[u8; 32]` parameter would rule out at the boundary (types-first: make the contract the type).

Evidence:

       370	    /// Look up the live leaf whose full 32-byte path is `path`, by a single
       371	    /// `O(depth)` descent.
       372	    pub fn get(&self, path: &[u8]) -> Option<(&Version, &Message)> {

    src/tree/typed/untyped.rs:
       378	                // A full 32-byte path lands exactly at a leaf; a leftover
       379	                // tail means the path was deeper than the tree.
       380	                Children::Leaf { version, message } => {
       381	                    return path.is_empty().then_some((version, message));

Resolution: `get(&self, path: &Path)` (or `&[u8; 32]`) on the typed root, converting once; keep the untyped walk over a slice and restate its length arms as the invariant they then are. Acceptance: `Tree::get` passes the `Path` it already computes; the "leftover tail" comment is gone or restated as an invariant.

## Mirror common

### mirror-common-15: `HandOffDefect::Undecodable` admits `Decode::Io`, a state `decode_party` never produces
- Where: src/tree/mirror/party.rs:43-51 (related: src/tree/mirror/party.rs:125-138, src/error.rs:40, crates/before/src/error.rs:68-92)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (read `decode_party` at party.rs:131-138, which routes `Decode::Io` to `Error::Io`; `before::error::Decode` has `Truncated`, `TrailingBits`, `NotCanonical`, `Io`; `HandOffDefect` is public via error.rs:40)
- Seen by: perfapi; refutation: confirmed; history: deliberate and holds in shape (R5 ruled the typed `Undecodable(before::error::Decode)` form; the `Io` pass-through is stated at `decode_party`, :127-130, not at the variant a public reader opens)
- Owner-gated: no (the one-sentence doc fix; the fuller three-variant reshaping would reopen R5 and is owner-gated)
- Recommendation: adopt the one-sentence doc fix; the three-variant reshaping reopens ruling R5 and is the owner's call.

The public variant wraps `before::error::Decode` whole, but only the non-`Io` variants ever reach it: the body is decoded from a slice and a reader failure surfaces as `Error::Io`. A user matching `HandOffDefect::Undecodable(Decode::Io(_))` writes a dead arm, and the variant doc explains the truncation case but not this one. Types-first: a public payload wider than the states produced is a contract the caller cannot read from the type; where the type stays, the doc must say what the type does not.

Evidence:

    43	    /// The byte string's content is not one canonical party encoding.
    44	    ///
    45	    /// The body arrived whole — exactly the length its head declared —
    46	    /// so this is never a transport cut: the content itself is wrong.
    ...
    51	    Undecodable(before::error::Decode),
    ...
    132	    Party::decode(bytes).map_err(|defect| match defect {
    133	        before::error::Decode::Io(e) => Error::Io(e),

Resolution: Add one sentence to the variant doc: "`Decode::Io` never appears here: the body is decoded from a slice, and a reader failure surfaces as [`Error::Io`]." The fuller alternative (a crate-owned `Truncated`/`TrailingBits`/`NotCanonical` enum with `From<Decode>` for the non-`Io` arms) is an owner decision. Acceptance: the variant doc names the unreachable arm, or the type excludes it.

## Materialized

### materialized-17: `MaterializedError` derives only `Debug`, is exhaustive while its sibling is not, and the enclosing `mirror::Error`'s `Clone` derive is unsatisfiable for the production alias
- Where: src/tree/mirror/streaming/materialized/error.rs:1-8 (related: src/tree/mirror.rs:42-43; src/tree/mirror/streaming/remote/proxy/error.rs:16-18; src/error.rs:53)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: verified (read the three derives: `#[derive(Debug, thiserror::Error)]` here with no `#[non_exhaustive]`; `mirror::Error<C, S>` derives `Clone` at mirror.rs:42; `RemoteError<E>` derives only `Debug` and is `#[non_exhaustive]` at proxy/error.rs:16-18; `MirrorError = mirror::Error<MaterializedError<Infallible>, RemoteError<Infallible>>` at src/error.rs:53)
- Seen by: perfapi; refutation: reframed (the dead `Clone` hinges on both inner types, not this one alone); history: no rationale found (the outer `Clone` predates this type, 2fd590d5)
- Owner-gated: yes: public derives and exhaustiveness
- Recommendation: consider; the recommendation is to derive `Clone, PartialEq, Eq` on both `MaterializedError` and `RemoteError` and keep both exhaustive with the two-variant partition stated in each doc, ruled together with the crate-wide `#[non_exhaustive]` decision (open question 5).

`Clone`/`PartialEq`/`Eq` are free here (`Violation` is `Copy + Eq`; the production `E` is `Infallible`), and a `Clone` derive on the outer `mirror::Error` that neither inner type satisfies is a dead derive. The two inner error enums also disagree on `#[non_exhaustive]`; the two-variant partition ("a backend error or a counterparty Violation") is a defensible reason to stay exhaustive, but the decision should be explicit and made for both types together.

Evidence:

    1	/// A session-fatal failure: a backend error or a counterparty [`Violation`].
    2	#[derive(Debug, thiserror::Error)]
    3	pub enum Error<E> {
    4	    #[error(transparent)]
    5	    Backend(#[from] E),
    6	    #[error(transparent)]
    7	    Violation(Violation),
    8	}

Resolution: Owner decision. Add `Clone, PartialEq, Eq` to both `MaterializedError` and `RemoteError` (or drop `Clone` from `mirror::Error`), and decide `#[non_exhaustive]` for both, recording the reason in each doc comment. Acceptance: `MirrorError: Clone + PartialEq` compiles, or `mirror::Error` no longer claims `Clone`; the exhaustiveness choice is stated at both enums.

### materialized-2: Window saturation is unobservable to the user tuning the budget
- Where: src/tree/mirror/streaming/materialized.rs:147-156 (related: src/tree/mirror/streaming/materialized/work/levels.rs:422, 503; src/tree/mirror/streaming/stats.rs:133-150; src/tree/mirror/streaming/window.rs:55-60)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: verified (read `SessionStats` in full: `window_granted` is the only window field, stats.rs:133-150; the struct is `#[non_exhaustive]`, stats.rs:40; window.rs:57 documents serialization as the degradation)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (487e17ea added `window_granted` as "the window solve's widest granted stage"; 7626543e pinned every counter against a computable dispute oracle, and a stall count is scheduler-dependent, so it could only be pinned as zero versus nonzero)
- Owner-gated: yes: adds a public `SessionStats` field
- Recommendation: consider; the recommendation is to add `window_stalls` as a zero-versus-nonzero readout, because it is the one number that tells a user which way to move `sync_memory_budget`, and the O(1) `capacity() == 0` spot-check is cheap (open question 9).

The window doc says that when a stage's population exceeds its capacity "that stage serializes", and `SessionStats::window_granted` reports only the granted ceiling. Nothing reports whether any send in the macro ever waited on a full edge, so a user who set `Peer::sync_memory_budget` cannot tell whether the budget bound the session or was slack; `SessionStats` is exactly where the crate already hands such readouts back.

Evidence:

    147	        if $resolutions.send(resolution).await.is_err() {
    148	            return;
    149	        }
    150	        for query in $next_queries {
    151	            #[cfg(test)]
    152	            progress::dependent($work, &query);
    153	            if $queries.send(query).await.is_err() {
    154	                return;
    155	            }
    156	        }

Resolution: Owner decision. If wanted: before each window-edge `send().await` in the macro and at the `upper.send` sites in `levels.rs`, spot-check `sender.capacity() == 0` (O(1) on tokio's bounded `Sender`) and count through a new `Recorder::stalled()` into a `SessionStats::window_stalls: u64` field documented like its siblings. Acceptance: a test in `streaming/tests/stats.rs` runs a maximally disputed fixture at `WindowConfig::FLOOR` and asserts `window_stalls > 0`, and the same fixture at a wide window asserts `window_stalls == 0`; `tests/session_stats.rs` re-checks the field publicly.

## Remote codec

### api-audit-14: Diagnostic types expose constructors and protocol-schedule helpers as public API
- Where: src/tree/mirror/streaming/remote/codec/error.rs:19-27 (related: src/tree/mirror/streaming/remote/codec/error.rs:87-94, src/tree/mirror/streaming/remote/codec/error.rs:170-185, src/tree/mirror/streaming/remote/codec/signal.rs:30-91, src/tree/mirror/streaming/remote/codec/signal.rs:119-126, src/reconciliation.rs:201-205)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read the definitions; the callers of `Origin::direction`/`stream`, `EncodeError::new`, `DecodeError::direction`/`stream`, `Stream::new`/`at_height`/`height`, and `Speaker::other` are all inside `src/tree/mirror/streaming`, across `streams.rs`, `erased.rs`, the adapter, proxy, and codec modules)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: narrows public methods
- Recommendation: adopt: every caller of the constructors and schedule helpers is a sibling module, so `pub(crate)` is mechanical, and `Stream::new` is what drags `signal::StreamError` into a public signature (api-audit-2, remote-codec-30).

`Origin::{direction, stream}`, `CodecEncodeError::new`,
`CodecDecodeError::{direction, stream}`, `error::Stream::{new, at_height,
height, COUNT, MAX}`, and `Speaker::other` are `pub` and rendered under
`rumors::error`, so a user can construct codec errors or compute which
logical stream carries a tree height, neither of which any public operation
accepts. `at_height`/`height` bind the public API to the wire schedule (two
heights per stream) that `reconciliation` describes as an internal choice,
and `Stream::new` is what drags the unexported `signal::StreamError` into a
public signature (api-audit-2). Their only callers are sibling modules, so
`pub(crate)` is mechanical.

Evidence:

    src/tree/mirror/streaming/remote/codec/error.rs
    19	impl Origin {
    20	    pub fn direction(speaker: Speaker) -> Self {
    21	        Origin::Direction(speaker)
    22	    }
    23	
    24	    pub fn stream(speaker: Speaker, stream: Stream) -> Self {
    25	        Origin::Stream { speaker, stream }
    26	    }
    27	}

    src/tree/mirror/streaming/remote/codec/signal.rs
    54	    /// Find the stream carrying nodes at `height` for `speaker`.
    55	    pub fn at_height(speaker: Speaker, height: usize) -> Option<Self> {
    ...
    79	    /// Find the node height carried by this stream for `speaker`.
    80	    pub fn height(self, speaker: Speaker) -> usize {

Resolution: make the constructors and schedule helpers `pub(crate)`; keep
the read accessors (`Stream::index`, `Speaker::role`,
`InvalidSignalPlacement::{stream, class}`). Acceptance: the rendered
`error/enum.Origin.html`, `error/struct.CodecEncodeError.html`,
`error/struct.CodecDecodeError.html`, and `error/struct.Stream.html` list
no constructors and no height arithmetic.

### remote-codec-18: `#[non_exhaustive]` follows a rule recorded only in a commit message
- Where: src/tree/mirror/streaming/remote/codec/error.rs:64-65 (related: error.rs:11-12, :41-42, :103-104, :112-114; src/tree/mirror/streaming/remote/codec/frame.rs:376-377 and :404-406; src/tree/mirror/streaming/remote/codec/signal.rs:387-388; src/tree/mirror/streaming/remote/codec/greeting.rs:99-101)
- Class / severity / confidence: api-surprise / low / high
- Provenance: assessed (read: `DecodeErrorKind`, `ListingIssue`, `GreetingError` carry the attribute; `EncodeErrorKind`, `FramePart`, `DecodeLeafError`, `Origin`, `LeafRunError`, `DecodeSignalError`, `StreamClass` do not)
- Seen by: perfapi (45), prose (open question); refutation: confirmed; history: a rule exists in 1e458d69 ("Six enums whose variant sets grow as enforcement grows ... the contract-outcome and wire-grammar enums stay exhaustive deliberately"), applied to `GreetingError`/`ListingIssue` by 0e2e85c6 and to `HeadError` by ruling R3; it is stated nowhere in the tree
- Owner-gated: yes (a semver policy on public enums)
- Recommendation: adopt: state the rule in `src/error.rs`'s module doc and rule on the three unclassified enums (open question 5).

Of the error enums this partition exports through `rumors::error`, three are `#[non_exhaustive]` and seven are not, and the tree states no rule. History has one: taxonomies that grow with enforcement are open; wire-grammar and contract-outcome enums stay closed. Under it `FramePart` and `DecodeSignalError` are wire-grammar and closed by design, and `EncodeErrorKind`, `DecodeLeafError`, and `LeafRunError` are the cases the rule was never explicitly applied to. The choice is deliberate and undocumented: state the rule at one site and rule on the three.

Evidence:

    64	#[derive(Debug, thiserror::Error)]
    65	pub enum EncodeErrorKind {

    112	#[derive(Debug, thiserror::Error)]
    113	#[non_exhaustive]
    114	pub enum DecodeErrorKind {

Resolution: State the rule once, in `src/error.rs`'s module doc or beside the first open enum ("taxonomies that grow as enforcement grows are `#[non_exhaustive]`; wire-grammar and contract-outcome enums are closed"), and decide `EncodeErrorKind`, `DecodeLeafError`, and `LeafRunError` under it. Acceptance: the rule is readable from the tree; each public error enum in the codec is consistent with it.

### remote-codec-28: `GreetingError::Order` is a public variant no public path produces
- Where: src/tree/mirror/streaming/remote/codec/greeting.rs:108-113 (related: greeting.rs:150-156 and :276-279; src/tree/mirror/streaming/remote/proxy/start.rs:317-321; src/tree/mirror/streaming/remote/proxy/error.rs:22-30; src/tree/mirror/streaming/remote/codec/greeting/tests.rs:177-195)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep over src and tests: `GreetingError::Order` is constructed only at greeting.rs:153 and consumed only by the stripping match at :277 and the test at greeting/tests.rs:190; every downstream consumer matches `Error::HandshakeListing`)
- Seen by: structure (3), correctness (36); refutation: confirmed; history: deliberate-but-expired (at 4dd2053c `GreetingError` was `pub(crate)` and `Order` a private routing variant; 0e2e85c6 made the enum `pub` and `#[non_exhaustive]` and re-exported it at `rumors::error` without revisiting the variant)
- Owner-gated: yes (a variant removal on a public enum)
- Recommendation: adopt: the variant has no producer through any public path, and the nested `ListingIssue::Order` already carries the information.

`ListingIssue` already carries `Order(QueryOrderError)`. `parse_greeting` lifts that variant out into a flat `GreetingError::Order`, and `read_greeting` immediately strips it back out into `ReadGreetingError::Listing`, which `start.rs` routes to `Error::HandshakeListing`. Through every public path the `Order` variant never occurs and `GreetingError::Listing(ListingIssue::Order(_))` is never constructed, so a consumer matching on `Order` writes a dead arm. Circular justification: the variant exists to be unwrapped by the next layer, which exists only because the first wrap was done.

Evidence:

    108	    /// The listing map violated a structural rule.
    109	    #[error("greeting listing is malformed: {0}")]
    110	    Listing(ListingIssue),
    111	    /// The listing's keys were not in canonical strictly ascending order.
    112	    #[error(transparent)]
    113	    Order(QueryOrderError),

    152	                listing = Some(parse_listing_map(&mut input).map_err(|issue| match issue {
    153	                    ListingIssue::Order(order) => GreetingError::Order(order),
    276	    parse_greeting(&bytes).map_err(|e| match e {
    277	        GreetingError::Order(order) => ReadGreetingError::Listing(order),

Resolution: Delete `GreetingError::Order`; let `parse_greeting` return `GreetingError::Listing(issue)` unmapped; in `read_greeting` route `GreetingError::Listing(ListingIssue::Order(order))` to `ReadGreetingError::Listing(order)` and everything else to `ReadGreetingError::Decode`. Update `greeting_listing_order_is_enforced` to the nested form. (If the owner would rather collapse `HandshakeListing` into `HandshakeDecode(GreetingError::Listing(..))`, the fix is different; the finding assumes the separate surfacing stays.) Acceptance: `GreetingError` has no variant that no public path produces; `parse_greeting` has no `map_err` on the listing parse; `proxy/start/tests.rs` still observes `Error::HandshakeListing` for descending and repeated radixes.

### remote-codec-30: Public methods on re-exported error types return types no user can name; `Display` routes through `Debug`
- Where: src/tree/mirror/streaming/remote/codec/signal.rs:41 (related: signal.rs:105-110, :361, :375; src/tree/mirror/streaming/remote/codec/error.rs:29-38; src/tree/mirror/streaming/remote/codec.rs:101-103; src/tree/mirror/streaming/remote/error.rs:11-18; src/error.rs:44-50; src/tree/mirror/streaming/remote/streams.rs:266)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (grep over src for `pub use` naming `Signal`, `InvalidSignalState`, or `StreamError` finds only remote/error.rs:18, which re-exports `streams::StreamError`; remote.rs:70 is `pub(crate) mod codec` and lib.rs:322 is `mod tree`, so the `pub use` chain at remote/error.rs:11-16 and src/error.rs:44-50 is the only public reach; no `unnameable_types`, `missing_docs`, `private_interfaces`, or `[lints]` in Cargo.toml, lib.rs, or the justfile)
- Seen by: structure (0), perfapi (42), prose (open question); refutation: confirmed; history: `signal::StreamError` and the `pub` on `Stream::new` date from e41e7069; the `Signal` leak is today's 3327a92b; ruling R3 (re-export `HeadError`) is precedent for the re-export option, not an obstacle
- Owner-gated: yes (public API surface)
- Recommendation: adopt option (a), diagnostic-only, as part of the error-taxonomy ruling (open question 4): every in-crate caller of `Stream::new` already uses `.expect` or `.ok()`, so narrowing costs nothing, and `unnameable_types` closes the class.

`Stream` and `InvalidSignalPlacement` reach users as `rumors::error::{Stream, InvalidSignalPlacement}`, but `Stream::new` returns `Result<Self, StreamError>` where `StreamError` is `signal::StreamError`, and `InvalidSignalPlacement::signal()` returns `Signal`; neither type, nor `InvalidSignalState` behind `Signal::from_state`, appears in any `pub use`. A user calling `Stream::new(17)` gets an error they can match only through `Debug`, and `signal()` yields a value they cannot store in a named binding. The codec's `StreamError` also shares its name with the public `streams::StreamError` re-exported from the same module, so the one type a user can name is not the one `Stream::new` returns; a single-variant enum named like a sibling public type wants to be a struct with a distinct name. The same internal enums leak into public text: `InvalidSignalPlacement`'s message is `"signal {signal:?} on stream {} is invalid for {class}"` and `Origin`'s is `"{speaker:?} direction"`, so renaming a variant rewrites a user-visible message. Nothing catches the class mechanically.

Evidence:

    41	    pub fn new(index: u8) -> Result<Self, StreamError> {

    105	/// A programmatic stream index outside the wire's logical streams.
    106	#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
    107	pub enum StreamError {
    108	    #[error("wire stream index {index} is outside the valid range")]
    109	    Invalid { index: u8 },
    110	}

    361	#[error("signal {signal:?} on stream {} is invalid for {class}", stream.index())]
    375	    pub fn signal(self) -> Signal {
    error.rs:32	            Origin::Direction(speaker) => write!(f, "{speaker:?} direction"),

Resolution: Decide the intended surface. Either (a) the error taxonomy is diagnostic only: narrow `Stream::new`, `Stream::at_height`, `Stream::height`, `Speaker::other`, `Speaker::role`, and `InvalidSignalPlacement::signal` to `pub(crate)` (every in-crate caller of `Stream::new` uses `.expect` or `.ok()`), rename `signal::StreamError` to a struct such as `InvalidStreamIndex { index: u8 }`, and give `Origin` and `InvalidSignalPlacement` a `Display` that does not route through `Debug`; or (b) `Signal`, `Flow`, `End`, and the index error are public vocabulary: re-export them from `remote/error.rs` and `src/error.rs`, rename the index error so it does not collide with `streams::StreamError`, and give `Signal` a `Display`. Either way, enable `unnameable_types` in the crate lints so the gate holds the line. Acceptance: every `pub fn` reachable from `rumors::error` has a signature whose every type is nameable from outside the crate; no two public types in `rumors::error` share a simple name; the `Display` impls of `Origin` and `InvalidSignalPlacement` contain no `{:?}`; the lint reports nothing.

### remote-codec-5: The effective run budget is not observable: saturation is silent, there is no getter, and no session stat records the negotiated minimum
- Where: src/tree/mirror/streaming/remote/codec/budget.rs:128-132 (related: budget.rs:102; src/peer.rs:537-551; src/tree/mirror/streaming/stats.rs:106)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (grep: `MAX_RUN_BUDGET_BYTES` is re-exported nowhere; `Peer`'s `&self` methods are `network`, `warm_caches`, `dangerously_alias_party` at peer.rs:304, :714, :728; `stats.rs` mentions the budget only in doc links)
- Seen by: perfapi (47); refutation: confirmed; history: the setting shipped in f94f2056 with no read-back; no note discusses a getter or stat
- Owner-gated: yes (public API addition)
- Recommendation: adopt the `MAX_RUN_BUDGET_BYTES` re-export and the `Peer` getter now; defer the negotiated-minimum stat until a user asks (open question 9).

`Peer::target_message_size(bytes)` stores `RunBudget::from_bytes(bytes)`, which clamps to `MAX_RUN_BUDGET_BYTES`; that constant is `pub` inside a private module and its value is described only in prose at peer.rs:540-543. A user tuning batching across a mixed fleet can neither read back the value a peer runs at nor learn which end's setting won the session minimum when frames come out smaller than configured.

Evidence:

    128	    pub fn from_bytes(bytes: usize) -> Self {
    129	        Self {
    130	            bytes: bytes.min(MAX_RUN_BUDGET_BYTES),
    131	        }
    132	    }

Resolution: Re-export `MAX_RUN_BUDGET_BYTES` beside `DEFAULT_TARGET_MESSAGE_SIZE` and cite it by name in `Peer::target_message_size`'s doc; add a `Peer` getter returning the saturated value as the greeting advertises it; consider a `SessionStats` field for the negotiated minimum. Acceptance: a test sets a target above the cap and reads back `MAX_RUN_BUDGET_BYTES` through the public getter; a two-peer session with unequal targets reports the minimum in both sides' stats if the field lands.

## Remote capture and codec tests

### remote-capture-atlas-8: `stream_label` returns a nested tuple, and the capture structs derive nothing
- Where: src/tree/mirror/streaming/remote/codec/capture.rs:117-122 (related: src/tree/mirror/streaming/remote/codec/capture.rs:75, src/tree/mirror/streaming/remote/codec/capture.rs:86, src/tree/mirror/streaming/remote/codec/capture.rs:98, src/testing.rs:29-33, tests/common/gossip_snapshot.rs:354, tests/observe.rs:186, tests/observe.rs:217, src/tree/mirror/streaming/remote/codec/capture/tests.rs:233)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (grep of every `stream_label(` call site: four positional destructurings; capture.rs:66-109 read: no `#[derive]` on `LinkCapture`, `HookCapture`, or `HookStream`)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found
- Owner-gated: no
- Recommendation: adopt: a named `StreamLabel` and `#[derive(Debug)]` on the three capture structs are mechanical, and the surface is `test-internals` only.

`((u8, u8), usize)` is `(epoch, index)` plus the label's byte length; the re-export doc at testing.rs:29-30 has to spell out the positions, and every caller writes `let ((epoch, index), label_len) = stream_label(...)`. The three `pub` capture structs exposed through `rumors::testing` derive nothing, not even `Debug`, so an assertion cannot print them. Types-first: named fields over positional tuples, especially across the test-internals boundary.

Evidence:

    117	pub fn stream_label(bytes: &[u8]) -> ((u8, u8), usize) {
    ...
    121	    ((epoch, index), bytes.len() - rest.len())

    86	pub struct HookCapture {

Resolution: Return a `StreamLabel { epoch: u8, index: u8, len: usize }`; update testing.rs:29-33, the three integration-test call sites, and capture/tests.rs:233. Add `#[derive(Debug)]` (at least) to the three capture structs. Acceptance: no `((` destructuring of `stream_label` remains; the three structs print under `{:?}`.

### remote-capture-atlas-35: The rule deciding which exported error enums are `#[non_exhaustive]` lives only in git history
- Where: src/tree/mirror/streaming/remote/error.rs:11-16 (related: src/error.rs:44-50, src/tree/mirror/streaming/remote/codec/error.rs:12, src/tree/mirror/streaming/remote/codec/error.rs:42, src/tree/mirror/streaming/remote/codec/error.rs:65, src/tree/mirror/streaming/remote/codec/error.rs:104, src/tree/mirror/streaming/remote/codec/frame.rs:377, src/tree/mirror/streaming/remote/codec/signal.rs:139, src/tree/mirror/streaming/remote/codec/signal.rs:388, src/tree/mirror/streaming/remote/adapter/error.rs:7, src/tree/mirror/streaming/remote/adapter/error.rs:30, src/tree/mirror/streaming/remote/adapter/error.rs:44, src/tree/mirror/streaming/remote/streams.rs:102, src/tree/mirror/streaming/remote/streams.rs:239, src/tree/mirror/cbor.rs:150)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep of `pub enum` with preceding attributes in each definition file: `#[non_exhaustive]` is present on `DecodeErrorKind`, `ListingIssue`, adapter `DecodeError<E>`, streams `StreamError`, `AcceptError`, `GreetingError`, proxy `Error<E>`; absent on `Origin`, `FramePart`, `EncodeErrorKind`, `DecodeLeafError`, `LeafRunError`, `StreamClass`, `DecodeSignalError`, `ScopeError`, `OpeningError`, adapter `EncodeError<E>`, `ReplyFrameError`, `SendError`, `HeadError`; all are re-exported through remote/error.rs into `crate::error`)
- Seen by: perfapi; refutation: confirmed; history: deliberate and holds, rationale unstated in code (1e458d69: "the contract-outcome and wire-grammar enums stay exhaustive deliberately"; REVIEW.md post-rebase ruling R3: `HeadError` "stays exhaustive (no `#[non_exhaustive]`)")
- Owner-gated: yes (any change to the attribute set is a public API decision; the recorded rule is the owner's)
- Recommendation: adopt: state the rule once and mark each closed enum; this is remote-codec-18 seen from the module that assembles the taxonomy, and open question 5 rules on the two enums it questions.

The taxonomy this module assembles is offered by `crate::error` "for matching and bug reports", and the split between exhaustive and non-exhaustive enums follows a recorded rule (taxonomies that grow with enforcement are open; contract-outcome and wire-grammar enums stay closed) that no enum states at its definition, so to a reader of the code the omissions look like accidents and a future maintainer adding a variant to a closed enum has no signal that it is a semver decision. Two enums also bear a second look against the rule's own criterion: `DecodeSignalError`'s variant set changed in 3327a92b (`Reserved` became `Stream` and `State`), and `LeafRunError` is decode-side structural validation, so both arguably "grow as enforcement grows".

Evidence:

    11	pub use super::codec::{
    12	    DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind, DecodeLeafError,
    13	    DecodeSignalError, EncodeError as CodecEncodeError, EncodeErrorKind as CodecEncodeErrorKind,
    14	    FramePart, GreetingError, HeadError, InvalidSignalPlacement, LeafRunError, ListingIssue,
    15	    Origin, QueryOrderError, Speaker, Stream, StreamClass,
    16	};

Resolution: State the rule once where the taxonomy is assembled (this module's doc or `crate::error`'s) and mark each deliberately closed enum with a one-line comment at its definition ("closed by design: a wire-grammar vocabulary; a new variant is a protocol change"). Separately decide `DecodeSignalError` and `LeafRunError` against the recorded criterion. Acceptance: every `pub enum` reachable from `crate::error` either carries `#[non_exhaustive]` or a closed-by-design comment; the rule is stated in one place the enums can cite.

## Remote adapter and streams

### remote-adapter-streams-22: `SendError` and the adapter's `EncodeError` are exhaustive while their incoming twins `StreamError`, `AcceptError`, and `DecodeError` are `#[non_exhaustive]`
- Where: src/tree/mirror/streaming/remote/streams.rs:238-239 (related: src/tree/mirror/streaming/remote/streams.rs:264-266, src/tree/mirror/streaming/remote/streams.rs:794-796, src/tree/mirror/streaming/remote/adapter/error.rs:43-44, src/tree/mirror/streaming/remote/adapter/error.rs:57-59, src/error.rs:44-50)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: verified (`grep -rn non_exhaustive src/tree/mirror/streaming/remote/ src/error.rs`: present on `StreamError` (streams.rs:265), `AcceptError` (795), adapter `DecodeError` (error.rs:58); absent on `SendError` (238-239) and adapter `EncodeError` (43-44); all five are re-exported through `crate::error`)
- Seen by: perfapi ([37]), structure (open question); refutation: confirmed; history: no-rationale-found (1e458d69 names six enums it opened and says "the contract-outcome and wire-grammar enums stay exhaustive deliberately" without placing `SendError` or `EncodeError` on either side; the CBOR review shows Finch rules this per enum)
- Owner-gated: yes: a public attribute
- Recommendation: adopt as part of the crate-wide `#[non_exhaustive]` ruling (open question 5): `SendError` grows with the stream lifecycle, which is the class 1e458d69 marks open.

The public error taxonomy marks the incoming side open and the outgoing side closed without saying why. `SendError` is the outgoing transport-failure taxonomy (connect, label, frame) and grows with the stream lifecycle, the same class as `StreamError`; a downstream exhaustive `match` on it compiles today and breaks on the first added variant. Either answer is fine; the asymmetry is unexplained at the declaration, which is where 1e458d69's own practice puts the ruling.

Evidence:

       238	#[derive(Debug, thiserror::Error)]
       239	pub enum SendError {

    adapter/error.rs:
        43	#[derive(Debug, thiserror::Error)]
        44	pub enum EncodeError<E> {

Resolution: Either add `#[non_exhaustive]` to `SendError` (and `EncodeError<E>` if the same reasoning holds) or state at each declaration that its variant set is closed and why. Acceptance: every public error enum in `crate::error` either carries `#[non_exhaustive]` or a declaration-site sentence saying why it is closed.

### remote-adapter-streams-21: No frame counts in `SessionStats` although every frame crosses this layer
- Where: src/tree/mirror/streaming/remote/streams.rs:230-233 (related: src/tree/mirror/streaming/remote/streams.rs:478-483, src/tree/mirror/streaming/stats.rs:39-41, src/tree/mirror/streaming/stats.rs:99-132, src/observe.rs:109-117, tests/session_stats.rs)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: assessed (read `SessionStats`: `bytes_sent`/`bytes_received` at stats.rs:118, 132, no frame counter, `#[non_exhaustive]` at 40; `StreamSender::write` and `read_frames` are the per-frame choke points)
- Seen by: perfapi ([40]); refutation: confirmed (facts; a proposal, not a defect); history: no-rationale-found (487e17ea designed `SessionStats` with bytes at the codec boundary and neither proposed nor declined frame counts; the observe hook is the current way to count frames)
- Owner-gated: yes: public API addition
- Recommendation: adopt: one `Relaxed` increment per frame at an existing choke point, and `bytes_sent / frames_sent` is the direct feedback for `target_message_size` tuning that today needs a `StreamObserver` (open question 9).

`SessionStats` reports bytes each direction carried but not frames. `StreamSender::write` and `read_frames` are the single choke points every reconciliation frame crosses, so a `frames_sent`/`frames_received` pair costs one `Relaxed` increment per frame on an already-shared `Recorder`. What a user does with it: `bytes_sent / frames_sent` is the achieved supply batching, the direct feedback for tuning `Peer::target_message_size` (today inferable only by attaching a `StreamObserver` and counting `message` callbacks, observe.rs:109-117); `frames_received` against `messages_gained` and `disputed_scopes` shows how chatty a session was relative to what it moved. `SessionStats` is `#[non_exhaustive]`, so the addition is non-breaking.

Evidence:

       230	        write
       231	            .frame(&(stream, frame))
       232	            .await
       233	            .map_err(SendError::Frame)

Resolution: Add `frames_sent`/`frames_received: u64` with `Recorder::frame_sent()`/`frame_received()` called after a successful `frame` in `StreamSender::write` and on each yielded frame in `read_frames` (whether the consumed `End::Stream` control counts, stated in the field doc); extend `tests/session_stats.rs` with a constructed-corpus expectation. Acceptance: the two fields exist with docs naming the boundary they count at; `tests/session_stats.rs` pins their values on a corpus where the expected frame count is derivable; `just gate` clean.

## Remote proxy

### remote-proxy-2: PayloadDepthMismatch is publicly exported in RemoteError but never reaches a user there
- Where: src/tree/mirror/streaming/remote/proxy/error.rs:31-41 (related: src/peer/gossip.rs:1419-1431, src/error.rs:105-122, src/tree/mirror/streaming/remote/proxy/start.rs:258-268, src/lib.rs:322)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep: raised only at start.rs:262; the only production constructor of `Error::Mirror` is gossip.rs:1430, and gossip.rs:1423-1429 lifts `Server(PayloadDepthMismatch)` first; `mod tree` is private at lib.rs:322 so no caller can drive `Handshaking` directly; tests/payload_depth.rs:292-299 asserts the lifted top-level variant)
- Seen by: perfapi; refutation: confirmed; history: deliberate-and-holds for the lift (71de90c11 chose the top-level variant; gossip.rs:1419-1422 states it inline), but nothing at the variant says so
- Owner-gated: yes: moving the check out of `proxy::Error` changes the public taxonomy; and the doc-only fix re-grows the exact paragraph the owner cut in 9085bd10 (PR #38)
- Recommendation: adopt the one-sentence doc fix now; the structural route (a handshake-level result the driver maps) belongs with open question 4, since it also deletes `streaming_error`'s special case.

A user matching `Error::Mirror(Server(RemoteError::PayloadDepthMismatch { .. }))` has written a dead arm, and nothing on the variant says so; the fact is documented only at the lift site in gossip.rs. Principle: public rustdoc names nothing the API does not reach; the design is deliberate but the rationale lives away from the surface a user reads.

Evidence:

    31	    /// The peer's configured payload depth limit differs from ours.
    32	    ///
    33	    /// Detected symmetrically, after the greetings and before anything
    34	    /// else, so a mixed fleet is caught even on a converged session.
    35	    #[error("peer's payload depth limit ({remote}) differs from ours ({local})")]
    36	    PayloadDepthMismatch {

    gossip.rs:1419	    // The depth-limit mismatch is a configuration diagnosis, not a
    gossip.rs:1420	    // reconciliation failure: surface it as its own top-level variant.
    gossip.rs:1421	    // Only the proxy (the server side of every production handshake)
    gossip.rs:1422	    // detects it; the materialized participant has no wire.

Resolution: either (a) replace the second sentence of the variant doc with one that states the lift ("Surfaces to users as [`crate::Error::PayloadDepthMismatch`], never under [`Error::Mirror`]."), keeping the doc at two lines; or (b) move the check to a handshake-level result the driver maps, so the proxy enum stops carrying a configuration diagnosis and `streaming_error`'s special case disappears. Acceptance: the variant doc states the lift in one sentence, or the variant is gone from `RemoteError` and gossip.rs:1423-1429 is deleted.

### remote-proxy-3: The public error taxonomy does not say which variants diagnose the local participant and which the peer; TerminalQuery is raised for both
- Where: src/tree/mirror/streaming/remote/proxy/error.rs:63-80 (related: src/tree/mirror/streaming/remote/proxy/error.rs:6, src/tree/mirror/streaming/remote/proxy/error.rs:42-44, src/tree/mirror/streaming/remote/proxy/error.rs:51-53, src/tree/mirror/streaming/remote/proxy/work/encode.rs:64-68, src/tree/mirror/streaming/remote/proxy/work/pump.rs:393-395, src/tree/mirror/streaming/materialized/error.rs:10-12, src/tree/mirror/streaming/remote.rs:69)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep of construction sites: `MissingOpening`, `ExtraOpening`, `UnaskedLocalReply`, `UnansweredRemoteQuery`, `OpeningEncode` are raised only in encode.rs, on the local reply path; `TerminalQuery` is raised at encode.rs:67 (local) and pump.rs:394 (remote); `mod adapter` is private at remote.rs:69)
- Seen by: prose, perfapi; refutation: confirmed both; history: no-rationale-found (the dual use dates from cbfe1aff3; the owner's PR #38 directive on this enum constrains added prose to be terse)
- Owner-gated: yes: nesting or splitting variants changes the public taxonomy; the doc-only route does not
- Recommendation: adopt the doc sentence and the three summary rewordings now; nesting the local-only variants under one `LocalProtocol` variant belongs with open question 4, and remote-proxy-29 (vestigial) reduces `TerminalQuery` to one producer first.

The enum doc names a private module (`adapter`) and internal vocabulary ("distinguished opening", "reply-only boundary"), and nothing classifies the variants by origin. Six variants can only fire if this crate's own protocol or adapter misbehaves; others are peer conformance faults, configuration, or transport. `TerminalQuery` is raised on both a local encode path and a remote decode path, so a reader of `Error::Mirror(Server(TerminalQuery))` cannot tell whom to file against. The walk's `Violation` states its scope up front ("The ways a counterparty can misbehave"). The principle is documentation altitude: public rustdoc answers which-one-do-I-care-about and names nothing the API does not reach; AGENTS.md frames this machinery as a conformance-bug detector, and the first triage question is whose bug it caught. Note that remote-proxy-29 shows the decode-side `TerminalQuery` site is unreachable from wire bytes, so after that lands the variant has one (local) producer and the split becomes a doc change.

Evidence:

    6	/// A protocol or adapter failure while proxying one remote counterparty.
    42	    /// The locally-produced distinguished opening could not be encoded.
    51	    /// A frame constructed by the adapter violated the reply-only boundary.
    63	    /// The local opening stream omitted its distinguished question.
    72	    /// The local protocol produced a reply which answered no remote query.
    78	    /// The terminal responder attempted to ask another leaf question.
    79	    #[error("terminal responder reply contained another query")]
    80	    TerminalQuery,

    encode.rs:66	        } else if !batch.is_empty() {
    encode.rs:67	            return Err(Error::TerminalQuery);
    pump.rs:393	                if !questions.is_empty() {
    pump.rs:394	                    Err(Error::TerminalQuery)?;

Resolution: minimum: add one short sentence to the enum doc naming the two origins ("Variants that name the local protocol diagnose this crate's own participant; the rest diagnose the peer or the transport."), reword the three summaries that name `adapter`/"distinguished"/"reply-only boundary" in user vocabulary, and after remote-proxy-29 make `TerminalQuery`'s doc name the local producer. Cleaner (owner call): nest the local-only variants under one `LocalProtocol` variant so the split is structural. Acceptance: no private module name or internal coinage in error.rs docs; the enum doc states the origin split; `grep -rn 'Error::TerminalQuery' src` shows one producer side.

## Test scaffolding (src/testing, src/tests)

### testing-infra-8: `IoFault`/`IoPlan` admit configurations the mechanism reinterprets without saying so
- Where: src/testing/transport.rs:66-74 (related: src/testing/transport.rs:60-65, 93-107, 180, 194-204, 208-213, 257, 298, 303, 370; src/tree/mirror/streaming/remote/proxy/tests/failures.rs:184-189)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (transport.rs read; failures.rs:184-189 read; `grep -rn 'read_delays\|write_delays\|flush_delays' src tests` finds only generators drawing `0_u8..=2`)
- Seen by: structure-prose [12]; blind-spots [25]; api-economics [38], [39]; refutation: confirmed; history: no rationale found; the operations-only rule for supply faults has been prose since 1a07a0901
- Owner-gated: no
- Recommendation: adopt the enum: the hand-avoidance at failures.rs:184-189 disappears with it, and the delay clamp becomes a named constant either way (open question 17).

`IoFault` pairs any `Operation` with any `FaultUnit`, but `failure` counts flushes, connects, and accepts regardless of unit (194-204), so a `Flush`/`Bytes` fault fires on an operation count while `InjectedIo` reports `unit: Bytes` (208-213); the one strategy that generates faults hand-avoids the combination (failures.rs:184-189). Two more normalizations are invisible from the fields: delays saturate at two polls (`.min(2)`, line 180) though the fields are `Vec<u8>`, and a chunk of 0 becomes 1 (298, 303, 370). Types-first: make the meaningless combination unrepresentable, or state every normalization where the field is declared; named constants over magic numbers for the clamp.

Evidence:

    66	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
    67	pub struct IoFault {
    68	    /// Surface which fails.
    69	    pub operation: Operation,
    70	    /// Successful prefix admitted before failure.
    71	    pub after: usize,
    72	    /// Whether `after` counts operations or bytes.
    73	    pub unit: FaultUnit,
    74	}

    180	        let delay = delays.get(*step).copied().unwrap_or(0).min(2);

    199	            (Operation::Flush, _) => self.report.flushes,
    200	            // Supply operations transfer no bytes; only operation counting
    201	            // is meaningful for them.
    202	            (Operation::Connect, _) => self.report.connects,
    203	            (Operation::Accept, _) => self.report.accepts,

Resolution: Either reshape `IoFault` as an enum carrying the unit only on the byte-moving surfaces (`Read { after, unit }`, `Write { after, unit }`, `Flush { after }`, `Connect { after }`, `Accept { after }`) so failures.rs:184-189 disappears, or document on `IoFault::unit` that it is ignored for `Flush`/`Connect`/`Accept`. Introduce `const MAX_DELAY_POLLS: u8 = 2;` with its reason (a schedule must not starve the peer) and cite it from the three `*_delays` field docs, or drop the clamp and let the generators own the bound; state "a chunk of 0 is treated as 1" on the chunk fields or reject 0. Acceptance: no wildcard over `FaultUnit` remains, or the field docs name every ignored case; `grep -n 'min(2)' src/testing/transport.rs` is empty; `every_transport_fault_surface_is_reachable` still passes.

## Integration tests (tests/common, then the suites)

### tests-common-2: the version a `send` created is recovered five different ways
- Where: tests/common/action.rs:47-64 (related: tests/common/peer.rs:77-87, tests/common/shape.rs:35-44, tests/gossip_snapshot.rs:41-47, tests/retire_redaction.rs:25-30, src/rumors.rs:165, src/rumors.rs:190-203)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read `Rumors::send` at src/rumors.rs:165 and its rationale at 190-203; `sed` of all five recovery sites)
- Seen by: api-economics; refutation: confirmed (keep `insert_one`'s observation-count assertion if the routine is swapped); history: deliberate-and-holds for the API half (the rationale is recorded in the code itself)
- Owner-gated: yes: the API half reopens a ruling recorded at src/rumors.rs:190-203
- Recommendation: adopt the harness half (one recovery routine); decline the API half: the state-machine argument at src/rumors.rs:190-203 stands, and open question 7 records the idiom to document instead.

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

### tests-lifecycle-10: `Peer::seed_rng` is public but hidden and ungated, while the partition depends on it for reproducibility
- Where: tests/network.rs:16-22 (related: tests/bootstrap_snapshot.rs:38-43, tests/retire_snapshot.rs:40-44, src/peer.rs:210-213, src/peer.rs:480-483, src/peer.rs:713-716, src/rumors.rs:413-416, src/network.rs:62)
- Class / severity / confidence: api-surprise / low / medium
- Provenance: verified (src/peer.rs:212-213 `#[doc(hidden)] pub fn seed_rng` with no `cfg`; `sync_window_floor` at 480-483 and `dangerously_alias_party` at 726-728 are gated on `test-internals`; `Network::from_rng` is `pub(crate)`; `warm_caches` at peer.rs:713 and rumors.rs:413 has the same hidden-ungated shape)
- Seen by: api-economics; refutation: confirmed (adds `warm_caches`); history: no-rationale-found (documented public API with a doctest at introduction, hidden two days later in a WIP commit whose message is silent on it)
- Owner-gated: yes: the resolution either documents a public method or gates it
- Recommendation: consider, ruled together with deps-5, api-audit-15, and benches-envelope-6 (open question 6); the recommendation is to gate both methods and, if deterministic networks are a capability users' test suites need, add a documented constructor over a `Network` value with the two-universes hazard stated on it.

The snapshot suites and network.rs reach `Peer::seed_rng` to get a deterministic `Network`. A user writing replay or snapshot tests of an application built on rumors has exactly this need and cannot discover the method; meanwhile it is semver-visible surface with no documentation contract, unlike `sync_window_floor`, which is gated. Hidden-but-public binds the crate to the signature without telling users it exists. `warm_caches` ("For benchmark and test calibration only") has the same shape, and dev-dependencies already enable `test-internals` for benches and tests (Cargo.toml:145).

Evidence:

        16	/// A peer seeded deterministically, so two seeds with distinct stream ids get
        17	/// distinct (but reproducible) networks.
        18	fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>(
        19	    stream: u64,
        20	) -> Peer<T> {
        21	    Peer::seed_rng(&mut SmallRng::seed_from_u64(stream)).sync_window_floor()
        22	}

Resolution: Owner decision covering both methods: (a) un-hide `seed_rng` with a short doc stating the reproducibility use and the one hazard (two universes seeded from equal RNG state share a `Network` and must never interact), or (b) add `#[cfg(any(test, feature = "test-internals"))]` beside the existing `#[doc(hidden)]` on `seed_rng` and `warm_caches`. Acceptance: each is either documented in the rendered rustdoc or unreachable from a default build.

### tests-lifecycle-31: Version recovery after `send` is reimplemented in six shapes; the harness already has `created_version`, and the corpus is evidence for the API decision
- Where: tests/single_peer.rs:20-27 (related: tests/single_peer.rs:216-221, tests/single_peer.rs:357-364, tests/retire.rs:194-198, tests/retire_redaction.rs:25-30, tests/pairwise.rs:193-207, tests/common/action.rs:47-64, tests/common/peer.rs:77-87, src/rumors.rs:143-149, src/rumors.rs:196-207)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -ln created_version tests/*.rs` lists causal.rs, changes.rs, gossip_when.rs, listen.rs, four binaries and none in this partition; `grep -rnE 'find_map\(\|\(v, m\)\|' tests/` lists retire.rs:197 and three sites outside the partition; the other shapes read at the cited lines; rumors.rs:143-149 and 196-207 state the no-return decision with its two reasons)
- Seen by: api-economics, structure-prose; refutation: confirmed (the "five binaries" count corrected to four); history: deliberate-and-holds for the API decision (its rationale is stated inline in the code), no-rationale-found for the harness duplication
- Owner-gated: yes for the API half (whether `send` should return a `Version`, or the docs should name the recommended recovery idiom); no for the harness half
- Recommendation: adopt the harness half (`created_version` everywhere plus a `version_of` oracle helper); for the API half, keep the ruling and name the recovery idiom in `send`'s docs (open question 7).

`Rumors::send` deliberately returns no `Version`. The tests, the crate's heaviest user, then recover it after every send in six shapes: `batch_send` here (pre-frontier snapshot then `range(causally::since(&pre))`), `Peer::insert_one` (drain-based), `created_version` (the harness helper built for exactly this), `find_map` by payload (retire.rs, single_peer.rs:357-364), `.iter().map(|(v, _)| v.clone()).next()` (retire_redaction.rs, single_peer.rs:216-221), and `range(since).next()` inline (pairwise.rs). For the harness, one idiom used everywhere is the legibility rule. For the API, the inline rationale rests on the observe-then-redact shape being the natural one; the corpus is evidence about how often callers instead need the version at the write site, worth weighing knowingly rather than leaving implicit in six workarounds.

Evidence:

        20	fn batch_send(peer: &Rumors<u64>, values: &[u64]) -> Vec<Version> {
        21	    let pre = peer.snapshot().latest().clone();
        22	    peer.send_all(values.iter().copied()).unwrap();
        23	    peer.snapshot()
        24	        .range(causally::since(&pre))
        25	        .map(|(v, _)| v.clone())
        26	        .collect()
        27	}

Resolution: Harness: use `common::action::created_version` (and a sibling `created_versions` for batches) at every single-send site, and add `version_of(&Snapshot<T>, &T) -> Version` to `common::oracle` for the by-payload lookups. API (owner): decide whether `send` should return the version after all, or whether `Rumors::send`'s docs should name the recommended recovery idiom (`snapshot().range(causally::since(&pre))`) so users do not each re-derive it. Acceptance: no partition file contains an inline `range(causally::since(&pre)).next()` or a by-payload `find_map` for version recovery; the API decision is recorded either way.

### tests-common-30: the bootstrap and retire driver family: a pass-through layer the V1 retirement left, and drivers four suites reimplement
- Where: tests/common/wire.rs:230-240 (related: tests/common/wire.rs:213-218, tests/common/wire.rs:244-264, tests/common/schedule/executor.rs:256-275, tests/hop_trace.rs:479-493, tests/bookmark_when.rs:217-228, tests/bootstrap.rs:31-49, tests/retire.rs:52-68, tests/retire_redaction.rs:36-43)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (`git show 368da2a5 -- tests/common/wire.rs` removes the `protocol: Protocol` parameter that gave the wrapper layer its purpose; `sed` of each suite copy: hop_trace.rs and bookmark_when.rs repeat the join-and-double-`expect` handshake and neither calls `assert_control_drained`; bootstrap.rs repeats it to surface the `Option`; retire.rs, retire_redaction.rs, and the executor's `Retire` arm each write the `try_into_peer` + `tokio::join!(retire, gossip)` driver with a slightly different contract)
- Seen by: structure-prose (pass-through; hop_trace/bookmark_when copies), api-economics (retire-into and `Option` bootstrap); refutation: confirmed (the "no `bootstrap().join(` outside common" acceptance is too broad: many of the roughly 45 such sites are the test's subject or run over non-memory links); history: deliberate-but-expired for the wrapper (it fixed `Protocol::V2` until 368da2a5), no rationale for the missing drivers
- Owner-gated: no
- Recommendation: adopt: one driver family in `tests/common/wire.rs` (a `Peer`-returning core, an `Option`-surfacing bootstrap, a configured-builder variant, and `retire_into_async`) replaces five suite copies and the executor's arm; tests-lifecycle-3 (simplification) is the same family seen from the suites.

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

### tests-observation-11: `forked()` re-derives the bootstrap fork without the control-drain assertion because the harness cannot attach a builder observer
- Where: tests/observe.rs:275-289 (related: tests/common/wire.rs:244-264, tests/observe.rs:256-270)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (read wire.rs in full: `bootstrap_fork_configured` is private, constructs `Peer::<T>::bootstrap()` internally, and calls `assert_control_drained` at 262; `forked()` uses `rumors::link::memory()` and no drain check)
- Seen by: api-economics [44]; refutation: confirmed; history: no rationale found
- Owner-gated: no
- Recommendation: adopt: `bootstrap_fork_with(parent, configure)` routed through the common core restores `LINK_BUF`, the window choice, and the drain assertion to `forked()`; it is the configured-builder variant tests-common-30 proposes.

`forked()` reimplements the bootstrap fork to attach an `Observer` on the `Bootstrap` builder, which no `tests/common/wire.rs` entry allows. In doing so it uses the default-capacity `memory()` link rather than `LINK_BUF`, pins no window, and omits `assert_control_drained`, the post-condition every other bootstrap in the partition checks (wire.rs:65-79 explains why a leftover control byte must fail at the session that caused it).

Evidence:

    275	fn forked(observer: Option<&Arc<Recording>>, parent: &Rumors<Vec<u8>>) -> Rumors<Vec<u8>> {
    276	    let (mut near, mut far) = rumors::link::memory();
    277	    let serve = parent.clone();
    278	    let mut bootstrap = Peer::<Vec<u8>>::bootstrap();
    279	    if let Some(observer) = observer {
    280	        bootstrap = bootstrap.observe(observer.clone());
    281	    }
    282	    block_on(async {
    283	        let (peer, served) = tokio::join!(bootstrap.join(&mut near), serve.gossip(&mut far),);

Resolution: Add `bootstrap_fork_with(parent, configure: impl FnOnce(Bootstrap<T>) -> Bootstrap<T>)` (or an `_async` core taking the builder) to `tests/common/wire.rs`, routed through `bootstrap_fork_configured` so `LINK_BUF`, the window choice, and the drain assertion apply; have `forked()` call it. Acceptance: observe.rs contains no direct `Peer::bootstrap()` call in `forked()`; the join session's control stream is asserted drained.

## Benches and examples

### benches-envelope-6: warm_caches is doc(hidden) but unconditionally public on Rumors, Peer, and Snapshot; its only callers are benches and tests
- Where: benches/gossip_fixed.rs:299-303 (related: src/rumors.rs:413-414, src/peer.rs:713-714, src/snapshot.rs:166-167, src/tree.rs:306-307, src/peer.rs:480-483, src/rumors.rs:420-422, Cargo.toml:145)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -rn -B2 'pub fn warm_caches' src` shows `#[doc(hidden)]` with no `cfg` at all four sites; `sync_window_floor` and `dangerously_alias_party` carry `#[cfg(any(test, feature = "test-internals"))]`; `grep -rn warm_caches` finds callers only in benches/ and src/tree/tests.rs; Cargo.toml:145 gives benches the feature)
- Seen by: perfapi; refutation: confirmed; history: deliberate-but-expired (611b325d added it when no `test-internals` feature existed; the feature arrived at 9eadfc680 and the gating convention at db2718d46, and `warm_caches` was not revisited)
- Owner-gated: yes (narrows the public surface)
- Recommendation: adopt, together with api-audit-15 (the same four sites filed from the sweep): benches and tests already build with `test-internals`, so gating costs them nothing (open question 6).

The bench helper calls a method that is documented "For benchmark and test calibration only" yet ships unconditionally on the production handles. Its neighbours with the same purpose are feature-gated. Hidden items are still semver surface and still reachable; the crate's own convention for calibration-only methods is the feature gate, and this one is the odd one out. Benches already build with `test-internals`, so gating costs them nothing.

Evidence:

   299	fn warmed((left, right): (Rumors<u8>, Rumors<u8>)) -> (Rumors<u8>, Rumors<u8>) {
   300	    left.warm_caches();
   301	    right.warm_caches();
   302	    (left, right)
   303	}

Resolution: Add `#[cfg(any(test, feature = "test-internals"))]` to the three public `warm_caches` methods (and `Tree::warm_caches`, whose module is private), matching `sync_window_floor`. Acceptance: `cargo doc` and `cargo check` without features show no `warm_caches` on the public types; `cargo check --all-targets` still builds the benches.

### swarm-example-22: `CountConnector::connect` discards the inner `Done` instead of forwarding completion
- Where: examples/swarm.rs:1166-1176 (related: src/link.rs:55-63, src/link.rs:171-199, src/link.rs:598-605, src/link/routed/stream.rs:55, src/link/routed/router.rs:229)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`MemoryConnector::connect` returns `Done::discard()` at src/link.rs:604, so the swarm is behavior-preserving; `Done` is `Box<dyn FnOnce(Half) + Send>` at :179 and `Done::new` takes `impl FnOnce + Send + 'static` at :191, so the forwarding form fits `Connector::connect`'s `Send` future; the routed link's recycling `Done`s per the perfapi lens's grep)
- Seen by: correctness, perfapi; refutation: confirmed (severity raised to low to match); history: deliberate and holds for the memory link (4be4b830: "plain transports pair with Done::discard()"), silent on wrappers
- Owner-gated: no
- Recommendation: adopt the forwarding form: the example is the crate's only worked decoration of a `Link`, and a reader copying it onto the routed link would sever stream recovery.

The wrapper drops the connector's `Done` and hands out a fresh `Done::discard()`. That is behavior-preserving for `MemoryConnector`, whose own `Done` is `discard`, but the link contract's completion clause makes `Done` where "A transport that reuses connections recovers them", and this is the crate's only worked decoration of a `Link`. A reader copying it onto the routed link (which recycles through `Done`) would sever the transport's stream recovery at every clean end. Examples teach patterns; forwarding is the general form and discarding a coincidence for the in-memory link.

Evidence:

    1166	    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
    1167	        let (tx, _) = self.inner.connect().await?;
    1174	            Done::discard(),

    src/link.rs
    176	/// anywhere else (its handle unused) is an abort. A transport that
    177	/// reuses connections recovers them here; the rest pair every stream
    178	/// with [`discard`](Self::discard).
    604	        Ok((tx, Done::discard()))

Resolution: `let (tx, done) = self.inner.connect().await?;` and return `Done::new(move |w: CountWrite<DuplexStream>| done.complete(w.inner))`, with one comment line: a wrapper forwards the half's clean end to the transport it wraps. Or state inline why discarding is right here (the memory link's `Done` is itself `discard`) if the general form is not wanted in the example. Acceptance: either no `Done::discard()` remains in the example, or its one use carries the stated reason.

### benches-envelope-19: Harvesting versions from a Snapshot pays an Arc clone, downcast, and drop per element; no versions-only enumeration exists
- Where: benches/support/grid.rs:159 (related: benches/in_memory.rs:71, benches/gossip_fixed.rs:295, src/rumors.rs:269-274, src/snapshot.rs:105-112, src/tree.rs:178-183, src/message.rs:513-518)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: verified (snapshot.rs's public methods are `network`, `latest`, `earliest`, `is_empty`, `len`, `hash`, `get`, `iter`, `range`, `warm_caches`; `iter` yields `(&Version, Arc<T>)` through tree.rs:182 `m.arc::<T>()`, which is message.rs:514-516 `self.message.clone().downcast::<T>()`; the inner `typed::Iter` yields `(&Version, &Message)`)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (9c73d7b463 retargeted the surface from `Key` to `Version` and replaced batch-minted key lists with snapshot walks in every fixture; no note considers a borrowed versions iterator)
- Owner-gated: yes (public API addition)
- Recommendation: adopt `Snapshot::versions()` (no `T` bound, borrowed items) and `Snapshot::contains(&Version)`: the underlying `typed::Iter` already yields the borrowed form, and the doc example at src/rumors.rs:269-274 is the first user (open question 8).

Three bench fixtures and the `redact_all` doc example at rumors.rs:269-274 enumerate versions with `snapshot().iter().map(|(v, _)| v.clone())`, and each item pays two atomic operations and a `TypeId` compare for a payload nobody reads. A user of a replicated set would expect membership and version enumeration without touching payloads, and the underlying iterator already has the borrowed form. The `iter` bench (in_memory.rs:107-131) currently measures this refcount traffic as "the walk itself".

Evidence:

   159	    let shared: Vec<Version> = left.snapshot().iter().map(|(v, _)| v.clone()).collect();

Resolution: Propose `Snapshot::versions(&self) -> impl DoubleEndedIterator<Item = &Version> + ExactSizeIterator` over `typed::Iter` (no `T` bound), and consider `Snapshot::contains(&Version) -> bool` beside `get`. Leave `iter()`'s owned-`Arc<T>` shape alone: it is a deliberate trade (the handle outlives the snapshot), and `&Arc<T>` is unavailable because storage is `Arc<dyn Any>`. Acceptance: the three bench sites and the doc example use `versions()`; if a `versions` bench group is added, its column against `iter` shows the payload-handle cost.

### swarm-example-18: The example discards `Gossiped` and never surfaces `SessionStats`, the crate's own per-session measurement
- Where: examples/swarm.rs:743-745 (related: examples/swarm.rs:769-771, examples/swarm.rs:95-112, examples/swarm.rs:121-123, src/peer/gossip.rs:167-180, src/tree/mirror/streaming/stats.rs:39-152)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (grep finds no `Gossiped` or `SessionStats` in either partition file; `Gossiped.stats: SessionStats` read at src/peer/gossip.rs:174-179)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (`SessionStats` landed in 487e17ea one day after the readout work and touched no example; not yet ported rather than declined)
- Owner-gated: no
- Recommendation: adopt: bind the `Gossiped` at both calls and fold its counters into `Metrics`; the example is the crate's showcase and today reports only what it measured itself while discarding what the library measured (pairs with swarm-example-16, correctness).

Both `gossip` calls drop the returned `Gossiped`, so the crate's `SessionStats` (`disputed_scopes`, `messages_gained`, `messages_shed`, `bytes_sent`/`bytes_received` at the codec boundary, `window_granted`) never reaches the readout. The example instead re-derives a transport-level byte count with about 120 lines of decorators and reports nothing that explains the bandwidth (how many messages moved, how many scopes were disputed, how wide the window ran). As the crate's showcase and its documented self-measurement tool, it teaches a reader nothing about `Gossiped`; and reporting only what it measured itself while ignoring what the library measured is the verified-versus-told asymmetry in miniature.

Evidence:

    743	    runtime
    744	        .block_on(rumors.gossip(&mut link))
    745	        .expect("initiator gossip");

    src/peer/gossip.rs
    174	    /// What the session measured about itself; every count is local, so
    179	    pub stats: SessionStats,

Resolution: Bind the `Gossiped` at both calls and fold `stats.messages_gained`, `messages_shed`, `disputed_scopes`, `bytes_sent`, `bytes_received` into `Metrics` as monotonic `AtomicU64`s; add rows (UI and headless) for messages gained/shed per sync and disputed scopes per sync; label the existing row "wire bytes (transport)" beside a "reconciliation bytes (codec)" row so the envelope overhead is visible; update the module doc's readout list. Pairs naturally with swarm-example-16 option (a). Acceptance: `--headless-secs` prints the new rows; `# The readout` lists exactly the rows printed; `grep -n Gossiped examples/swarm.rs` finds a binding.

## Positives

- The public docs alone sufficed to write a complete application: the fresh-eyes sweep's custom `Bookmark`, three-level wire `Observer`, routed TCP `Dial`/`Listen` pair, stoppable `gossip_when` drivers, and all three set observers compiled on the first `cargo check`, the only errors being its two deliberate probes (the finalizer read `check-1.log` and found exactly two `error[` lines).
- Every `#[must_use]` on an outcome type says what a silent drop loses: `Bootstrap` and `BookmarkedBootstrap` (does nothing until `join`), `Joined` ("every `Joined` variant carries a peer or the bookmark; dropping it loses one or the other", src/peer/bootstrap.rs:364), `Retire` (leaks the identity), `Unbookmarked` (strands the identity), and `gossip_when` (the driver does nothing until polled).
- `Bootstrap` and `BookmarkedBootstrap` form a typestate that gives each `join` only its own outcomes (src/peer/bootstrap.rs:262-273), so a persist failure after a successful session cannot be mistaken for success; the manual `Clone` (81-84) and `Debug` (286-287) each carry their reason.
- `Batch` is a closed scope handle enforced by the borrow checker: two `compile_fail` doctests (src/rumors.rs:320-336) pin both escape routes.
- `Peer`, `Rumors`, and `Bootstrap` implement `Debug` by hand, print a bounded summary independent of `T: Debug`, and say why (src/peer.rs:182-193, src/rumors.rs:79-90, src/peer/bootstrap.rs:96-105): the standard the derive findings ask the outcome types to meet is one the crate already wrote down.
- The `error` module doc opens with a table mapping every top-level variant to the replica's state and the caller's next action (src/error.rs:10-28); `Error::Epilogue`'s doc (124-159) states the two-generals residue and what the exchange protects; `Error::widen` closes the uninhabited arm with `match never {}`, so adding a variant cannot mis-widen silently.
- `Link`'s "What a session promises" (src/link.rs:279-321) states the `Ok`, `Err`, and cancellation contract, names the three qualified exceptions to "unchanged", and puts the timeout with the caller; the contract section (src/link.rs:24-68) gives, for every clause, the mechanism, the failure it precludes, and the carrier shape that would violate it, and every clause has a conformance check with a committed negative control.
- `SessionState` is sealed exactly right for a value wrappers must carry: `Copy`, private fields, public getters, `pub(crate)` mutators (src/link.rs:345-407); `Done<Half>` lets single-use transports write `Done::discard()` and recycling transports hand the connection back, and it composes through the erasure layer without a second concept.
- `Gossiped`, `SessionStats`, `SessionInfo`, `StreamInfo`, and `StreamId` are `#[non_exhaustive]` data carriers with public fields, so the counters this document asks for are non-breaking additions; `SessionStats`'s field docs follow one shape (the mechanism, where the count is taken, when it is zero), and tests/session_stats.rs pins the byte counters against an independent transport-level tally.
- `PayloadDepthLimit` is a proper newtype with `const fn new` and `get`, `Display`, `Default`, and a named default constant; `Network` is opaque, `Copy`, totally ordered, serializes as its 16 raw bytes, and reserves its all-zero bootstrap sentinel structurally.
- `Snapshot::range` takes anything `Into<causally::Query>` and states that a small causal delta costs work proportional to the delta (src/snapshot.rs:114-161); `Snapshot`'s cheapness claim is true at the mechanism (a `Root` clone is two refcount bumps, `len` is stored, and the hash and bounds are `OnceLock` memos).
- Deliberate omissions are explained where a user would look for the feature: src/rumors.rs:196-207 says why `send` returns no `Version` and gives the intended observe-then-redact pattern; `SessionInfo`'s doc says why there is no session number and what to use instead; src/rumors/changes.rs:36-43 says why `Changes` alone is not a gossip driver and points at `gossip_when`.
- The `observe` hook is rumors-blind (no protocol type in the signature, one whole CBOR item per call), its four contract bullets (Ordering, Never block, Coverage, Cost) are at the altitude a hook consumer needs, and the cost claim is true at every site.
- `Bookmark::store`'s contract names atomicity as a safety obligation the crate cannot check and spells out the consequence (src/bookmark.rs:104-120); the finding against `store` concerns the shape of its parameter, not its contract.
- The conformance suite's `yield_once` avoids `tokio::task::yield_now`, so a caller-built link can be validated under any executor; `link::memory` is deterministic and closed-world; and the link suite's "What the suite cannot see" section (src/conformance/link.rs:28-55) states the black-box limits in terms a transport author can act on.
- The tutorial is a sequence of complete programs with their exact output, and the `Peer` lifecycle example (src/peer.rs:47-116) covers every `Retire` outcome and the mutual-bootstrap bail in one runnable block; `impl From<()> for Gossip` lets `rumors.changes()` plug straight into `gossip_when`.
- tests/api_send_bounds.rs pins the `Send` and `Sync` contract of the handle types and every async entry point at compile time, which is exactly where the derive-bound acceptance tests belong; tests/common/oracle.rs:22-29 implements `Default` by hand so `Oracle<T>` acquires no spurious `T: Default` bound.

## Open questions for Finch

1. The `Snapshot` iterator's type (tree-core-5, fresh-eyes-4, inventory-1, api-audit-2): re-export `Iter` at the crate root and return it by name from `Snapshot::iter`, or keep the `impl` return and give `IntoIterator` a nameable type. Recommendation: re-export and return it by name, keeping `typed::Iter` private inside it (which meets 8dc0596ed's goal of hiding the engine), and enable `unnameable_types` in src/lib.rs as the committed check.
2. `Snapshot: PartialEq` (api-core-35): document `==` as same network, same live set, and same frontier, or drop the impl and re-express tests/routed_link.rs:250 as `hash()` plus `latest()` equality, which reopens the link-transport ruling R13. Recommendation: document.
3. The `Bookmark` trait's shape (session-bookmark-23, api-audit-4): an owned `Vec<u8>` parameter for `store` with `Serialized` deleted, `type Error` folded into `Bookmark` with `BookmarkError` deleted, and `load` kept reader-shaped for file-handle implementors. Recommendation: both changes in one pre-release edit of the trait, since every implementor touches the same impl block; the fresh-eyes application is the worked example of what each costs today.
4. The depth of `rumors::error` (api-audit-13, api-audit-14, remote-codec-30, remote-codec-28, remote-proxy-2, remote-proxy-3): is the roughly thirty-type codec, adapter, stream, and proxy taxonomy public vocabulary, or diagnostic surface for matching and bug reports? Recommendation: diagnostic-only. Make the constructors and schedule helpers `pub(crate)`, rename `signal::StreamError` to a struct with a distinct name, give `Origin` and `InvalidSignalPlacement` a `Display` that does not route through `Debug`, present a concrete `MirrorError` (local violation or wire failure) constructed at `streaming_error` so the `Infallible` arms and the dead `PayloadDepthMismatch` arm disappear together, delete `GreetingError::Order`, and state the local-versus-peer origin split on `RemoteError`.
5. A crate-wide `#[non_exhaustive]` ruling (session-bookmark-46, remote-codec-18, remote-capture-atlas-35, remote-adapter-streams-22, materialized-17; link-17 and api-audit-10 in the idiom and documentation documents). Recommendation: state 1e458d69's rule once in src/error.rs's module doc (taxonomies that grow as enforcement grows are open; wire-grammar and contract-outcome enums are closed); open `EncodeError`, `SendError`, the adapter's `EncodeError<E>`, `EncodeErrorKind`, `DecodeLeafError`, `LeafRunError`, and `DecodeSignalError`; mark each deliberately closed enum with a one-line comment; and derive `Clone, PartialEq, Eq` on `MaterializedError` and `RemoteError` or drop the dead `Clone` from `mirror::Error`.
6. `seed_rng` and `warm_caches` (api-audit-15, benches-envelope-6, tests-lifecycle-10, deps-5). The reports disagree: the inventory sweep recommends un-hiding `seed_rng`, since deterministic seeding is a legitimate need of users' test suites; the tests-lifecycle and deps reports recommend gating both under `test-internals`; the tests-wire-format report recommends keeping `seed_rng` hidden and, if the need is real, offering a documented constructor that takes a `Network` value with the two-universes rule stated on it; and the async-hazards sweep asks whether `warm_caches` should become public API so applications can pre-pay the first greeting's materialization. Recommendation: gate all four now (benches and tests lose nothing), and if application test suites need deterministic networks, add a documented `Network`-taking constructor that rejects the all-zero sentinel and carries the hazard; if pre-paying the first greeting matters to a user, `warm_caches` deserves a documented name with its cost stated rather than a hidden one.
7. `Rumors::send` returning the `Version` it stamped (tests-common-2, tests-lifecycle-31): the state-machine argument at src/rumors.rs:190-203 stands, and the batching argument does not bind the single-message method; the crate's own tests recover the version six ways. Recommendation: keep the ruling, add the recommended recovery idiom (`snapshot().range(causally::since(&pre))`) to `send`'s docs, consolidate the harness onto `created_version`, and record that the six shapes were weighed.
8. Direct readers (benches-envelope-19; the api-audit sweep's open question on `Rumors::{len, is_empty, latest}` and `Snapshot::contains`): should `Snapshot` gain `versions()` and `contains(&Version)`, and should `Rumors` read anything without a snapshot? Recommendation: add `versions` (no `T` bound, borrowed items) and `contains`; leave `Rumors` reading through a snapshot, and say so in the `Rumors` type doc, since the cost is one `Arc` bump.
9. Session, router, and settings observability (session-bookmark-38, remote-adapter-streams-21, materialized-2, remote-codec-5, api-audit-11, link-21, link-16, session-bookmark-20). Recommendation: now, the `SessionObserver::finished` hook, the `frames_sent`/`frames_received` counters, the run-budget getter with the `MAX_RUN_BUDGET_BYTES` re-export, the three settings in the `Peer` and `Rumors` `Debug` output, and the `(LinkInfo, RoutedLink)` tuple from `Endpoint::link`, each small and either non-breaking or pre-release; `window_stalls` as a zero-versus-nonzero readout; router counters only after link-28 lands and only where a known-bad scenario moves them; the offline bookmark inspector when an operator tool first needs it.
10. `conformance::bookmark` (conformance-1) and hang attribution in `check` (conformance-6). Recommendation: ship the bookmark suite with its negative control (a bookmark that commits a partial frame on `Err`) and a paragraph naming what it cannot see (crash atomicity); document on `check` that a hang under the caller's timeout names no check and that the focused `check_*` functions exist for attribution, adding a progress hook only if a transport author asks.
11. `Joined::Bailed` and `Joined::Failed` (fresh-eyes-9): return the whole `BookmarkedBootstrap<T, B>`, or add a sentence to `Bootstrap::bookmark` saying to clone the plain builder first. Recommendation: return the builder; the type's own doc promises the retry, and the outcome should carry what the retry needs.
12. Payload bounds (api-audit-3): reword src/lib.rs:240-242 so only the serde and `Eq` obligations are "demanded once", or put `T: Send + Sync + 'static` on the definitions of `Peer`, `Rumors`, `Snapshot`, and the observers and delete the per-method clauses. Recommendation: the structural fix; it makes the sentence true as written.
13. `pub use ::before;` (deps-6): keep the whole-crate re-export as the version-pinning path for the `before` types in rumors' signatures, or enumerate the reachable items in a `pub mod before`. Recommendation: keep it, record the reason in a one-line comment at src/lib.rs:328, and keep the named re-exports at :330 as the convenience spellings, saying so.
14. `Message` visibility and `unreachable_pub` (session-bookmark-44, api-audit-14): make `Message` and its methods `pub(crate)` with maintainer-altitude docs and enable `#![warn(unreachable_pub)]` crate-wide, or re-export `Message` deliberately so its docs have a reader. Recommendation: `pub(crate)` and the lint; nothing in the public surface needs the erased type, since `Snapshot` yields `Arc<T>`.
15. A second `observe` replaces the first (session-bookmark-40): one sentence on both builders, or accumulate observers and fan sessions out. Recommendation: the sentence, until a consumer needs two observers at once.
16. `latest` and `earliest` (tree-core-6): state at the tree level that `latest` is the causal ceiling of every action and `earliest` the floor of the live leaves, or rename the ceiling accessor (`frontier`) so the pair can be a symmetric live pair. Recommendation: state it now; rename before the first release if at all.
17. `IoFault` (testing-infra-8): reshape as an enum carrying the unit only on the byte-moving surfaces, or document that `unit` is ignored for `Flush`, `Connect`, and `Accept`. Recommendation: the enum; the hand-avoidance in the fault generator disappears with it.
18. Two questions from other documents that bear on the API contract, recorded here so the owner sees them beside their neighbours: `Bootstrap::join` returns `Result<Option<Peer>>` while `BookmarkedBootstrap::join` returns a typed `Joined`, an asymmetry 55e34738 recorded as deliberate and seven test sites double-`expect` (the tests-disruption-handshake partition recommends stating the type-shape rationale in `join`'s doc and leaving the API); and whether the replica-independent causal delivery order the tests pin should be promoted into `CausalMessages`' public contract (tests-observation-3, test-quality; that partition recommends promoting it).
19. `Cargo.toml`'s `description` and `license` (deps-12): the description is published prose in Finch's name. Recommendation: `license = "MPL-2.0"` per the workspace convention, and a one-sentence description drawn from the crate doc's first sentence.

## Counts

Fifty-seven entries: 42 api-surprise and 15 feature-gap. By severity: high 0, medium 8, low 38, nit 11. One entry (conformance-37) was merged after the conformance partition's correctness lens, which hung during the main run, was rerun.

| Module | Medium | Low | Nit | Total |
|---|---:|---:|---:|---:|
| Crate root and public surface | 4 | 7 | 5 | 16 |
| Session and bookmark | 1 | 6 | 0 | 7 |
| Link | 0 | 3 | 0 | 3 |
| Conformance | 1 | 2 | 0 | 3 |
| Tree core | 1 | 1 | 1 | 3 |
| Tree typed | 0 | 0 | 1 | 1 |
| Mirror common | 0 | 0 | 1 | 1 |
| Materialized | 0 | 1 | 1 | 2 |
| Remote codec | 1 | 4 | 0 | 5 |
| Remote capture and codec tests | 0 | 1 | 1 | 2 |
| Remote adapter and streams | 0 | 1 | 1 | 2 |
| Remote proxy | 0 | 2 | 0 | 2 |
| Test scaffolding | 0 | 1 | 0 | 1 |
| Integration tests | 0 | 5 | 0 | 5 |
| Benches and examples | 0 | 4 | 0 | 4 |
| **All** | **8** | **38** | **11** | **57** |

Counts are computed from the entries above at generation time; a dated record of them may live in the review's decision log.
