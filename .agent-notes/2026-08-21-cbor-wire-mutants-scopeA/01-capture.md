# capture.rs: the 34 reflection-renderer survivors

`src/tree/mirror/streaming/remote/codec/capture.rs` is the CBOR reflection
renderer behind the byte-exact session snapshots: the snapshot harness
(`tests/common/gossip_snapshot.rs`) records every wire byte of a captured
session through the public observation hook, asserts the observed items
account for the transport bytes exactly (`assert_items_account_for`), and
renders each item as a fully unfolded diagnostic value tree. The module's
load-bearing claim is *injectivity on wire bytes*: under the wire's
deterministic-encoding contract, a complete rendering has exactly one
preimage, and wherever the walk cannot vouch for that (non-canonical heads,
invalid UTF-8, depth past the budget) it falls back to an explicit failure
line above exact hex. That claim is what licenses "a rendering with no
hexdump is still a byte pin." Every mutant in this cluster is therefore a
*diagnostics-integrity* gap, not a shipped-protocol behavior bug — but the
integrity of the wire pin is exactly what the snapshot hard rule leans on,
so the gaps are worth closing.

The single structural reason the cluster survived: **the renderer is total
over CBOR, while everything that exercises it speaks only the protocol's
dialect.** The committed fixtures (session snapshots, and the point tests
in `capture/tests.rs`) drive unsigned ints, byte strings, text, arrays,
maps with uint keys plus the one `"listing"` text key, and the named tags.
No fixture renders a float (any width), a simple value, a negative
integer, an unnamed tag, or a container nested under a plain map entry or
array element. The two negative-path tests assert the reason-independent
substring `"!! not rendered as CBOR"`, so mutants that only *change the
failure reason* pass them. I verified this by reading
`capture/tests.rs` in full and the snapshot helper's construction; the
survival of the underflow-panicking `depth + 1 -> depth - 1` legs is
itself the proof that no test renders a container through those arms
(any such rendering at depth 0 panics in the dev/test profile).

Dispositions below follow the roster ladder. Net: 1 refactor, 1 roster
entry (a bit-disjointness equivalence identical in shape to the
already-rostered `write_head` legs), and 32 mutants killed by five
property families, all of them stateable as general invariants rather
than point pins.

---

## Family 1: the major-7 grammar has no oracle (15 mutants, plus 1 roster entry)

Covers, in `parse_major_seven` and its routing in `parse_node`:

- `capture.rs:349:16 replace >> with <<` (major-7 route test in `parse_node`)
- `capture.rs:393:24 replace & with |`, `& with ^` (info mask)
- `capture.rs:395:9 delete match arm 0..=23` (immediate simples)
- `capture.rs:399:9 delete match arm 24` (extended simples)
- `capture.rs:409:9 delete match arm 25..=27` (floats)
- `capture.rs:422:9 delete match arm 28..=30` (reserved)
- `capture.rs:403:22 replace < with <=` (extended-simple canonical floor)
- `capture.rs:410:32 replace << with >>`, `410:41 replace - with +`, `- with /` (float width arithmetic)
- `capture.rs:411:27 replace < with ==`, `< with >`, `< with <=` (float truncation check)
- `capture.rs:417:29 replace << with >>`, `417:34 replace | with &` (float bit assembly)

**Reachable in principle?** Yes, without any exotic state: supply-run
records carry application payloads, which are arbitrary application CBOR,
and the renderer unfolds them. A payload containing `true`, `null`, or a
float exercises every one of these lines through the harness-reachable
frame path. The fixtures just never contain one.

**What survival means.** Several of these are loud in the right hands:
under `349 >> -> <<`, a rendered `true` reaches `parse_node`'s
`unreachable!` and panics the snapshot harness; under `411 < -> ==`, an
exactly-fitting float panics in `split_at` instead of erring gracefully.
Others are quiet corruption of the pin: `410 << -> >>` computes float
width 0, renders `Float(info, 0)`, and then re-parses the float's payload
bytes as *following items* — the rendering silently misattributes bytes,
which is precisely the injectivity failure the module exists to prevent.

**Why the suites missed them.** The only major-7 bytes any test feeds
the renderer are non-canonical garbage (`0xf8 0x05`), and the assertion
checks the fallback marker, not the reason — so mutants that reroute
Reserved to Indefinite, or a graceful error to a different graceful
error, pass.

