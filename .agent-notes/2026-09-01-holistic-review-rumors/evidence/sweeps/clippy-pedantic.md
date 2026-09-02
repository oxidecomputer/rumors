# Sweep clippy-pedantic: Pedantic and nursery clippy lints, judged

## Method and coverage

The sweep ran one clippy invocation with `-W clippy::pedantic -W clippy::nursery`
over the rumors package at commit 9e5784fb (log:
`<session scratchpad>/sweeps/clippy-pedantic.log`,
exit 0) and parsed it into deduplicated (lint, file, line) tuples: 2383
crate-wide, 1103 under rumors' own `src/`, `tests/`, `benches/`, and
`examples/`, the remainder in `crates/before` and `crates/suanpan` (out of
scope, excluded). Five lint groups (`use_self`, `redundant_pub_crate`,
`missing_const_for_fn`, `cast_precision_loss`, `cast_possible_truncation`)
account for 64% of the in-scope hits; the sweep judged all five not worth
adopting and I concur with each ruling (reasons under Dropped and Positives).

This finalization pass opened every line range the seventeen sweep findings
cite and quoted from the file, never from the sweep's report. Beyond reading:

- Grep for use sites where a claim depends on them (`link_header`'s callers,
  `MAX_QUERY_CHILDREN`'s type, the receivers of `backend()` in both `work.rs`
  files, `let ... else` and `<'_` counts in `src/`).
- `git log -L` on `pump.rs:175` and `header.rs:248`, `git log -S` on the
  traverse globs; grep of `.agent-notes/` for rulings on casts, pedantic
  lints, globs, and lifetimes. The 2026-08-20 CBOR wire review prescribes
  `usize::try_from`/`u8::try_from` at codec sites (REVIEW.md:473, 581);
  nothing recorded touches lifetimes, globs, receivers, or pedantic lints.
- Read proptest 1.11.0's `prop_assert_eq!` from the cargo registry to settle
  three `redundant_clone` hits.
- Enumerated all twelve `cast_possible_truncation` hits in non-test library
  code and classified each: the five in finding 1; `cbor.rs:111-119`
  (match-arm bounded); `codec.rs:150` (a deliberate wrapping fill);
  `budget.rs:58` (inside a `const` initializer, where `TryFrom` is not
  callable, so `as` is the only spelling); `window.rs:653` (a byte depth
  bounded by the tree height).
- Zero test invocations: no finding rests on a runtime correctness claim.

