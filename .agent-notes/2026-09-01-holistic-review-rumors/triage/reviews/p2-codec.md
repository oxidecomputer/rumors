<!-- CAVEAT LECTOR: review packet for lane p2-codec, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p2-codec

## Goal

The wire codec's decode path had two correctness holes the review
demonstrated with fixtures: a payload resume that could read past the
declared length, and an opener read that re-read the transport after a
recorded failure instead of surfacing it at the item that needed the
missing bytes. This lane lands the exactness clamp in `resume_payload`
with the fixtures as tests through the kept sync-oracle differential,
and the pending-failure slot in the async opener read, so a recorded
transport failure surfaces in wire order.

## Rulings landed

- T33: `resume_payload` clamps every read so no iteration fills past
  `len`; its doc states the remaining precondition; the demonstrated
  fixtures land as tests through the sync-oracle differential.
- T41: the bulk opener read keeps a pending-failure slot and returns the
  recorded failure as a read error at the first item that needs more
  bytes, never re-reading the transport.
- T141: the prose standard.

## Stack position

- Base: `a6a79c39` (main after the memwatch merge; the lane rebased there for its gate of record)
- Parent: `main`
- Children: `p1-harness-crate` (stacked on this lane's tip).

## Acceptance table

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| all | `09f5f25d` | `cargo nextest run -p rumors --all-features --locked --no-fail-fast -E 'test(framing::) \| test(codec::) \| binary(decode_alloc) \| binary(gossip_snapshot)'` (box, scratch worktree) | `108 tests run: 108 passed` |
| all | `09f5f25d` | snapshot files in the diff | 0 |
| `mirror-common-8`, `remote-codec-14` (T33) | `094a75fc` | the `.limit(owed)` clamp removed from `resume_payload`, the framing tests and the lone-record test | `resumed_read_never_consumes_beyond_the_declared_length` FAILS (25 bytes recovered for a 9-byte payload), `resumed_read_matches_whole_read` FAILS at the committed seed's shape, `over_budget_lone_record_read_stays_within_its_frame` FAILS: `the two decoders disagree` (async `NotARecord`, sync `Ok`); the precondition test keeps passing, since only the clamp was removed |
| (same) | `094a75fc` | the precondition and the committed seed | `InvalidData` returned before any read when the prefix exceeds `len` (framing.rs:104-109); seed `cc 4ac1e806… # shrinks to (len, prefix_len) = (1, 0), spare = 2, seed = 0, schedule = [2], cut = None` |
| `remote-codec-11` (T41) | `7510ec37` | `read_frame` made to drop the opener failure instead of recording it, `opener_read_failures_are_reported_in_wire_order` | FAIL: `Initiator, 1 bytes then a close: Truncated { missing: Signal, source: Kind(UnexpectedEof) }` (the failure re-read as a close) |
| prose | `09f5f25d` | em-dash lines in `framing.rs` at base and head | 5 to 3 (the two in `resume_payload`'s rewritten doc) |
| all | `09f5f25d` | `just gate` on the box after the rebase (lane log `gate3.log`) | seven streams ok (`workspace`: `1835 tests run: 1835 passed`); `fuzz` the accepted illumos leg |
| repair `86f65f8f` | `86f65f8f` | `cargo nextest run --workspace --all-features --locked -E 'test(framing::) \| test(codec::)'` (box, at a one-minute load of 90) | `148 tests run: 148 passed` |
| (negative control, re-read) | | `fill`'s failure check moved below its read loop | `opener_read_failures_are_reported_in_wire_order` FAILS: `Initiator, [82, 09, 09] cut after 1, then Fail: the transport was read after it failed; left: 1, right: 0` |

## Fresh-eyes rounds

**Round 1** (surface correctness with operational validity), by sha:
no defect in the production change. The reviewer traced the clamp's
totality (each iteration fills at most the bytes still owed; the
growth branch never runs `Vec`'s own reserve; `Ok(0)` and the zero
remainder handled), the precondition's unreachability from its one
caller, the pending-failure slot through every path (a recorded failure
implies one to two bytes in hand, so the next item's head consumes it
in wire order; no body read can begin with it set), the fixture rows
for both speakers, and the `decode_alloc` meters' invariance by
construction. Repairs landed: the `FailAfter` fixture
gained an after-failure mode (fail again, close, or resume) and a count
of reads after the failure, so the test now asserts that no read
follows the recorded failure and that a resuming transport still holds
its later bytes (a mutant that reads before consulting the slot now
fails by name); two extension-leg rows (`[0x82, 0x18]` and a
`[0x9b]` array head, each cut and then failing) pin the slot through
`Exact::head`'s extension path on both decoders; the differential's doc
says its oracle is the implementation from an empty prefix and that the
absolute clauses carry it; three `async_io.rs` sentences made exact.
The rounds stopped here.

Reviewer notes not acted on: the `InvalidData` precondition surfaces at
the codec as `Read { part: SupplyRun, source: InvalidData }`, a boundary
check presented as a transport failure, unreachable from its one
caller; the atlas still has no async read witnesses (remote-codec-15,
P4, for which the new `FailAfter` fixture is the seed); the remaining
em-dashes in the file's rustdoc await the rustdoc-scope decision on
T48.

## Stops

None. No public signature or rustdoc contract changed; no snapshot moved; the `decode_alloc` meters are bands with derived ceilings and did not move.

## Reading order

### deviations from a stated resolution

- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:265` ([hunk](#hunk-12))

### new tests and negative controls

- mirror-common-8 (T33) at `proptest-regressions/tree/mirror/framing/tests.txt:0` ([hunk](#hunk-2))
- mirror-common-8 (T33) at `src/tree/mirror/framing/tests.rs:197` ([hunk](#hunk-8))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:802` ([hunk](#hunk-14))
- remote-codec-14 (T33) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:1239` ([hunk](#hunk-15))

### production edits

- mirror-common-8 (T33) at `src/tree/mirror/framing.rs:26` ([hunk](#hunk-3))
- mirror-common-8 (T33) at `src/tree/mirror/framing.rs:86` ([hunk](#hunk-4))
- mirror-common-8 (T33) at `src/tree/mirror/framing.rs:95` ([hunk](#hunk-5))
- mirror-common-8 (T33) at `src/tree/mirror/framing.rs:104` ([hunk](#hunk-5))
- remote-codec-14 (T33) at `src/tree/mirror/framing.rs:115` ([hunk](#hunk-5))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:145` ([hunk](#hunk-9))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:153` ([hunk](#hunk-9))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:157` ([hunk](#hunk-9))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:155` ([hunk](#hunk-9))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:167` ([hunk](#hunk-10))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:199` ([hunk](#hunk-11))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:250` ([hunk](#hunk-12))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:262` ([hunk](#hunk-12))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:251` ([hunk](#hunk-12))

### tests and prose

- mirror-common-8 (T33) at `src/tree/mirror/framing/tests.rs:36` ([hunk](#hunk-6))
- mirror-common-8 (T33) at `src/tree/mirror/framing/tests.rs:109` ([hunk](#hunk-7))
- mirror-common-8 (T33) at `src/tree/mirror/framing/tests.rs:112` ([hunk](#hunk-7))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:1` ([hunk](#hunk-13))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:6` ([hunk](#hunk-13))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:810` ([hunk](#hunk-14))
- remote-codec-11 (T41) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:900` ([hunk](#hunk-14))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p2-codec.tsv `@@ -0,0 +1,28 @@`

```diff
@@ -0,0 +1,28 @@
+# Lane p2-codec, rulings T33 and T41. Line numbers are new-side lines of `git diff a6a79c39...HEAD`.
+# Entry ids: mirror-common-8 and remote-codec-14 (one clamp, T33); remote-codec-11 (T41).
+proptest-regressions/tree/mirror/framing/tests.txt	0	mirror-common-8	T33	Negative control. The seed the differential proptest wrote when it ran against the parent tree, before the clamp: it shrinks to an empty prefix in a buffer with two bytes of spare capacity and a two-byte read schedule, the smallest over-read there is (`read_payload` itself is exact because `Vec::new()` has no capacity; any caller-supplied capacity was enough). Committed as the brief requires; it replays before fresh generation from now on.
+src/tree/mirror/framing.rs	26	mirror-common-8	T33	The import the clamp needs: `BufMut::limit` on the `Vec<u8>` borrow. `bytes` is already a direct dependency.
+src/tree/mirror/framing.rs	86	mirror-common-8	T33	Judgment call. I edited this item's doc for the contract below, and the lane's prose rule says a touched paragraph carries no em-dashes, so the two in the item's first paragraph became spaced double hyphens. T48 sweeps only non-doc comments and leaves rustdoc em-dashes to a pending owner decision; this is crate-private rustdoc on one item whose doc I was already rewriting, so I read the rule at the item, not the paragraph. Reversible in one line if the owner wants rustdoc dashes left for that decision.
+src/tree/mirror/framing.rs	95	mirror-common-8	T33	The entry found the contract silent on two things: what bounds a read when the caller's buffer has spare capacity, and what happens to a prefix longer than `len`. The doc now states both in the sentence that already delegates to `read_payload`: every read is bounded by the bytes still owed, and the one remaining precondition with its error kind. Three added lines; they buy the two contract clauses the entry showed were missing, and the paragraph still says nothing about the mechanism (`limit`), which is the reader's-of-the-code business.
+src/tree/mirror/framing.rs	104	mirror-common-8	T33	The precondition check, returning `InvalidData` as T33 rules (not the `debug_assert!` the resolution also allowed): a prefix past `len` is a caller bug, and a caller bug at a contract boundary gets an error in every profile. It costs one comparison before the loop. The one production caller cannot trip it (`lone_record_spans` admits a record only when its heads plus content equal `len`, so the prefix of heads is at most `len`); I say that here rather than in the code because the callee's contract, not the caller's discipline, is what the entry is about.
+src/tree/mirror/framing.rs	115	remote-codec-14	T33	The clamp, closing both entries: each `read_buf` goes through `(&mut payload).limit(owed)`, so the chunk offered to the reader is `min(spare capacity, bytes still owed)` and no iteration can fill past `len`. The growth policy above it is unchanged, as the resolution asks; `owed` is named so the loop reads as the bound it is. Rejected alternative: shrinking the buffer to `len` before the loop, which the resolution also offered; it would reallocate a caller's buffer to enforce a bound that a per-read limit enforces for free.
+src/tree/mirror/framing/tests.rs	36	mirror-common-8	T33	An accessor for the bytes the chunked reader has not delivered, so the differential can assert the trailing bytes stay unread on the resumed side too, not only on the plain cursor.
+src/tree/mirror/framing/tests.rs	109	mirror-common-8	T33	The differential proptest the resolution asks for. The resumed side reads through the existing `ChunkedRead` (every partial-delivery shape), the reference is `read_payload` over a plain cursor holding the whole transcript, exactly the resolution's `read_payload(prefix ++ rest, len)`. `prefix_len` is drawn from `0..=len` so the precondition always holds, `spare` ranges past a whole chunk so the over-read is reachable at every scale, and the cut lands inside the bytes after the prefix so a truncation is always the transport's, never the prefix's. Trailing bytes are appended only on full delivery, where both sides must leave them unread; on truncation both must say `UnexpectedEof`. Rejected alternative: `whole_read_reference` as the oracle, which the neighbouring proptest uses; the resolution names `read_payload`, and holding the two entry points to each other is the point.
+src/tree/mirror/framing/tests.rs	197	mirror-common-8	T33	Negative control. The two witness fixtures from `evidence/witness.md`, in the acceptance's shape: a 3-byte prefix in a capacity-64 buffer, `len` 9, and the 16 trailing bytes `next-frame-bytes`; and a 12-byte prefix against `len` 9. The second asserts more than the witness did: the error kind is `InvalidData` and the cursor is untouched, since T33 fixed the kind. Both failed on the parent tree with the output the commit message records, and pass with the clamp.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	145	remote-codec-11	T41	The pending-failure slot starts empty for every frame; `Exact` is built per frame, so a failure can never carry over to the next frame's opener.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	153	remote-codec-11	T41	The comment that explains the opener read gains the one clause it lacked: where a failure met mid-fill goes. Two added lines; they buy the rule a reader of `read_frame` otherwise has to reconstruct from the slot's doc further down.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	157	remote-codec-11	T41	`Arrived` is destructured so the failure can move into the slot below while `filled` stays a plain count for the head that takes the bytes in hand.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	167	remote-codec-11	T41	The recording, placed after the no-bytes return so a failure with nothing in hand keeps its `FrameHead` classification exactly as before; only the partial-fill case changes. This is the line the entry's fixture turns on.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	250	remote-codec-11	T41	The slot's doc: what it holds and why it exists (wire order over whatever the transport does after failing). The summary sentence is the what; the mechanism sits below the blank line because `doclint` holds summaries to 220 characters and because the maintainer scanning the struct wants the what first.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	262	remote-codec-11	T41	`fill`'s doc gains the one sentence its new first branch needs.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	265	remote-codec-11	T41	Where the failure surfaces. The resolution's example puts it in `fill_exact`; I put it one level down in `fill`, which every read through `Exact` goes through (the opener's heads, a fresh head, a listing's bulk reads), so the rule is total rather than per entry point. The payload reads bypass `Exact` and read the transport directly, and that is safe: a recorded failure means the opener fill stopped short of three bytes, so an opener head runs out of bytes and surfaces it before any body read can begin. Not a deviation: the resolution says "for example".
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	1	remote-codec-11	T41	Imports for the two `AsyncRead` fixtures this lane adds (the fail-after transport and the chunked reader); nothing else in the suite implemented the trait before.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	6	remote-codec-11	T41	Same import group: the tokio trait and its buffer type.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	802	remote-codec-11	T41	Negative control. The failing-reader fixture in remote-codec-15's shape (serve `remaining` bytes, fail once with `Other`, then close or keep failing) and the test T41 names. One struct implements both `std::io::Read` and `AsyncRead`, so the oracle and `FrameRead` read the same transport and the atlas's sync-only `FailAfterReader` is not duplicated here; the P4 lane can move this to the atlas offsets rather than replace it. The test covers every opener delivery count (0 to 3 bytes), both then-close and sticky, both speakers: the acceptance's decisive case is one byte then a close, which classified as `Truncated { missing: Signal }` on the parent tree (the commit message records the run) and as `Read { part: Signal }` now; the sticky and full-delivery rows are the unchanged cases the acceptance also names. Kept additive and placed beside the existing `FailingReader` for the harness-crate lane's rebase.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	1239	remote-codec-14	T33	Negative control. The witness bytes verbatim (`[83 09 07] [d8 3f 44] [d8 3f 41 00]` then `[82 09 09]`), first through `decode_both` (the two decoders disagreed on the parent tree, async `InvalidRun(NotARecord { remaining: 3 })` against sync `Ok`; the commit message records it), then through `FrameRead` over a chunked reader at every chunk size from one byte to the whole transcript, decoding the second frame and the clean close after it: chunk sizes from two up reproduced the over-read on the parent tree, the whole-slice size being the witness's own shape. The chunked reader is a fresh minimal fixture rather than a reuse of the framing suite's `ChunkedRead`, which is private to that module; the harness-crate lane may fold the two. No shared fragment of the suite is touched, per the ordering note against `p1-harness-crate`.
+src/tree/mirror/framing/tests.rs	112	mirror-common-8	T33	Fresh-eyes repair. The doc now says what the differential is: the reference is the same function from an empty prefix, so the comparison alone is a self-check, and the absolute clauses (the payload recovered exactly, the trailing bytes unread on both sides, both truncations `UnexpectedEof`) are what carry the test. Two added lines; they keep a future reader from trusting the comparison arm alone.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	155	remote-codec-11	T41	Fresh-eyes repair. The clause now says "more bytes than arrived": the opener read may return with a failure and fewer bytes than it asked for, and the earlier wording named the fetch, not the delivery.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	199	remote-codec-11	T41	Fresh-eyes repair. `Arrived` is also what a read that only replays the recorded failure returns (nothing filled, the failure); its doc gains that one clause so the struct doc covers every value the type carries.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	251	remote-codec-11	T41	Fresh-eyes repair. The slot summary names the opener read: it is the only read that records into the slot; a bulk read in general (a listing read) never does.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	810	remote-codec-11	T41	Fresh-eyes repair. The fixture gains an after-failure mode (keep failing, close, resume delivering) and a counter of reads made after its failure, so the test holds the decoder to reporting the failure without another read of the transport. The moved-check mutant (the slot consulted after the read loop in `Exact::fill`) passed every earlier row, because a closed transport handed the stored failure back anyway; the commit message records that it now fails on the first sticky row. The resume mode is the third outcome the entry named (a transport that resumes delivering decoded the frame as if nothing had failed), and the unread-bytes assertion pins that those bytes stay in the transport.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	900	remote-codec-11	T41	Fresh-eyes repair. Two rows drive the pending failure through `Exact::head`'s extension leg, which every canonical opener item skips: a stream item `0x18` owed its one-byte extension (both decoders `Read` at `Signal`) and an array head `0x9b` owed eight (both `Read` at `FrameHead`). The rows are table-driven with the canonical opener, so each runs under all three after-failure modes and both speakers.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### proptest-regressions/tree/mirror/framing/tests.txt `@@ -0,0 +1,7 @@`

```diff
@@ -0,0 +1,7 @@
+# Seeds for failure cases proptest has generated in the past. It is
+# automatically read and these particular cases re-run before any
+# novel cases are generated.
+#
+# It is recommended to check this file in to source control so that
+# everyone who runs the test benefits from these saved cases.
+cc 4ac1e8068352261435a1322d9b0b3b4dfa70bdb058a72339bad0c84fa293f6cd # shrinks to (len, prefix_len) = (1, 0), spare = 2, seed = 0, schedule = [2], cut = None
```

<!-- annotation -->
> **mirror-common-8** (T33), line 0:
>
> Negative control. The seed the differential proptest wrote when it ran against the parent tree, before the clamp: it shrinks to an empty prefix in a buffer with two bytes of spare capacity and a two-byte read schedule, the smallest over-read there is (`read_payload` itself is exact because `Vec::new()` has no capacity; any caller-supplied capacity was enough). Committed as the brief requires; it replays before fresh generation from now on.

<a id="hunk-3"></a>
### src/tree/mirror/framing.rs `@@ -23,6 +23,7 @@`

```diff
@@ -23,6 +23,7 @@
 //! buffering is safe because it outlives a session and rides into the
 //! next one.
 
+use bytes::BufMut;
 use tokio::io::{AsyncRead, AsyncReadExt};
 
 /// The initial reservation granule for framed payload buffers.
```

<!-- annotation -->
> **mirror-common-8** (T33), line 26:
>
> The import the clamp needs: `BufMut::limit` on the `Vec<u8>` borrow. `bytes` is already a direct dependency.

<a id="hunk-4"></a>
### src/tree/mirror/framing.rs `@@ -82,8 +83,8 @@ pub(crate) async fn read_payload<R: AsyncRead + Unpin>(`

```diff
@@ -82,8 +83,8 @@ pub(crate) async fn read_payload<R: AsyncRead + Unpin>(
 }
 
 /// Continue an exact `len`-byte payload read into `payload`, whose
-/// existing bytes — a prefix the caller already consumed from the same
-/// source — count toward `len`.
+/// existing bytes -- a prefix the caller already consumed from the same
+/// source -- count toward `len`.
 ///
 /// The single-buffer continuation for a caller that had to inspect a
 /// payload's leading bytes before deciding to accept the rest (the
```

<!-- annotation -->
> **mirror-common-8** (T33), line 86:
>
> Judgment call. I edited this item's doc for the contract below, and the lane's prose rule says a touched paragraph carries no em-dashes, so the two in the item's first paragraph became spaced double hyphens. T48 sweeps only non-doc comments and leaves rustdoc em-dashes to a pending owner decision; this is crate-private rustdoc on one item whose doc I was already rewriting, so I read the rule at the item, not the paragraph. Reversible in one line if the owner wants rustdoc dashes left for that decision.

<a id="hunk-5"></a>
### src/tree/mirror/framing.rs `@@ -91,18 +92,28 @@ pub(crate) async fn read_payload<R: AsyncRead + Unpin>(`

```diff
@@ -91,18 +92,28 @@ pub(crate) async fn read_payload<R: AsyncRead + Unpin>(
 /// buffer keeps the whole read at one allocation of the payload's bytes,
 /// where a read-then-splice would briefly hold the payload twice. Growth,
 /// exactness, and error behavior are [`read_payload`]'s (it is this
-/// function from an empty buffer).
+/// function from an empty buffer): every read is bounded by the bytes
+/// still owed, whatever spare capacity `payload` carries. A prefix
+/// longer than `len` is a caller error, reported as
+/// [`InvalidData`](std::io::ErrorKind::InvalidData) before any read.
 pub(crate) async fn resume_payload<R: AsyncRead + Unpin>(
     read: &mut R,
     mut payload: Vec<u8>,
     len: usize,
 ) -> std::io::Result<Vec<u8>> {
+    if payload.len() > len {
+        return Err(std::io::Error::new(
+            std::io::ErrorKind::InvalidData,
+            "payload prefix exceeds the declared length",
+        ));
+    }
     while payload.len() < len {
         if payload.len() == payload.capacity() {
             let target = (payload.capacity() * 2).max(PAYLOAD_CHUNK_LEN).min(len);
             payload.reserve_exact(target - payload.len());
         }
-        if read.read_buf(&mut payload).await? == 0 {
+        let owed = len - payload.len();
+        if read.read_buf(&mut (&mut payload).limit(owed)).await? == 0 {
             return Err(std::io::ErrorKind::UnexpectedEof.into());
         }
     }
```

<!-- annotation -->
> **mirror-common-8** (T33), line 95:
>
> The entry found the contract silent on two things: what bounds a read when the caller's buffer has spare capacity, and what happens to a prefix longer than `len`. The doc now states both in the sentence that already delegates to `read_payload`: every read is bounded by the bytes still owed, and the one remaining precondition with its error kind. Three added lines; they buy the two contract clauses the entry showed were missing, and the paragraph still says nothing about the mechanism (`limit`), which is the reader's-of-the-code business.

<!-- annotation -->
> **mirror-common-8** (T33), line 104:
>
> The precondition check, returning `InvalidData` as T33 rules (not the `debug_assert!` the resolution also allowed): a prefix past `len` is a caller bug, and a caller bug at a contract boundary gets an error in every profile. It costs one comparison before the loop. The one production caller cannot trip it (`lone_record_spans` admits a record only when its heads plus content equal `len`, so the prefix of heads is at most `len`); I say that here rather than in the code because the callee's contract, not the caller's discipline, is what the entry is about.

<!-- annotation -->
> **remote-codec-14** (T33), line 115:
>
> The clamp, closing both entries: each `read_buf` goes through `(&mut payload).limit(owed)`, so the chunk offered to the reader is `min(spare capacity, bytes still owed)` and no iteration can fill past `len`. The growth policy above it is unchanged, as the resolution asks; `owed` is named so the loop reads as the bound it is. Rejected alternative: shrinking the buffer to `len` before the loop, which the resolution also offered; it would reallocate a caller's buffer to enforce a bound that a per-read limit enforces for free.

<a id="hunk-6"></a>
### src/tree/mirror/framing/tests.rs `@@ -33,6 +33,11 @@ impl ChunkedRead {`

```diff
@@ -33,6 +33,11 @@ impl ChunkedRead {
             step: 0,
         }
     }
+
+    /// The bytes no read has taken yet.
+    fn unread(&self) -> &[u8] {
+        &self.data[self.at..]
+    }
 }
 
 impl AsyncRead for ChunkedRead {
```

<!-- annotation -->
> **mirror-common-8** (T33), line 36:
>
> An accessor for the bytes the chunked reader has not delivered, so the differential can assert the trailing bytes stay unread on the resumed side too, not only on the plain cursor.

<a id="hunk-7"></a>
### src/tree/mirror/framing/tests.rs `@@ -101,6 +106,59 @@ proptest! {`

```diff
@@ -101,6 +106,59 @@ proptest! {
             }
         }
     }
+
+    /// Resuming a payload read behind a prefix is observably identical to
+    /// reading the whole payload from its start, whatever spare capacity
+    /// the prefix's buffer carries and however the rest is chunked.
+    ///
+    /// The reference, `read_payload`, is `resume_payload` from an empty
+    /// prefix, so the comparison alone would be a self-check; the
+    /// absolute clauses carry the test: full delivery recovers exactly
+    /// the payload and leaves the following bytes unread on both sides,
+    /// and any truncation surfaces as `UnexpectedEof` on both.
+    #[test]
+    fn resumed_read_matches_whole_read(
+        (len, prefix_len) in (0usize..=2 * PAYLOAD_CHUNK_LEN + 130)
+            .prop_flat_map(|len| (Just(len), 0..=len)),
+        spare in 0usize..=PAYLOAD_CHUNK_LEN + 7,
+        seed in any::<u8>(),
+        schedule in prop::collection::vec(1usize..=PAYLOAD_CHUNK_LEN + 7, 1..8),
+        cut in proptest::option::of(0f64..1f64),
+    ) {
+        let payload = pattern(len, seed);
+        let trailing = b"next-frame-bytes";
+        let delivered = match cut {
+            None => len,
+            Some(fraction) => prefix_len + (fraction * (len - prefix_len) as f64) as usize,
+        };
+        let mut transcript = payload[..delivered].to_vec();
+        if delivered == len {
+            transcript.extend_from_slice(trailing);
+        }
+
+        let mut prefix = Vec::with_capacity(prefix_len + spare);
+        prefix.extend_from_slice(&payload[..prefix_len]);
+        let mut chunked = ChunkedRead::new(transcript[prefix_len..].to_vec(), schedule);
+        let via_resume = pollster::block_on(resume_payload(&mut chunked, prefix, len));
+        let mut cursor: &[u8] = &transcript;
+        let via_whole = pollster::block_on(read_payload(&mut cursor, len));
+
+        match (via_resume, via_whole) {
+            (Ok(resumed), Ok(whole)) => {
+                prop_assert_eq!(&resumed, &whole);
+                prop_assert_eq!(resumed, payload);
+                prop_assert_eq!(chunked.unread(), trailing.as_slice());
+                prop_assert_eq!(cursor, trailing.as_slice());
+            }
+            (Err(resumed), Err(whole)) => {
+                prop_assert_eq!(resumed.kind(), std::io::ErrorKind::UnexpectedEof);
+                prop_assert_eq!(whole.kind(), std::io::ErrorKind::UnexpectedEof);
+            }
+            (resumed, whole) => {
+                prop_assert!(false, "readers disagree: {:?} vs {:?}", resumed, whole);
+            }
+        }
+    }
 }
 
 /// Truncation cuts landing one byte short of, exactly on, and one byte
```

<!-- annotation -->
> **mirror-common-8** (T33), line 109:
>
> The differential proptest the resolution asks for. The resumed side reads through the existing `ChunkedRead` (every partial-delivery shape), the reference is `read_payload` over a plain cursor holding the whole transcript, exactly the resolution's `read_payload(prefix ++ rest, len)`. `prefix_len` is drawn from `0..=len` so the precondition always holds, `spare` ranges past a whole chunk so the over-read is reachable at every scale, and the cut lands inside the bytes after the prefix so a truncation is always the transport's, never the prefix's. Trailing bytes are appended only on full delivery, where both sides must leave them unread; on truncation both must say `UnexpectedEof`. Rejected alternative: `whole_read_reference` as the oracle, which the neighbouring proptest uses; the resolution names `read_payload`, and holding the two entry points to each other is the point.

<!-- annotation -->
> **mirror-common-8** (T33), line 112:
>
> Fresh-eyes repair. The doc now says what the differential is: the reference is the same function from an empty prefix, so the comparison alone is a self-check, and the absolute clauses (the payload recovered exactly, the trailing bytes unread on both sides, both truncations `UnexpectedEof`) are what carry the test. Two added lines; they keep a future reader from trusting the comparison arm alone.

<a id="hunk-8"></a>
### src/tree/mirror/framing/tests.rs `@@ -136,3 +194,31 @@ fn payload_read_never_consumes_beyond_the_declared_length() {`

```diff
@@ -136,3 +194,31 @@ fn payload_read_never_consumes_beyond_the_declared_length() {
     assert_eq!(decoded, payload);
     assert_eq!(cursor, trailing.as_slice());
 }
+
+/// A resumed payload read consumes exactly the bytes still owed: spare
+/// capacity on the caller's buffer never draws in the following bytes,
+/// which stay untouched in the transport.
+#[test]
+fn resumed_read_never_consumes_beyond_the_declared_length() {
+    let len = 9;
+    let mut transcript = pattern(len, 5);
+    let trailing = *b"next-frame-bytes";
+    transcript.extend_from_slice(&trailing);
+    let mut prefix = Vec::with_capacity(64);
+    prefix.extend_from_slice(&transcript[..3]);
+
+    let mut cursor: &[u8] = &transcript[3..];
+    let decoded = pollster::block_on(resume_payload(&mut cursor, prefix, len)).unwrap();
+    assert_eq!(decoded, &transcript[..len]);
+    assert_eq!(cursor, trailing.as_slice());
+}
+
+/// A prefix longer than the declared length is a caller error: reported
+/// as `InvalidData`, with nothing read from the transport.
+#[test]
+fn a_prefix_longer_than_the_declared_length_is_invalid_data() {
+    let mut cursor: &[u8] = b"unused";
+    let error = pollster::block_on(resume_payload(&mut cursor, vec![0u8; 12], 9)).unwrap_err();
+    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
+    assert_eq!(cursor, b"unused");
+}
```

<!-- annotation -->
> **mirror-common-8** (T33), line 197:
>
> Negative control. The two witness fixtures from `evidence/witness.md`, in the acceptance's shape: a 3-byte prefix in a capacity-64 buffer, `len` 9, and the 16 trailing bytes `next-frame-bytes`; and a 12-byte prefix against `len` 9. The second asserts more than the witness did: the error kind is `InvalidData` and the cursor is untouched, since T33 fixed the kind. Both failed on the parent tree with the output the commit message records, and pass with the clamp.

<a id="hunk-9"></a>
### src/tree/mirror/streaming/remote/codec/decode/async_io.rs `@@ -142,16 +142,21 @@ async fn read_frame<R: AsyncRead + Unpin>(`

```diff
@@ -142,16 +142,21 @@ async fn read_frame<R: AsyncRead + Unpin>(
     listing: &mut Vec<u8>,
 ) -> Result<Option<WireFrame>, DecodeError> {
     let direction = |kind| DecodeError::direction(speaker, kind);
-    let mut exact = Exact { read };
+    let mut exact = Exact {
+        read,
+        failure: None,
+    };
     // Every frame opens with its array head, its stream item, and its
     // state item, each a one-byte head when canonical, so one read may
     // take all three. A close before the first byte is the clean end of
     // the direction; anything shorter after it is judged in wire order
-    // below, each item taking what the read fetched ahead of it.
+    // below, each item taking what the read fetched ahead of it, and a
+    // failure the read met is reported at the first item that needs
+    // more bytes than arrived.
     let mut opener = [0u8; OPENER_LEN];
-    let arrived = exact.fill(&mut opener).await;
-    if arrived.filled == 0 {
-        return match arrived.failure {
+    let Arrived { filled, failure } = exact.fill(&mut opener).await;
+    if filled == 0 {
+        return match failure {
             None => Ok(None),
             Some(source) => Err(direction(DecodeErrorKind::Read {
                 part: FramePart::FrameHead,
```

<!-- annotation -->
> **remote-codec-11** (T41), line 145:
>
> The pending-failure slot starts empty for every frame; `Exact` is built per frame, so a failure can never carry over to the next frame's opener.

<!-- annotation -->
> **remote-codec-11** (T41), line 153:
>
> The comment that explains the opener read gains the one clause it lacked: where a failure met mid-fill goes. Two added lines; they buy the rule a reader of `read_frame` otherwise has to reconstruct from the slot's doc further down.

<!-- annotation -->
> **remote-codec-11** (T41), line 157:
>
> `Arrived` is destructured so the failure can move into the slot below while `filled` stays a plain count for the head that takes the bytes in hand.

<!-- annotation -->
> **remote-codec-11** (T41), line 155:
>
> Fresh-eyes repair. The clause now says "more bytes than arrived": the opener read may return with a failure and fewer bytes than it asked for, and the earlier wording named the fetch, not the delivery.

<a id="hunk-10"></a>
### src/tree/mirror/streaming/remote/codec/decode/async_io.rs `@@ -159,9 +164,10 @@ async fn read_frame<R: AsyncRead + Unpin>(`

```diff
@@ -159,9 +164,10 @@ async fn read_frame<R: AsyncRead + Unpin>(
             })),
         };
     }
+    exact.failure = failure;
     let (arity, index, state) = {
         let mut head = Pending::new(FramePart::FrameHead);
-        head.take(&opener[..arrived.filled]);
+        head.take(&opener[..filled]);
         let (head, rest) = exact.head(&mut head).await.map_err(direction)?;
         let arity = frame_arity(head).map_err(direction)?;
         let mut stream = Pending::new(FramePart::Signal);
```

<!-- annotation -->
> **remote-codec-11** (T41), line 167:
>
> The recording, placed after the no-bytes return so a failure with nothing in hand keeps its `FrameHead` classification exactly as before; only the partial-fill case changes. This is the line the entry's fixture turns on.

<a id="hunk-11"></a>
### src/tree/mirror/streaming/remote/codec/decode/async_io.rs `@@ -190,7 +196,8 @@ async fn read_frame<R: AsyncRead + Unpin>(`

```diff
@@ -190,7 +196,8 @@ async fn read_frame<R: AsyncRead + Unpin>(
 }
 
 /// What one transport read attempt delivered: the bytes filled before the
-/// buffer was full, the transport closed, or it failed.
+/// buffer was full, the transport closed, or it failed; a read that
+/// replays a recorded failure delivers nothing.
 struct Arrived {
     filled: usize,
     /// The transport's failure, if the read ended in one rather than in a
```

<!-- annotation -->
> **remote-codec-11** (T41), line 199:
>
> Fresh-eyes repair. `Arrived` is also what a read that only replays the recorded failure returns (nothing filled, the failure); its doc gains that one clause so the struct doc covers every value the type carries.

<a id="hunk-12"></a>
### src/tree/mirror/streaming/remote/codec/decode/async_io.rs `@@ -240,13 +247,27 @@ impl Pending {`

```diff
@@ -240,13 +247,27 @@ impl Pending {
 /// guarantees to exist given what has already been parsed.
 struct Exact<'a, R> {
     read: &'a mut R,
+    /// A transport failure met by the opener read, which delivered some
+    /// bytes ahead of it.
+    ///
+    /// The next read reports it instead of touching the transport, so
+    /// the failure lands at the item it interrupted in wire order, not
+    /// at whatever the transport does after failing.
+    failure: Option<std::io::Error>,
 }
 
 impl<'a, R: AsyncRead + Unpin> Exact<'a, R> {
     /// Read into `buf` until it is full, the transport closes, or it
     /// fails, reporting what arrived. The caller judges the bytes in wire
-    /// order.
+    /// order. A failure recorded by an earlier read is reported first,
+    /// with nothing read.
     async fn fill(&mut self, buf: &mut [u8]) -> Arrived {
+        if let Some(source) = self.failure.take() {
+            return Arrived {
+                filled: 0,
+                failure: Some(source),
+            };
+        }
         let mut filled = 0;
         while filled < buf.len() {
             match self.read.read(&mut buf[filled..]).await {
```

<!-- annotation -->
> **remote-codec-11** (T41), line 250:
>
> The slot's doc: what it holds and why it exists (wire order over whatever the transport does after failing). The summary sentence is the what; the mechanism sits below the blank line because `doclint` holds summaries to 220 characters and because the maintainer scanning the struct wants the what first.

<!-- annotation -->
> **remote-codec-11** (T41), line 262:
>
> `fill`'s doc gains the one sentence its new first branch needs.

<!-- annotation -->
> **remote-codec-11** (T41), line 265:
>
> Where the failure surfaces. The resolution's example puts it in `fill_exact`; I put it one level down in `fill`, which every read through `Exact` goes through (the opener's heads, a fresh head, a listing's bulk reads), so the rule is total rather than per entry point. The payload reads bypass `Exact` and read the transport directly, and that is safe: a recorded failure means the opener fill stopped short of three bytes, so an opener head runs out of bytes and surfaces it before any body read can begin. Not a deviation: the resolution says "for example".

<!-- annotation -->
> **remote-codec-11** (T41), line 251:
>
> Fresh-eyes repair. The slot summary names the opener read: it is the only read that records into the slot; a bulk read in general (a listing read) never does.

<a id="hunk-13"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -1,5 +1,9 @@`

```diff
@@ -1,5 +1,9 @@
+use std::pin::Pin;
+use std::task::{Context, Poll};
+
 use crate::message::{PayloadCodec, PayloadDepthLimit};
 use proptest::prelude::*;
+use tokio::io::{AsyncRead, ReadBuf};
 
 use super::*;
 use crate::Version;
```

<!-- annotation -->
> **remote-codec-11** (T41), line 1:
>
> Imports for the two `AsyncRead` fixtures this lane adds (the fail-after transport and the chunked reader); nothing else in the suite implemented the trait before.

<!-- annotation -->
> **remote-codec-11** (T41), line 6:
>
> Same import group: the tokio trait and its buffer type.

<a id="hunk-14"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -795,6 +799,180 @@ fn reader_errors_are_contextual() {`

```diff
@@ -795,6 +799,180 @@ fn reader_errors_are_contextual() {
     }
 }
 
+/// What a `FailAfter` transport does once it has failed: keep failing,
+/// close cleanly, or resume delivering the rest of its bytes.
+#[derive(Debug, Clone, Copy)]
+enum AfterFailure {
+    Fail,
+    Close,
+    Resume,
+}
+
+/// A transport that delivers the first `remaining` bytes of `bytes`,
+/// fails with `Other`, and then does what `after` says.
+///
+/// One fixture serves both decoders: it reads synchronously for the
+/// oracle and asynchronously for `FrameRead`. It counts the reads made
+/// after its failure, so a test can hold a decoder to reporting the
+/// failure without touching the transport again.
+struct FailAfter {
+    bytes: Vec<u8>,
+    position: usize,
+    remaining: usize,
+    after: AfterFailure,
+    failed: bool,
+    reads_after_failure: usize,
+}
+
+impl FailAfter {
+    fn new(bytes: &[u8], remaining: usize, after: AfterFailure) -> Self {
+        Self {
+            bytes: bytes.to_vec(),
+            position: 0,
+            remaining,
+            after,
+            failed: false,
+            reads_after_failure: 0,
+        }
+    }
+
+    /// The bytes one read of up to `want` bytes delivers, or its failure.
+    fn serve(&mut self, want: usize) -> std::io::Result<&[u8]> {
+        if self.failed {
+            self.reads_after_failure += 1;
+            match self.after {
+                AfterFailure::Fail => return Err(std::io::ErrorKind::Other.into()),
+                AfterFailure::Close => return Ok(&[]),
+                AfterFailure::Resume => {}
+            }
+        } else if self.remaining == 0 {
+            self.failed = true;
+            return Err(std::io::ErrorKind::Other.into());
+        }
+        let available = self.bytes.len() - self.position;
+        let served = if self.failed {
+            available
+        } else {
+            self.remaining.min(available)
+        }
+        .min(want);
+        let start = self.position;
+        self.position += served;
+        if !self.failed {
+            self.remaining -= served;
+        }
+        Ok(&self.bytes[start..start + served])
+    }
+
+    /// The bytes no read has taken.
+    fn unread(&self) -> &[u8] {
+        &self.bytes[self.position..]
+    }
+}
+
+impl std::io::Read for FailAfter {
+    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
+        let served = self.serve(out.len())?;
+        out[..served.len()].copy_from_slice(served);
+        Ok(served.len())
+    }
+}
+
+impl AsyncRead for FailAfter {
+    fn poll_read(
+        mut self: Pin<&mut Self>,
+        _cx: &mut Context<'_>,
+        buf: &mut ReadBuf<'_>,
+    ) -> Poll<std::io::Result<()>> {
+        let served = self.serve(buf.remaining())?;
+        buf.put_slice(served);
+        Poll::Ready(Ok(()))
+    }
+}
+
+/// A transport failure inside a frame's opener is reported at the item
+/// it interrupted, identically by both decoders and without another read
+/// of the transport, whatever the transport would do next.
+///
+/// Each row delivers a prefix of an opener and then fails. The canonical
+/// `End(Stream)` opener on stream 9 is three one-byte items: a failure
+/// before the first byte is a read error at the frame head; after one or
+/// two bytes, at the signal; after all three, unseen, and the frame
+/// decodes. Two non-canonical openers fail inside a head's extension
+/// bytes: a stream item `0x18` owed its one-byte extension, and an array
+/// head `0x9b` owed eight. Every row runs against a transport that then
+/// keeps failing, closes, or resumes delivering; in every case no read
+/// follows the failure, so a resuming transport still holds the bytes
+/// it would have delivered.
+#[test]
+fn opener_read_failures_are_reported_in_wire_order() {
+    let stream = stream(9);
+    let canonical = bare_frame(stream, Signal::End(End::Stream));
+    assert_eq!(canonical, [0x82, 0x09, 0x09]);
+    let rows: Vec<(&[u8], usize, Option<FramePart>)> = vec![
+        (&canonical, 0, Some(FramePart::FrameHead)),
+        (&canonical, 1, Some(FramePart::Signal)),
+        (&canonical, 2, Some(FramePart::Signal)),
+        (&canonical, 3, None),
+        (&[0x82, 0x18, 0x09, 0x09], 2, Some(FramePart::Signal)),
+        (
+            &[0x9b, 0, 0, 0, 0, 0, 0, 0, 2, 0x09, 0x09],
+            1,
+            Some(FramePart::FrameHead),
+        ),
+    ];
+    for speaker in SPEAKERS {
+        for after in [
+            AfterFailure::Fail,
+            AfterFailure::Close,
+            AfterFailure::Resume,
+        ] {
+            for &(bytes, remaining, interrupted) in &rows {
+                let case =
+                    format!("{speaker:?}, {bytes:02x?} cut after {remaining}, then {after:?}");
+                let budget = RunBudget::default();
+                let mut sync = FailAfter::new(bytes, remaining, after);
+                let from_sync = decode(speaker, budget, &mut sync);
+                let mut reader =
+                    FrameRead::new(speaker, budget, FailAfter::new(bytes, remaining, after));
+                let from_async = pollster::block_on(reader.frame());
+                let r#async = reader.into_inner();
+                for transport in [&sync, &r#async] {
+                    assert_eq!(
+                        transport.reads_after_failure, 0,
+                        "{case}: the transport was read after it failed"
+                    );
+                    assert_eq!(
+                        transport.unread(),
+                        &bytes[remaining..],
+                        "{case}: the transport does not rest where the failure struck"
+                    );
+                }
+                let Some(interrupted) = interrupted else {
+                    let frame = (stream, Frame::End(End::Stream));
+                    assert_eq!(from_sync.expect(&case), frame, "{case}");
+                    assert_eq!(from_async.expect(&case), Some(frame), "{case}");
+                    continue;
+                };
+                let from_sync = from_sync.expect_err(&case);
+                let from_async = from_async.expect_err(&case);
+                assert_eq!(from_async.origin, from_sync.origin, "{case}");
+                for error in [&from_sync, &from_async] {
+                    assert!(
+                        matches!(
+                            &error.kind,
+                            DecodeErrorKind::Read { part, source }
+                                if *part == interrupted && source.kind() == std::io::ErrorKind::Other
+                        ),
+                        "{case}: {:?}",
+                        error.kind
+                    );
+                }
+            }
+        }
+    }
+}
+
 /// Supply-body truncation cuts at every seeded offset all classify as a
 /// truncated `SupplyRun` with an `UnexpectedEof` source.
 ///
```

<!-- annotation -->
> **remote-codec-11** (T41), line 802:
>
> Negative control. The failing-reader fixture in remote-codec-15's shape (serve `remaining` bytes, fail once with `Other`, then close or keep failing) and the test T41 names. One struct implements both `std::io::Read` and `AsyncRead`, so the oracle and `FrameRead` read the same transport and the atlas's sync-only `FailAfterReader` is not duplicated here; the P4 lane can move this to the atlas offsets rather than replace it. The test covers every opener delivery count (0 to 3 bytes), both then-close and sticky, both speakers: the acceptance's decisive case is one byte then a close, which classified as `Truncated { missing: Signal }` on the parent tree (the commit message records the run) and as `Read { part: Signal }` now; the sticky and full-delivery rows are the unchanged cases the acceptance also names. Kept additive and placed beside the existing `FailingReader` for the harness-crate lane's rebase.

<!-- annotation -->
> **remote-codec-11** (T41), line 810:
>
> Fresh-eyes repair. The fixture gains an after-failure mode (keep failing, close, resume delivering) and a counter of reads made after its failure, so the test holds the decoder to reporting the failure without another read of the transport. The moved-check mutant (the slot consulted after the read loop in `Exact::fill`) passed every earlier row, because a closed transport handed the stored failure back anyway; the commit message records that it now fails on the first sticky row. The resume mode is the third outcome the entry named (a transport that resumes delivering decoded the frame as if nothing had failed), and the unread-bytes assertion pins that those bytes stay in the transport.

<!-- annotation -->
> **remote-codec-11** (T41), line 900:
>
> Fresh-eyes repair. Two rows drive the pending failure through `Exact::head`'s extension leg, which every canonical opener item skips: a stream item `0x18` owed its one-byte extension (both decoders `Read` at `Signal`) and an array head `0x9b` owed eight (both `Read` at `FrameHead`). The rows are table-driven with the canonical opener, so each runs under all three after-failure modes and both speakers.

<a id="hunk-15"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -1058,6 +1236,76 @@ fn overbatched_corners_classify_exactly() {`

```diff
@@ -1058,6 +1236,76 @@ fn overbatched_corners_classify_exactly() {
     }
 }
 
+/// An in-memory `AsyncRead` delivering at most `chunk` bytes per read, so
+/// a body read runs under partial delivery.
+struct ChunkedRead<'a> {
+    bytes: &'a [u8],
+    chunk: usize,
+}
+
+impl AsyncRead for ChunkedRead<'_> {
+    fn poll_read(
+        mut self: Pin<&mut Self>,
+        _cx: &mut Context<'_>,
+        buf: &mut ReadBuf<'_>,
+    ) -> Poll<std::io::Result<()>> {
+        let granted = self.chunk.min(buf.remaining()).min(self.bytes.len());
+        let (now, later) = self.bytes.split_at(granted);
+        buf.put_slice(now);
+        self.bytes = later;
+        Poll::Ready(Ok(()))
+    }
+}
+
+/// The over-budget lone-record body read consumes no byte beyond the
+/// declared run: the frame after it stays intact in the transport and
+/// decodes next, whatever the delivery chunking.
+///
+/// Stream 9, `Supply(End)` declaring a four-byte run that is exactly one
+/// record of one content byte, then `End(Stream)` on the same stream,
+/// under a zero budget so the lone-record path is taken. Both decoders
+/// accept the record; the async reader then decodes the second frame and
+/// reports the clean close after it.
+#[test]
+fn over_budget_lone_record_read_stays_within_its_frame() {
+    let stream = stream(9);
+    let zero = RunBudget::from_bytes(0);
+    let lone = raw_record(&[0x00]);
+    assert_eq!(lone, [0xd8, 0x3f, 0x41, 0x00]);
+    let mut encoded = supply(stream, Flow::End, &lone);
+    assert_eq!(&encoded[..6], [0x83, 0x09, 0x07, 0xd8, 0x3f, 0x44]);
+    let trailing = bare_frame(stream, Signal::End(End::Stream));
+    assert_eq!(trailing, [0x82, 0x09, 0x09]);
+    encoded.extend_from_slice(&trailing);
+    let run = LeafRun::from_encoded(lone).unwrap();
+
+    for speaker in SPEAKERS {
+        let first = (
+            stream,
+            Frame::Reaction(Reaction::Supply(run.clone()), Flow::End),
+        );
+        assert_eq!(
+            decode_both(speaker, zero, &encoded).expect("both decoders accept the lone record"),
+            first
+        );
+        for chunk in 1..=encoded.len() {
+            let read = ChunkedRead {
+                bytes: &encoded,
+                chunk,
+            };
+            let mut reader = FrameRead::new(speaker, zero, read);
+            let mut next = || pollster::block_on(reader.frame()).unwrap();
+            assert_eq!(next(), Some(first.clone()), "chunk {chunk}");
+            assert_eq!(
+                next(),
+                Some((stream, Frame::End(End::Stream))),
+                "chunk {chunk}"
+            );
+            assert_eq!(next(), None, "chunk {chunk}");
+        }
+    }
+}
+
 /// A hand-crafted record whose payload nests one scope past the peer's
 /// depth limit dies typed at wire ingress, while the same shape at
 /// exactly the limit decodes clean, pinning the boundary.
```

<!-- annotation -->
> **remote-codec-14** (T33), line 1239:
>
> Negative control. The witness bytes verbatim (`[83 09 07] [d8 3f 44] [d8 3f 41 00]` then `[82 09 09]`), first through `decode_both` (the two decoders disagreed on the parent tree, async `InvalidRun(NotARecord { remaining: 3 })` against sync `Ok`; the commit message records it), then through `FrameRead` over a chunked reader at every chunk size from one byte to the whole transcript, decoding the second frame and the clean close after it: chunk sizes from two up reproduced the over-read on the parent tree, the whole-slice size being the witness's own shape. The chunked reader is a fresh minimal fixture rather than a reuse of the framing suite's `ChunkedRead`, which is private to that module; the harness-crate lane may fold the two. No shared fragment of the suite is touched, per the ordering note against `p1-harness-crate`.