**Killing test, most general form — an exhaustive byte-space spec,**
mirroring `ingress_paths_agree_on_every_initial_byte` in
`cbor/tests.rs`: for every initial byte `0xe0..=0xff` and a small
deliberate matrix of extension payloads (empty; too short by one;
exactly fitting; over-long by one; extended-simple values 0, 23, 31,
32, 33, 255; float bits 0, a low-byte-only pattern, a high-byte-only
pattern, all-ones — the boundary set that separates every arm and both
shift directions), assert `parse_major_seven`'s verdict against a
ten-line executable spec written directly from RFC 8949's
additional-information table: the exact `Node` (variant and value/bits,
where bits are `u16/u32/u64::from_be_bytes` of the payload — an
independent computation, not the code's shift loop) or the exact reason
string. Exhaustive over the initial byte, so arm deletions, mask legs,
the route test, the canonical floor's boundary, and the width and
truncation arithmetic all discriminate; exact verdicts, so
reason-rerouting survivors die too.

**Second, complementary family — a `parse_node` <-> ciborium differential
oracle** (this also carries Family 2 and feeds Families 3 and 4): generate
arbitrary `ciborium::Value` trees — integers of both signs, floats
biased to include f16- and f32-representable values (ciborium encodes
shortest lossless width, so the generator reaches all three widths),
bools, null, byte/text strings, arrays, maps, tags both named and
unnamed — serialize with ciborium (the workspace's pinned payload codec,
already the admission oracle elsewhere), and assert `parse_node` returns
the structural image of the `Value` under a small test-side
`Value -> Node` conversion, consuming the input exactly. This is the
same shape as the handshake decoders' field-by-field oracle families:
the decoder is held to an independent encoder over the whole value
space, not to fixtures.

**Roster entry (rung 3), one leg:** `capture.rs:417:34 replace | with ^`.
In `bits = bits << 8 | u64::from(byte)`, the left operand's low eight
bits are zero and the right operand occupies only those eight bits, so
the operands are bit-disjoint and `|` equals `^` for all inputs — the
identical argument, nearly word for word, as the roster's existing
`write_head`/`push_head` `| -> ^` entries. The `& ` leg faces the family
(it zeroes the accumulator and any nonzero float bits discriminate).

## Family 2: absent vocabulary in parse_node (1 mutant)

- `capture.rs:355:9 delete match arm 1` (negative integers)

Deleting the major-1 arm sends every negative integer to
`unreachable!` — a panic on a legal application payload. Reachable
through any capture whose payload holds a negative integer; no fixture
does. Killed by the ciborium differential family above (its generator
includes negative integers), and cheaply worth one deterministic witness:
a supply run whose payload is `-1` renders as `-1`. No refactor applies:
the arm is real vocabulary, just untested.

## Family 3: depth-budget bookkeeping (9 mutants)

- `capture.rs:375:51`, `376:53 replace + with *` (map key/value re-parse depth)
- `capture.rs:383:46 replace + with *` (tag content re-parse depth)
- `capture.rs:469:73 replace + with -`, `+ with *` (map-value render descent)
- `capture.rs:479:65 replace + with -`, `+ with *` (array-item render descent)
- `capture.rs:532:70 replace + with -`, `+ with *` (listing container-value descent)

These are all `depth + 1` in the walk's one shared nesting budget. The
`-` legs panic by usize underflow at depth 0 the moment *any* container
renders through that arm — their survival proves the arms are unexercised
(no plain map with a container value, no array element that is itself a
container, no listing whose value is a container ever renders in any
test). The `*` legs stall the budget (`0 * 1 = 0`), so nesting chains
threaded through that arm draw down the shared `MAX_DEPTH` budget slower
than the invariant states; the committed depth tests
(`deep_embedded_chain_falls_back_instead_of_recursing`,
`deep_payload_through_the_frame_path_falls_back`) drive chains built
from tag-24 wrappers only, which is why the map/array/listing legs
survive. Nothing here is roster material: the rustdoc on `render_node`
states the conservation invariant these mutants each break.

**Killing test, most general form — a boundary-exact fallback family per
recursion arm.** One generator builds a nesting chain of length `n`
whose links are drawn from the arm set {array element, map value under a
plain key, map value under `"listing"`... container, tag content,
embedded-byte-string unfold}, mixed arbitrarily (proptest chooses the
link sequence), terminating in a scalar. The invariant, shape over
point: rendering the chain with `n = MAX_DEPTH - 1` links produces no
fallback line, and with `n = MAX_DEPTH` links produces exactly the
too-deep fallback with the convicted bytes standing below it. Because
the link sequence is arbitrary, every `depth + 1` site sits on some
generated chain's budget path, and any stalled (`*`) or reversed (`-`)
leg moves the observed threshold or panics. This generalizes the two
committed chain tests (keep those as deterministic witnesses) from
one link type to the whole arm alphabet.

## Family 4: annotation placement (5 mutants)

- `capture.rs:460:41 replace match guard text == "listing" with true` (render_node)
- `capture.rs:580:9 delete match arm (CLOCK_TAG, Node::Bytes)` (render_tag)
- `capture.rs:589:32 replace match guard scalar(..).is_some() with true`, `with false` (render_tag)

The rumors naming layer — `/ listing /`, `/ clock /`, inline vs block
tag renderings — is what makes a re-accept diff speak the protocol's
vocabulary. `460 -> true` annotates every text-keyed map value as a
listing; `580`'s deletion drops the `/ clock /` comment (clock-tagged
atoms then render through the generic scalar-tag arm — same bytes, no
meaning); `589 -> true` panics on an unnamed tag over a container
(`expect("checked by the guard")`); `589 -> false` reroutes unnamed
scalar tags to block form. All reachable: clock atoms are real protocol
vocabulary (their absence from any rendered fixture is the gap — worth
checking whether any captured session should be carrying them through a
bookmark-bearing scenario), and unnamed tags arrive in application
payloads.