What this pass could not see: whether any proposed rewrite compiles (no
build permitted), so the `&self` and `'_` changes are compile-unverified;
whether rustdoc actually renders `observe.rs:182` as two code spans (assessed
from rustdoc's inline-code styling, not rendered); the remaining
`redundant_clone` hits beyond the eleven examined.

Under this lens the partition is clean. Nothing surfaced is a lossy cast, a
truncation, or an overflow reachable from wire, payload, or environment
input. One finding at severity low touches signature semantics (`&mut`
receivers that never mutate); one at low is a five-site idiom straggler the
crate's own recorded direction already names; the rest are nits.

## Findings

### clippy-pedantic-1: Range-checked `as` narrowings where `try_from` carries the check
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:455-458 (related: src/tree/mirror/streaming/remote/codec/frame.rs:439-443; src/bookmark/format.rs:360-363; src/tree/typed/untyped/iter.rs:411-413 and 429; src/link/routed/header.rs:248 and 254; house pattern at src/tree/mirror/streaming/remote/streams.rs:729-731; construction check at src/link/routed/endpoint.rs:205)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (lint hits in the run log; every guard and cast read at the cited lines; `MAX_ADDR_LEN` and `MAX_QUERY_CHILDREN` definitions read; `link_header`'s one non-test caller grepped)
- Verification: confirmed; history: deliberate-and-holds for the `try_from` idiom (the 2026-08-20 CBOR wire review prescribes it at codec sites; these five are stragglers), no-rationale-found for the `as` spellings
- Owner-gated: no

Five library sites narrow with `as` one line after a range check (or, at
`header.rs`, a `debug_assert!`) that makes the cast lossless; the reader
pairs check and cast by hand. `u8::try_from`/`usize::try_from` folds them
into one expression whose losslessness is local, which `streams.rs:729-731`
already writes. At `header.rs:254` the only guard is a `debug_assert!`, so a
violated precondition in a release build writes a wrong length byte instead
of failing; the decode side (`header.rs:307`) rejects only `len == 0`.

Evidence:

    frame.rs
    455        if head.major != MAJOR_UINT || head.value > u64::from(u8::MAX) {
    456            return Err(ListingIssue::Shape("listing key is not a radix"));
    457        }
    458        let radix = head.value as u8;

    439        if count > MAX_QUERY_CHILDREN as u64 {
    443            children: Vec::with_capacity(count as usize),

    bookmark/format.rs
    360    if declared > (bytes.len() - reader.at) as u64 {
    363    let payload = reader.take(declared as usize).expect("length checked");

    typed/untyped/iter.rs
    411                        Children::Branch { children, .. } if level.next <= u8::MAX as u16 => {
    413                                .successor(level.next as u8)
    429                            level.next = radix as u16 + 1;

    link/routed/header.rs
    248    debug_assert!((1..=MAX_ADDR_LEN).contains(&addr.len()));
    254    bytes.push(addr.len() as u8);

    house pattern, streams.rs
    729        let stream = u8::try_from(index)
    730            .ok()
    731            .and_then(|index| Stream::new(index).ok())

Resolution: `frame.rs:455-458`: keep the `major` check, then
`let radix = u8::try_from(head.value).map_err(|_| ListingIssue::Shape("listing key is not a radix"))?;`.
`frame.rs:438-446`: `usize::try_from(count).ok().filter(|c| *c <= MAX_QUERY_CHILDREN).ok_or(ListingIssue::Shape(..))?`.
`format.rs:360-363`: `usize::try_from(declared).ok().filter(|d| *d <= bytes.len() - reader.at).ok_or(FormatError::Truncated { len: bytes.len() })?`,
then `reader.take(len)` with its `expect` gone. `iter.rs:411-413`: one arm,
`Children::Branch { children, .. } => u8::try_from(level.next).ok().and_then(|next| children.successor(next)).map(..)`;
line 429 `u16::from(radix) + 1`. `header.rs:248-254`: the minimal change is
`bytes.push(u8::try_from(addr.len()).expect("advertised-name length is validated at endpoint construction"))`
with the `debug_assert!` kept for its lower bound; the types-first change is
a validated-length newtype for the encoded advertised name that
`Endpoint` constructs at line 205 and `link_header` accepts, which deletes
both the assert and the cast. All behavior-preserving on valid input.
Acceptance: no `as` narrowing remains at the five sites; `just gate` clean;
the wire and bookmark snapshot suites unchanged.

### clippy-pedantic-2: `&mut` receivers and a `&mut Context` that never mutate
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:174-175 (related: pump.rs:276-277 and 371-372; src/tree/mirror/streaming/materialized/work/levels.rs:342-343, 540-541, 645-646; src/tree/mirror/streaming/remote/proxy/state.rs:75; src/tree/mirror/streaming/backend/local/adversarial.rs:101)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (lint hits in the run log; every body read; `backend()` is `fn backend(&self) -> B` at `remote/proxy/work.rs:128` and `materialized/work.rs:76`, so no read through `self` requires `&mut`)
- Verification: confirmed; history: no-rationale-found (`git log -L` on pump.rs:175 shows the receiver carried through cbfe1aff, 4d55d484, d27cb5aa, d8bef16b without a stated reason)
- Owner-gated: no

Seven methods take `&mut self` and one helper takes `cx: &mut Context<'_>`,
but every body builds its result from copies and clones (`self.progress`,
`self.backend()`, `self.peer_version_bytes`, `self.peer_supplies.clone()`,
`self.codec`, `self.stats.clone()`, `self.window.capacity(..)`,
`cx.waker().wake_by_ref()`). The signature is the reader's contract for what
a call may change; `&mut` here sends the maintainer looking for a mutation
that is not there.

Evidence:

    pump.rs
    174    fn decode_pump(
    175        &mut self,
    ...
    183        let progress = self.progress;
    184        let backend = self.backend();
    185        let version_bytes = self.peer_version_bytes;
    186        let ledger = self.peer_supplies.clone();
    187        let codec = self.codec;

    state.rs
    75    fn outgoing<H: Height>(&mut self) -> StreamSender<C> {

    adversarial.rs
    101 fn delay(delay: &mut Option<u8>, role: Role, cx: &mut Context<'_>) -> bool {

Resolution: `&self` at pump.rs:175, 277, 372; levels.rs:343, 541, 646;
state.rs:75. At adversarial.rs:101 `cx: &Context<'_>` compiles
(`Context::waker` takes `&self`), but `&mut Context<'_>` is the universal
poll-adjacent convention; an owner taste call, see Open questions. This lint
is nursery and has false positives around closure and async captures, so
apply site by site and compile; a site that fails as `&self` is a lint false
positive to leave in place. Acceptance: each site is `&self` or carries a
one-line note naming the capture that needs `&mut`; `just gate` clean.

### clippy-pedantic-3: Named lifetimes used once, where the crate writes `'_`
- Where: src/tree.rs:190-196 (related: src/link/erased.rs:154; src/peer/gossip.rs:598; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:245 and 350; src/tree/typed/untyped/iter.rs:213, 219, 264)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (lint hits in the run log; every header and signature read; `<'_` appears 125 times in `src/`)
- Verification: reframed: nine of the fourteen hits are single-use lifetimes; the other five (gossip.rs:242, 269, 1186; backend.rs:198; local.rs:191) relate an input to a `BoxFuture<'a, _>` or `impl NodeStream<Self, H> + 'a` output, where the named lifetime states the relation and should stay; history: no-rationale-found
- Owner-gated: no

Nine impl headers and one fn signature name a lifetime that appears exactly
once and relates nothing; the crate's own idiom for such positions is `'_`
(`Context<'_>`, `Formatter<'_>`, `DynLinkParts<'_>`), and a named lifetime
signals a relation the reader then looks for.

Evidence:

    tree.rs
    190 impl<'a, T: Send + Sync + 'static> DoubleEndedIterator for Iter<'a, T> {
    196 impl<'a, T: Send + Sync + 'static> ExactSizeIterator for Iter<'a, T> {}

    erased.rs
    154 impl<'a, 'd> Acceptor for &'a mut (dyn AcceptDyn + 'd) {

    gossip.rs (single use: 'a appears only in `link`)
    598    async fn gossip_inner<'a>(
    602        link: DynLinkParts<'a>,

    excluded: the lifetime relates input to output
    242    pub(crate) fn bootstrap_inner<'a, CR, CW, C, A>(
    244        link: &'a mut Link<CR, CW, C, A>,
    245    ) -> BoxFuture<'a, Result<Option<Self>, Error>>

Resolution: `impl<T: Send + Sync + 'static> DoubleEndedIterator for Iter<'_, T>`
and likewise at 196; `impl Acceptor for &mut (dyn AcceptDyn + '_)` at
erased.rs:154; `link: DynLinkParts<'_>` at gossip.rs:602 with the `<'a>`
dropped; `Exact<'_, R>`, `AsyncFrameDecoder<'_, R>`, `Iter<'_>`,
`Range<'_, P>` at the remaining impl headers. Leave gossip.rs:242, 269,
1186, backend.rs:198, and local.rs:191 as written. Acceptance: the lint's
remaining rumors hits are exactly those five; `just gate` clean.

### clippy-pedantic-4: Closures wrapping one method call where the crate passes the method path
- Where: src/tree/typed/node.rs:414-416 (related: src/tree/traverse/act.rs:96 and 155; src/tree.rs:269; src/link/routed/router.rs:68; src/testing/memnet.rs:90; src/conformance/backend.rs:768; thirteen test sites listed under `redundant_closure_for_method_calls` in the parsed log)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (lint hits in the run log; library sites read; path-form `.map(Type::method)` grepped at 27 `src/` sites)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Twenty `|x| x.method()` closures wrap a single call, and the same chains
already use the path form for their other half (`Hash::empty_root` at
node.rs:416, `typed::node::Root::iter` at tree.rs:325, `TreeNode::ceiling`
at `streaming/tests/fixtures.rs:66, 93, 375, 466`), so one expression mixes
both spellings and the reader wonders whether the closure does more.

Evidence:

    node.rs
    414        node.as_ref()
    415            .map(|n| n.hash())
    416            .unwrap_or_else(Hash::empty_root)

    act.rs
    96        let mut existing_children = node.map(|n| n.into_children()).unwrap_or_default();
    155                    .map(|n| n.ceiling())

    router.rs
    68        .unwrap_or_else(|poisoned| poisoned.into_inner())

Resolution: `.map(Root::hash)`; `.map(Node::into_children)` and
`.map(Node::ceiling)` (turbofish where `H` does not infer);
`.unwrap_or_else(PoisonError::into_inner)` at router.rs:68 and memnet.rs:90;
`cargo clippy --fix -W clippy::redundant_closure_for_method_calls` for the
rest. Acceptance: the lint produces no rumors hit; `just gate` clean.

### clippy-pedantic-5: Glob imports whose source is not a curated surface
- Where: src/tree/traverse.rs:7 (related: src/tree/mirror/streaming/materialized.rs:185; prelude-shaped globs left as they are at src/tree/traverse/act.rs:5, join.rs:39, unknown.rs:17, src/tree/mirror/streaming.rs:75, driver.rs:10, materialized/work/levels.rs:18; idiomatic `use super::*` in a sealed module at src/tree/typed/height.rs:169 and a test-facing `ops` module at src/tree/mirror/streaming/erased.rs:202)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (lint hits in the run log; every import line read; the glob sources read: `typed.rs:24-29` is a curated `pub use` list, `protocol.rs` a trait vocabulary module, `queues.rs` channel constructors "Each function names one edge", `common.rs` one `pub async fn children_of`)
- Verification: reframed: the sweep counted eight library globs as one pattern; six of them import a module that is a deliberate vocabulary surface (`typed`'s `pub use` list, `protocol`, `queues`), where the glob is the honest spelling of "this module's vocabulary"; the two with a real cost are `use super::*` at traverse.rs:7, which inherits tree.rs's entire import list (`PhantomData`, `Arc`, `Version`, `causally`, `Message`, `Node` and every item tree.rs defines), and `use common::*` at materialized.rs:185 for one function; history: no-rationale-found (the traverse globs date to fe3612311)
- Owner-gated: no

A reader orienting by the top of `traverse.rs` learns that its vocabulary
is "whatever `tree.rs` has in scope"; the one-function glob at
`materialized.rs:185` hides a single name behind a wildcard.

Evidence:

    traverse.rs
    7 use super::*;

    materialized.rs
    185 use common::*;

    tree.rs, what the glob inherits
    63 use std::marker::PhantomData;
    64 use std::sync::Arc;
    69 use crate::{Version, causally, message::Message, tree::typed::Node};

Resolution: explicit lists at traverse.rs:7 and materialized.rs:185
(`use common::children_of;`). Leave the `typed::*`, `protocol::*`, and
`queues::*` globs; if a future reader's question is anticipated, one line on
the `mod protocol` declaration at streaming.rs:52 naming it as the
protocol's vocabulary settles it. Acceptance: no glob in a library module
whose source is not a curated vocabulary module; `just gate` clean.

### clippy-pedantic-6: `Eq` bound repeated in three where clauses
- Where: tests/common/peer.rs:115 (related: tests/common/peer.rs:125, 142)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (lint hits; lines 113-146 read)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The bound list names `Eq` twice; the duplicate is inert but reads as if two
constraints were meant.

Evidence:

    115    T: Clone + Eq + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,

Resolution: delete the second `Eq` at lines 115, 125, 142. Acceptance: one
`Eq` per bound list; `just gate` clean.

### clippy-pedantic-7: Long integer literals without digit separators
- Where: tests/single_peer.rs:118-119 (related: tests/disruption.rs:628, 644, 679)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (lint hits; lines read)
- Verification: confirmed, with the weight on single_peer.rs: the line above it already writes `0x9E37_79B9_7F4A_7C15` with separators, and the two constants are the Knuth MMIX LCG multiplier and increment, which a reader may want to check against a reference; the disruption.rs values are proptest-derived seeds with nothing to compare against, so separators help less there; history: no-rationale-found
- Owner-gated: no

Evidence:

    single_peer.rs
    115            let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    118                    .wrapping_mul(6364136223846793005)
    119                    .wrapping_add(1442695040888963407);

    disruption.rs
    644        seed_messages: vec![16893878652516216069, 17088246115921829969],

Resolution: `6_364_136_223_846_793_005` and `1_442_695_040_888_963_407`;
the seed vectors likewise via `cargo clippy --fix -W clippy::unreadable_literal`.
Acceptance: the lint produces no rumors hit.

### clippy-pedantic-8: `contains` then `insert` on a `BTreeSet` where `insert`'s return answers both
- Where: tests/common/overlap.rs:551-552
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (lint hit; lines 544-556 read)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`BTreeSet::insert` returns whether the element was new; one call states the
intent and does one lookup instead of two, and removes the reader's check
that nothing between the two lines changes the answer.

Evidence:

    551                    if !self.ever_known[p].contains(&k) {
    552                        self.ever_known[p].insert(k);
    553                        self.live[p].insert(k);
    554                        self.observed_log[p].push(k);
    555                    }

Resolution: `if self.ever_known[p].insert(k) { self.live[p].insert(k); self.observed_log[p].push(k); }`.
Acceptance: the lint produces no hit; the overlap oracle's suites pass
unchanged.

### clippy-pedantic-9: `match` spelling of `let ... else` in two library sites
- Where: src/bookmark.rs:167-170 (related: src/tree/typed/node.rs:329-332; examples/swarm.rs:684; tests/disruption.rs:747)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (lint hits; sites read; `let ... else` grepped at 107 `src/` sites, e.g. remote/codec.rs:141 and conformance/backend.rs:734)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Two library sites bind through a `match` whose one arm diverges while the
crate writes the same shape as `let ... else` everywhere else; the
`let ... else` form puts the bound name first and the exit second.

Evidence:

    bookmark.rs
    167            let mut reader = match loaded.map_err(BookmarkIo::Io)? {
    168                None => return Ok(BTreeMap::new()),
    169                Some(reader) => reader,
    170            };

    node.rs
    329        let children = match self.inner.into_children() {
    330            Ok(children) => children,
    331            Err(_) => unreachable!("typed nonzero-height node cannot be an uncompressed leaf"),
    332        };

Resolution: `let Some(mut reader) = loaded.map_err(BookmarkIo::Io)? else { return Ok(BTreeMap::new()) };`
and `let Ok(children) = self.inner.into_children() else { unreachable!("typed nonzero-height node cannot be an uncompressed leaf") };`;
the example and test sites likewise. Acceptance: the lint produces no
rumors hit; `just gate` clean.

### clippy-pedantic-10: Wildcard arm over the crate's two-variant `Children`
- Where: src/tree/typed/untyped.rs:356-359 (related: test sites at src/tree/mirror/streaming/remote/adapter/tests/opening.rs:149, src/tree/mirror/streaming/remote/proxy/tests/failures.rs:48, tests/common/sim.rs:446; src/link/routed/tests.rs:545 matches std's `Poll` and is excluded)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (lint hits; `enum Children` at untyped.rs:124-134 read: `Leaf` and `Branch`)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Naming `Children::Branch { .. }` costs one token and turns a future third
variant into a compile error at this site rather than a `None`.

Evidence:

    356        match &self.inner.children {
    357            Children::Leaf { message, .. } => Some(message),
    358            _ => None,
    359        }

Resolution: `Children::Branch { .. } => None`; the three test sites over
crate-owned enums likewise. Acceptance: no wildcard arm over a crate-owned
enum at the four sites; `just gate` clean.

### clippy-pedantic-11: Code span split around an intra-doc link renders as two fragments
- Where: src/observe.rs:182 (related, a different shape: src/tree/mirror/streaming/convert/tests.rs:38, src/tree/mirror/streaming/materialized/work/tests.rs:60, tests/common/window.rs:104)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read; rendering inferred from rustdoc's per-`<code>` inline styling, not rendered)
- Verification: reframed: the sweep's one resolution fits only observe.rs:182, the one public-rustdoc site; the three test-side sites put the link first and a code span after (`` [`key`]`(parent, _)` ``) or bracket a range with two links, so they take a different rewrite and are maintainer-facing; history: no-rationale-found
- Owner-gated: no

Evidence:

    observe.rs
    182        /// The stream's wire index, `0..`[`STREAM_COUNT`](crate::link::STREAM_COUNT):

    convert/tests.rs
    38 /// The parent-height prefix of [`key`]`(parent, _)`.

    tests/common/window.rs
    104 /// [`MIN_BUDGET_EXPONENT`]`..=`[`MAX_BUDGET_EXPONENT`]; the two endpoint

Resolution: `` [`0..STREAM_COUNT`](crate::link::STREAM_COUNT) `` at
observe.rs:182; `` [`key(parent, _)`](key) `` at the two `parent_prefix`
docs; at window.rs:104 either `` `MIN_BUDGET_EXPONENT..=MAX_BUDGET_EXPONENT` ``
with the two constants linked in a following clause, or leave as is (private
test doc). Acceptance: the lint produces no hit on public rustdoc; `just docs`
renders the observe.rs range as one code span.

### clippy-pedantic-12: `get_or_insert` in the arm that already knows the slot is empty
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:486-493
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (lint hit; lines 480-506 read)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Inside the `None` arm of a match on `self.supplies`, the code calls
`self.supplies.get_or_insert(..)`, whose name says the slot might be
occupied when the arm has just established it is not. `Option::insert` has
the same `&mut T` return and states the arm's premise.

Evidence:

    486            let supplies = match &mut self.supplies {
    487                Some(supplies) => supplies,
    488                None => {
    ...
    493                    self.supplies.get_or_insert(Box::pin(early_supplies::<B, _>(

Resolution: `self.supplies.insert(Box::pin(early_supplies::<B, _>(..)))`.
Acceptance: `get_or_insert` no longer appears in a known-`None` arm;
`just gate` clean.

### clippy-pedantic-13: Or-pattern spelled at the outer level where the crate nests it
- Where: tests/disruption.rs:810 (related: the nested form at src/tree/mirror/streaming/remote/codec/capture.rs:708-712)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (lint hit; both sites read)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Evidence:

    disruption.rs
    810            Some(EXIT_BOOT_LOSS) | Some(EXIT_UNCERTAIN) => possible_losses += 1,

    capture.rs, the crate's form
    708        Node::Tag(
    709            TAG_CBOR_SEQUENCE
    710            | TAG_EMBEDDED_ITEM

Resolution: `Some(EXIT_BOOT_LOSS | EXIT_UNCERTAIN) => possible_losses += 1,`.
Acceptance: the lint produces no hit.

### clippy-pedantic-14: Three test clones whose result is discarded or whose original is never used again
- Where: src/tree/tests.rs:1819-1823 (related: src/tree/tests.rs:1847-1851; src/message/tests.rs:70; unexamined same-shape hits at tests/wire_legibility.rs:137, 143, 193 and the other sites listed under `redundant_clone` in the parsed log)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified for the three confirmed sites (read: `v.clone()` binds to `_`; `bytes` has no use after line 70) and for the false positives (proptest 1.11.0 `sugar.rs:792-800` read: `prop_assert_eq!` binds `let left = $left; let right = $right;`, moving both operands)
- Verification: reframed: of eleven examined hits, three are redundant clones, four are lint false positives (tests/bootstrap.rs:71 and 104 and src/tree/mirror/streaming/backend/local/tests.rs:129, where `prop_assert_eq!` moves the clone and the original is moved again on the next assertion; tests/network.rs:36, where the clone is the test's subject: `assert_eq!(rumors.clone().network(), network)`), and two are probable false positives (tests/observe.rs:334 and 340, where the outer `a` and `b` are used again at lines 378-393 of the same test). The lint is nursery for this reason; `cargo clippy --fix` on it is not safe here; history: no-rationale-found
- Owner-gated: no

Evidence:

    tree/tests.rs
    1819    let (_, message) = ours
    1820        .iter()
    1821        .map(|(v, m)| (v.clone(), m.clone()))
    1822        .next()
    1823        .expect("one live message");

    message/tests.rs
    68        let bytes = cbor_vec(&p);
    70        let b = Message::from_bytes::<Payload>(Bytes::from(bytes.clone()), PayloadDepthLimit::default()).unwrap();

    false positive by macro semantics, bootstrap.rs
    71            readout(&bootstrapped.snapshot()), control.clone(),
    75            readout(&provider.snapshot()), control,

Resolution: `.map(|(_, m)| m.clone())` binding `message` directly at
tree/tests.rs:1819-1823 and 1847-1851; `Bytes::from(bytes)` at
message/tests.rs:70. For the remaining hits, examine each by hand; do not
`--fix`. Acceptance: the three sites are rewritten; every other
`redundant_clone` hit under rumors paths is either fixed or noted as a
false positive in the change's description; `just gate` clean.

### clippy-pedantic-15: `Default::default()` and a spelled-out `std::collections::BTreeSet` where the type name would tell the reader what is built
- Where: tests/common/overlap.rs:517-518 (related: tests/common/overlap.rs:509-510, 535)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (lint hits; lines 505-522 and 535 read)
- Verification: confirmed, extended: the same struct spells `std::collections::BTreeSet` in full at three sites, against the doctrine's imports-over-qualified-paths rule; history: no-rationale-found
- Owner-gated: no

Evidence:

    509    ever_known: Vec<std::collections::BTreeSet<EventIdx>>,
    510    live: Vec<std::collections::BTreeSet<EventIdx>>,
    ...
    517            ever_known: vec![Default::default(); n_peers],
    518            live: vec![Default::default(); n_peers],
    519            observed_log: vec![Vec::new(); n_peers],

Resolution: `use std::collections::BTreeSet;` at the file head;
`Vec<BTreeSet<EventIdx>>` at 509-510 and 535; `vec![BTreeSet::new(); n_peers]`
at 517-518. Acceptance: the lint produces no hit; no
`std::collections::BTreeSet` path remains in the file.

## Positives

- `await_holding_lock` (pedantic) produced zero hits anywhere in the run log
  (verified: `grep -c` over the log is 0): no std mutex guard is held across
  an `.await` in rumors.
- `future_not_send` did not fire on the public conformance entry
  `conformance::link::check` (`src/conformance/link.rs:158`, `pub async fn`);
  its five hits are the crate-private, test-only backend gate
  (`src/conformance.rs:19`: `#[cfg(test)] pub(crate) mod backend;`) and two
  test bodies in `src/link/routed/tests.rs`. A library user can spawn the
  public suite on a multi-threaded runtime.
- Every narrowing `as` in non-test library code (twelve
  `cast_possible_truncation` hits, all enumerated above) sits behind a range
  check, a match arm, a `const` bound, or a deliberate wrapping intent; no
  lossy cast is reachable from wire, payload, or environment input.
  `streams.rs:729-731` already writes the `try_from` shape finding 1 asks the
  stragglers to adopt, and the 2026-08-20 CBOR wire review recorded that
  direction.
- Match arms name the enum explicitly rather than `Self::` (a rough regex
  over `src/` counts 154 explicit `=> Type::Variant` arms to 13 `=> Self::`),
  a consistent house style; `error.rs:295`
  (`handshake::Error::Io(error) => Error::Io(error),`) shows the explicit
  name disambiguating two `Error` types in one arm.
- The manual `Clone` impls that `expl_impl_clone_on_copy` flags carry their
  rationale in place (`path.rs:61`: `// Manual copy/clone impls so we don't
  require unnecessary bounds on `H`:`; `prefix.rs:202` likewise); the derive
  the lint suggests would regress it.
- `#[must_use = "..."]` is applied with a stated reason at eight sites, e.g.
  `rumors.rs:616` (`"the driver does nothing until the returned stream is
  polled"`) and `tree.rs:694` (`"dropping the guard disarms the fuse"`), so
  the lone `return_self_not_must_use` hit on `Speaker::other` is rightly not
  adopted.
- `let ... else` is the crate's idiom for a diverging bind (107 sites in
  `src/`); finding 9's two library sites are the only stragglers.

## Open questions for Finch

1. `header.rs:248-254`: minimal `u8::try_from(..).expect(..)` beside the
   kept `debug_assert!`, or a validated-length newtype for the encoded
   advertised name that `Endpoint` constructs at `endpoint.rs:205` and
   `link_header` accepts, deleting assert and cast together? The newtype is
   the types-first answer and the change is crate-private; recommendation:
   the newtype.
2. `adversarial.rs:101`: `cx: &Context<'_>` (accurate) or keep
   `&mut Context<'_>` (the universal poll-adjacent convention)?
   Recommendation: keep `&mut`, and note the lint as a false positive by
   convention if the rest of finding 2 lands.
3. Lint policy: the stragglers in findings 3, 4, 9, 10 stay fixed only if
   something enforces them. `[lints.clippy]` in `Cargo.toml` with
   `elidable_lifetime_names`, `redundant_closure_for_method_calls`,
   `manual_let_else`, and `match_wildcard_for_single_variants` at `warn`
   would ride the existing `-D warnings` gate. Adopt, or leave pedantic
   lints as a periodic sweep? Recommendation: adopt those four; they had
   zero false positives in this run.
4. Carried from the sweep: the CBOR additional-information values 24..27
   recur at `cbor.rs:83, 112-122, 172-184, 273-276`; named constants tying
   the head grammar's functions together is a taste call the sweep raised
   and I did not evaluate further.

## Dropped

- Sweep [2] `used_underscore_binding`/`unused_self` in `progress.rs`: the
  underscore prefix is Rust's spelling for a parameter unused under some
  cfg, and the `#[cfg(test)]` on the next line names the cfg; neither
  `#[cfg(not(test))] let _ = (..)` nor `#[cfg_attr(not(test),
  allow(unused_variables))]` reads better, and the cost of the current form
  is nil. The lint is pedantic because this pattern is idiomatic.
- Sweep [7] `range_plus_one` in `cbor.rs`: `bytes[1..1 + extension]` shows
  the slice length (`extension` bytes after the initial byte) directly,
  which is the quantity a reader of a head parser checks; the inclusive
  form moves that arithmetic into the reader's head. No nameable cost to the
  current form.
- Sweep open-question rulings on `use_self`, `redundant_pub_crate`,
  `missing_const_for_fn`, `cast_precision_loss`, `cast_sign_loss`,
  `cast_possible_wrap`, `or_fun_call` (the six `ok_or` sites),
  `semicolon_if_nothing_returned`, `option_if_let_else`,
  `single_match_else`, `similar_names`, `too_many_lines`,
  `significant_drop_tightening`, `large_types_passed_by_value`,
  `needless_pass_by_value`, `match_same_arms`, `manual_assert`,
  `naive_bytecount`, `suspicious_operation_groupings`, `option_option`,
  `while_float`, `many_single_char_names`, `suboptimal_flops`,
  `duration_suboptimal_units`, `missing_fields_in_debug`,
  `collection_is_never_read`, `match_wild_err_arm`, `ref_option`: concur
  with the sweep's not-adopted ruling on each; none names a cost.
- `budget.rs:58` `radix as u8`: inside a `const` initializer, where
  `TryFrom` is not callable; not a straggler of finding 1.
- Sweep [3]'s five relating-lifetime sites (gossip.rs:242, 269, 1186;
  backend.rs:198; local.rs:191): the named lifetime relates an input to a
  `BoxFuture<'a, _>` or `impl .. + 'a` output; keep.
- Sweep [15]'s hits at tests/bootstrap.rs:71, 104, backend/local/tests.rs:129
  (`prop_assert_eq!` moves both operands; the clone is required),
  tests/network.rs:36 (the clone is the test's subject), and probably
  tests/observe.rs:334, 340 (outer binding reused at 378-393): lint false
  positives.