**Killing test, most general form — an annotation-placement invariant
over the differential family's generated values:** for an arbitrary
generated value rendered under `Naming::Plain`, (a) the string
`/ listing:` appears on exactly the rendered lines of map values whose
key is the text `"listing"` (count them in the test-side `Value` walk,
count them in the rendering); (b) each named tag (version, party,
clock) renders with its annotation exactly once per occurrence; (c) an
unnamed tag renders inline (`n(scalar)`) iff its content is a scalar,
and in block form otherwise. The oracle counts come from the
independent `Value` walk, so the guards' both directions discriminate.
The `589 -> true` leg additionally dies by panic the first time the
generator emits an unnamed tag over a container.

## Family 5: the capture-integrity detector must fire (1 mutant)

- `capture.rs:288:21 replace match guard rest.is_empty() with true` (render_item)

The guard is a conformance bug detector on the capture harness itself
(one CBOR item per hook buffer); mutated to `true`, a control item with
trailing bytes renders its parsed prefix and *silently drops the tail* —
the one survivor in this cluster that directly voids injectivity rather
than degrading annotations. Only a broken harness can produce the input
(honest hook delivery is one item per buffer), which is exactly why the
detector panics rather than erring — and why the mutant needs a test
that the detector fires. **Killing test:** a `should_panic` property —
for an arbitrary generated canonical item and an arbitrary nonempty
tail, `render_item(item ++ tail, ..)` panics with the trailing-bytes
message. (Its sibling panic, the non-canonical control item one line
below, is already exercised via `control_item_name`'s panics; if the
verify pass disagrees, the same family extends to it.)

## Refactor (1 mutant): the vacuous short-listing clause

- `capture.rs:501:26 replace < with >` (render_listing)

In `entries.windows(2).all(..) || entries.len() < 2`, the second clause
is structurally redundant: `windows(2)` on a slice shorter than two
yields no windows, and `all` over an empty iterator is already `true`.
Every mutation of the dead clause is equivalent — the survivor is the
tell. **Disposition: rung 1, delete the clause** (and the mutant with
it). The neighboring `capture.rs:498:49 replace < with <=` is *not*
dead: it admits duplicate adjacent radixes as "ascending". **Killing
test, general form:** for an arbitrary sequence of u8 radixes (generator
biased to include sorted, duplicated-adjacent, and descending cases),
render the listing map and assert the `NON-CANONICAL ORDER` verdict
appears iff the test-side check `radixes.windows(2).all(a < b)` fails —
the strictness boundary (duplicates) then discriminates `<=`.

---

## Summary table

| Mutants | Disposition |
|---|---|
| 15 major-7 grammar legs | Family 1: exhaustive byte-space spec + ciborium differential oracle |
| `417:34 \| -> ^` | Roster: bit-disjoint accumulator, same proof as `write_head` |
| `355:9` nint arm | Family 2: ciborium differential (+ one witness) |
| 9 `depth + 1` legs | Family 3: boundary-exact fallback over the recursion-arm alphabet |
| `460`, `580`, `589` ×2 | Family 4: annotation-placement counts vs an independent value walk |
| `288:21` trailing-bytes guard | Family 5: should_panic property (detector fires) |
| `501:26` | Refactor: delete the vacuous `len() < 2` clause |
| `498:49` | Family 6 (order verdict vs independent strictness check) |

The ciborium differential oracle (Family 1's second half) is the
workhorse: it alone kills the nint arm, most major-7 legs, and feeds
Families 3 and 4 with its generator; the exhaustive byte-space spec
pins the grammar's boundaries the way the ingress-agreement test
already pins `read_head`'s. Nothing in this cluster is geometry-shadowed
(no mutant's trigger involves tree depth or hash collisions — all inputs
are directly constructible bytes), and nothing rests on hostile-peer
reachability: every discriminating input arrives through our own
harness's captures or legal application payloads.
