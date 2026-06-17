# Transcript: hash-width reduction analysis

*A conversation with Claude Code (Fable 5), 2026-06-11. Topic: whether the
32-byte hashes in the `rumors` Merkle radix trie — the content hash that keys
each `(Version, T)` leaf, and the Merkle branch hash used for subtree
comparison in the mirror protocol — can be narrowed to save bandwidth and
storage.*

*Outcome: keep the content hash at 32 bytes (it carries all content
integrity, and attacker-influenced value bytes can't be ruled out a priori);
truncate the Merkle hash to 16 bytes (it guards a self-healing liveness
optimization, not a corruption boundary), which roughly halves the protocol's
dominant hash traffic. Halving the trie height was examined and rejected as
redundant-to-regressive once stacked on the Merkle-width halving.*

---

## Finch

Right now this Merkle tree uses full 32-byte hashes; however, I'm wondering
if we can still avoid collisions in *this* use-case with a reduced hash
width. Can you analyze and determine if we can save bandwidth and storage by
reducing one or both of the two kinds of hashes herein: (1) the content hash
used to determine the Key for a given (Version, T) entry in the tree; (2)
the Merkle hash used to determine whether a given subtree has changed
between two peers?

## Claude

# Can the hash widths shrink?

**Yes — both can, to 16 bytes each, and the win is real: it roughly halves
the protocol's dominant bandwidth cost (the "distinguishing hash traffic"
your own crate docs flag at 10–25 KB per differing message), at no cost in
round trips. The deciding question is one fact about deployments, not about
the code: can message *values* be influenced by an attacker who can't
otherwise join the universe?** If values are purely control-plane-generated,
16/16 is comfortably sound. If arbitrary tenant bytes can flow into values,
keep the content hash at 32 and truncate only the Merkle hash. I'd stop at
16 — going to 8 bytes is defensible only for the Merkle hash and only under
the strictest reading of the trust model, and the savings beyond 16 are
marginal.

### The two hashes carry very different loads

**The content hash carries everything.** This is the structural fact the
whole analysis pivots on: the leaf's Merkle hash is a *constant* —
`Hash::leaf()` commits to nothing but a domain tag
(`src/tree/typed/hash.rs:25-29`). Content integrity rides entirely on the
path being the content hash. So a content-hash collision is not "two keys
alias," it is **undetectable split-brain**: if peer A holds `(v₁, m₁)` and
peer B holds `(v₂, m₂)` at the same path, every Merkle hash from leaf to
root agrees, mirror prunes the subtree as "equal," and the two replicas
disagree forever while the system believes itself convergent. `join`
codifies the assumption explicitly: "identical paths carry identical
contents. Keep one." (`src/tree/traverse/join.rs:152-156`). A collision also
corrupts redaction: `redact(k)` kills whichever message holds the path.

**The Merkle hash is only a comparison signal.** It is never identity, never
persisted as meaning, and a wrong-equal is *self-healing under churn*: the
two subtrees stay unreconciled only until any insert or redaction lands in
either copy, which perturbs both hashes and forces a re-compare. The failure
is delayed propagation in a frozen subtree — a liveness wound, not a
corruption.

This asymmetry means the two widths are independently tunable (nothing
couples them: branch preimages are `(radix, hash)` records; path width only
fixes trie depth), and the Merkle hash tolerates far more reduction.

### The threat model does most of the work

The crate docs are explicit: peers in a universe trust one another; the
protocol "is not Byzantine-tolerant — a compromised member can fabricate and
redact at will" (`src/lib.rs:42-47`). And consistently with that, the mirror
receiver inserts provided subtrees at their named prefix **without
re-hashing leaves** (`src/tree/traverse/mirror/message.rs:52-58`) — a
malicious *peer* can already place arbitrary content at arbitrary paths for
free. So collision-grinding by a peer buys nothing a peer doesn't already
have; wide hashes were never what stood between you and a Byzantine member.

What the hash width *does* defend against is the intermediate adversary:
**an attacker who can't join the universe but can influence the bytes that
honest peers insert** (versions are minted honestly; values may not be). For
that adversary, with `w`-bit content hashes:

| Attack | Work | Impact |
|---|---|---|
| Birthday self-collision: submit crafted `v₁` to peer A and `v₂` to peer B | ~2^(w/2) | attacker-controlled permanent split-brain at one key |
| Multi-target preimage against the N live keys | ~2^w / N | suppress/replace a *victim's* message on some peers |
| Same attacks vs. the Merkle hash (shaping subtree child-sets via ground leaf paths) | ~2^(w/2) | *transient* split-brain, heals on organic churn in that prefix |

And for purely honest content, accidental collision over a universe lifetime
is birthday over N messages: `P ≈ N²/2^(w+1)`.

| Content width | Accidental (N = 10⁹ lifetime) | Crafted split-brain | Victim suppression (N = 2³⁰) |
|---|---|---|---|
| 32 B (256-bit) | ~2⁻¹⁶⁹ | 2¹²⁸ | 2²²⁶ |
| **16 B (128-bit)** | **~2⁻⁶⁹** | **2⁶⁴** | **2⁹⁸** |
| 12 B (96-bit) | ~2⁻³⁷ | 2⁴⁸ ⚠️ GPU-weekend | 2⁶⁶ |
| 8 B (64-bit) | ~10⁻⁴ ⚠️ | 2³² ✗ | 2³⁴ ✗ |

16 bytes is the clean line: accidental risk is beyond negligible at any
scale the "whole set in memory on every peer" constraint permits,
victim-targeting stays out of reach, and the only thing 32 bytes buys beyond
it is pushing the *attacker-content self-collision* from 2⁶⁴
(nation-state-expensive) to 2¹²⁸. Whether that matters is exactly the "are
values attacker-influenced" question. 12 bytes is where it genuinely starts
to break.

For the Merkle hash the same table is more forgiving: accidental wrong-equal
is per-comparison 2^(−w) (pairwise, not birthday — a fleet doing ~2³⁸
differing-subtree comparisons per year is at ~2⁻²⁶/year even at 8 bytes),
the adversarial primitive is transient rather than permanent, and a
peer-grade adversary doesn't need it. 16 bytes keeps crafted collisions at
2⁶⁴ symmetry with the content hash; 8 bytes is defensible only if
attacker-influenced values are firmly out of scope.

### What you actually save

**Bandwidth — the real win, and it's on the protocol's sore spot.** The
crate root already concedes that "at small divergences, each differing
message drags roughly 10–25 KB of distinguishing hash traffic — for payloads
under ~20 KB the hashes, not your data, are most of the bill"
(`src/lib.rs:48-55`). That traffic is almost entirely Merkle hashes: the
`uncertain` channel is `(Prefix, Hash)` pairs where the prefix is 1–3 bytes
against a 32-byte hash, and the responder's `Opening` alone is a saturated
256 × 33 ≈ 8.4 KB every session. At 16-byte Merkle hashes the Opening drops
to ~4.4 KB and the whole hash tax roughly halves: **the "payloads under
~20 KB" crossover drops to ~10 KB**, directly widening the envelope in the
*should-you-not-use-it* section. Content-hash reduction adds a smaller term:
shipped leaves carry their full path on the wire (channel prefix +
path-compression bytes), so 16 bytes saved per leaf in `providing`.

Critically, this is *free along the design's scarce axis*. The 256-ary tilt
spends bandwidth to buy round trips; truncation spends neither — round count
is governed by the disjoint frontier depth (`log₂₅₆` of the population, 2–3
levels in practice per `results/ANALYSIS.md` §2b), which doesn't change.
Unlike narrowing the fan-out, this touches no latency.

**Storage — modest.** Per leaf: ~16 B of path bytes plus a per-leaf share of
branch memos (branches ≈ 0.3 N at 10⁵ leaves), call it ~20 B against the
~150 B/leaf resident estimate from the cost-model work — roughly 10–15%.
Real but not the headline.

**CPU — small bonus.** Branch preimages shrink from 33 to 17 B per child
record, so the rehash-up-the-path term (`b_d` in the fitted model) and memo
recomputation get cheaper; BLAKE3 itself still runs at full width and
truncates, so no security-relevant change to the primitive (BLAKE3 is an
XOF; prefix truncation is the sanctioned construction, security =
min(w/2, 128) collision).

**Schedule — a nicety.** Trie depth 16 halves the type-level `Height` chain
and the hardcoded 14/15-level exchange schedule in `protocol.rs`. Typical
sessions never see those levels, but the types get half as tall.

### What it costs

- **Full protocol break**: wire snapshots re-accepted deliberately, and the
  preamble's `proto_version` gate (already validated before any frame is
  trusted, per `remote.rs`) is the right mechanism — old and new peers
  refuse each other cleanly rather than corrupting.
- **`Key` API break** — with one redeeming property worth preserving
  deliberately: if the new path is defined as the *prefix* of the same
  `for_leaf` output, then every 32-byte key users have durably persisted for
  later redaction maps onto its 16-byte successor by truncation. That
  migration story is worth a doctest if you do this.
- Mechanical churn in `Height`/`Prefix`/`Path` (the `32 − HEIGHT` indexing,
  `Prefix<H>` byte counts) and the `define_peer!` schedule.
- The `for_leaf` interior (depth-1 Merkle of `H(version) ‖ H(value)`, the
  length-malleability guard) is untouched — only the final output truncates.

### Recommendation

Truncate **both to 16 bytes**, unless deployments can carry attacker-chosen
value bytes, in which case truncate **only the Merkle hash to 16** and leave
the content hash at 32 (you still capture most of the bandwidth win, since
the hash tax is Merkle-dominated, and you give up the storage/depth
savings). I would not take either below 16: the incremental savings from
16→8 are ~25% of the hash traffic for a qualitative drop in adversarial
margin (2⁶⁴ → 2³²), and the content hash at 8 fails even accidentally.

Residual items I did not verify: I took the 14/15-level schedule and
~150 B/leaf figures from `results/ANALYSIS.md` rather than re-deriving them,
and I have not audited whether any code path beyond `join`/mirror placement
assumes path-equality implies content-equality (e.g., persistence layers
above this crate). If you go ahead, the doc updates write themselves: the
`lib.rs` bandwidth caveat's 10–25 KB and ~20 KB figures, and the `tree.rs`
"fixed depth 32" shape section.

## Finch

I think it is *possible* that deployments could contain attacker-chosen
value bytes; at least, we should not a priori rule that out, and it's a
subtle footgun. You could prevent it by inserting a random nonce into every
value, but that loses you exactly what you'd gain, so washes out. So I think
it's still smart to keep the 32-byte content hash. Can you give more
justification for your numbers about the infeasibility of a Merkle
collision at 16 bytes?

## Claude

Good call on the content hash — the nonce trick does wash out, and "subtle
footgun" is exactly right: the cost of being wrong is *silent permanent*
divergence, so you should pay the 32 bytes to never have to reason about it.
Here's the rigorous version for why 16 bytes is nonetheless fine for the
Merkle hash specifically.

### First, what a Merkle collision actually costs — because it sets the bar

This is the asymmetry that justifies treating the two hashes differently. A
content-hash collision is permanent and silent: two distinct leaves land at
one path, `join` keeps one (`src/tree/traverse/join.rs:156`), the other is
dropped forever, and because the path is stable nothing ever perturbs it
back into view. No amount of churn heals it.

A Merkle collision is neither permanent nor silent-forever. The hash is only
ever consumed as an **equality test between two peers' subtrees at the same
prefix** (the `uncertain` channel). A false-equal means two *differing*
subtrees get pruned as equal, so a divergent corner goes unreconciled. But
the branch hash is a pure function of its children's hashes, so **the next
mutation whose path descends through that prefix changes the hash and forces
a re-compare.** No data is dropped — both peers retain everything — they
simply haven't shared one subtree yet. The failure mode is delayed
propagation that heals on churn, not corruption. So the collision-resistance
bar for the Merkle hash is genuinely lower than for the content hash, and
that is *why* the same width that's too risky for one is comfortable for the
other.

### The truncation is sound at face value

BLAKE3 is an extendable-output function (a PRF/XOF), not a Merkle–Damgård
construction, so taking the leading 128 bits is the sanctioned way to get a
128-bit hash: collision resistance 2^64, preimage 2^128, with no
length-extension caveat. The existing domain separation survives truncation
unchanged — `LEAF_TAG`/`BRANCH_TAG` keep leaf and branch preimages disjoint
(`src/tree/typed/hash.rs:29-33`), and the radix-interleaved child records
keep distinct child-sets distinct. Nothing about narrowing the output
weakens those structural guarantees; it only lowers the brute-force ceiling
from 2^128/2^256 to 2^64/2^128.

So the whole question reduces to: **is 2^64 collision / 2^128 preimage
enough, given the failure mode above?** Two regimes.

### Accidental collision

The instinct is "birthday problem, 2^64, scary." But the birthday bound is
the wrong model here, because a false-equal isn't "any two of N hashes
coincide" — it's "*this specific pair* coincides," and the pairs are fixed
by the prefix structure. The hash at prefix P on peer A is only ever
compared against the hash at prefix P on peer B. So the triggerable event is
a **per-comparison Bernoulli trial at 2^−128**, accumulated over every
comparison where the two subtrees actually differ in content (matching
subtrees can't false-positive — they're genuinely equal).

Count the triggerable trials, paranoid fleet:

- Divergent-content comparisons per maximally-divergent session ≈ `D·L` ≤
  10⁵ × 3 ≈ 2¹⁸.
- Sessions over a decade of 100 peers each initiating every second ≈ 100 ×
  3×10⁸ ≈ 2³⁵.
- Total triggerable trials ≈ 2⁵³, so cumulative false-equal probability ≈
  2⁵³ × 2⁻¹²⁸ ≈ **2⁻⁷⁵**.

The cumulative figure is dominated by the 2⁻¹²⁸ per-trial term, which makes
it robust to how wrong my activity estimate is. The clean environment test:
**to reach even a one-in-a-million lifetime probability (2⁻²⁰) you'd need
2¹⁰⁸ triggerable divergent comparisons; at a fleet-wide million comparisons
per second that's ~10¹⁹ years** — a billion times the age of the universe.
You will hit an undetected ECC failure or a cosmic-ray bit flip in the hash
register vastly sooner.

For completeness, the *loose* birthday upper bound — which over-counts by
including collisions that can never be triggered — is dominated by the root
subtree, whose content turns over on every mutation anywhere in the tree. At
~2³⁸ lifetime mutations that's (2³⁸)²/2¹²⁹ ≈ **2⁻⁵³**. This counts a
root-content from year 1 colliding with one from year 9 as a "collision"
even though no two peers ever hold those two simultaneously to compare them,
so it's a true but slack ceiling. Even this slack ceiling is
one-in-a-quadrillion over the fleet's whole life, against a heal-on-churn
nuisance.

### Adversarial collision

Split by who the adversary is.

**A member.** Per the trust model (`src/lib.rs:42-47`), a compromised member
fabricates and redacts at will, and the mirror receiver inserts provided
subtrees at their named prefix **without re-hashing**
(`src/tree/traverse/mirror/message.rs:52-58`). A member can already place
arbitrary content at arbitrary prefixes and desync peers directly. A Merkle
collision buys this adversary *nothing it doesn't already have*, at any
width. The Merkle hash is simply not a defense boundary against members —
which is the deeper reason its collision resistance isn't load-bearing.

**A non-member who influences value bytes** — the footgun flagged for the
content hash. Here's why it threatens the content hash but not
(meaningfully) the 16-byte Merkle hash:

- The content hash stays at 32 bytes, so **leaf placement stays at
  128-bit-collision strength.** The attacker cannot cheaply choose where a
  crafted leaf lands.
- A branch hash is `H(BRANCH_TAG ‖ r₀ ‖ h₀ ‖ …)` where the child hashes
  `h_i` are themselves subtree hashes determined by *what honest peers
  actually hold*. The attacker can't enumerate branch-hash preimages offline
  the way they can enumerate content-hash preimages — each trial requires
  crafting a value, getting it inserted onto honest peers, and inheriting
  whatever child hashes the real tree produces. So the nominal 2^64
  collision search is a *lower bound assuming free grinding*; the realized
  cost is far higher and rate-limited by the honest insertion path.
- And the payoff is transient, prefix-local desync that heals on the next
  mutation.

Contrast that with the content hash, where holding at 32 was right: there
the grind is **offline** (just compute hashes, no insertion until the final
colliding pair), and the payoff is **permanent silent corruption.**
Cheap-and-permanent versus expensive-and-self-healing is the entire reason
the split decision is sound.

To be honest about the one number that deserves it: **2^64 collision work is
not "infeasible" in the abstract** — it's the SHA-1-collision ballpark
(~2^63), which Google executed once at industrial cost. If the Merkle hash
were a load-bearing security boundary that could not be waved away. It
isn't, for three compounding reasons: the strongest adversary in the model
(a member) gets the same effect for free, so spending 2^64 buys a
*weaker-than-member* capability; the non-member must *induce and insert*
each trial rather than grind offline; and the result self-heals. A
capability that's strictly dominated, far more expensive than advertised,
and transient is not worth 2^64 to anyone. The multi-target variant
(preimage against any of N live subtree hashes, ~2^128/N ≈ 2^98 at a billion
subtrees) is even less economical and additionally requires knowing victim
subtree contents to define the target.

### Residual risks, stated plainly

- **Cold-prefix heal time.** "Heals on churn" means heal time = time to the
  next mutation beneath that prefix. A genuinely cold corner that never
  mutates again would stay unreconciled indefinitely — but it's also, by
  definition, a corner nothing touches, so the divergence is unobservable
  until something does touch it, at which point it heals. No data is ever
  lost; this is the worst case and it's still strictly weaker than a content
  collision.
- **The 2⁻⁷⁵ vs 2⁻⁵³ gap** is the tight-triggerable estimate vs the slack
  birthday ceiling. The structure is solid (per-comparison, not
  birthday-amplified, because pairing is prefix-fixed); the truth sits near
  2⁻⁷⁵ and the 2⁻⁵³ is a ceiling the failure can't actually reach.
- The `D·L` comparisons-per-session and the `L ≈ 2–3` depth come from the
  protocol shape and `results/ANALYSIS.md` §2b rather than an instrumented
  session counter; a durable artifact would want the per-session comparison
  count confirmed empirically before enshrining the 2⁻⁷⁵.

**Bottom line:** keep the content hash at 32 (correct, for the reason
given), truncate the Merkle hash to 16. Its collision resistance drops to
2^64, but it guards a self-healing liveness optimization rather than a
corruption boundary, the accidental rate is ~2⁻⁷⁵ over a paranoid fleet
lifetime, and the only adversary who'd benefit from grinding it can already
do worse for free. That's the half-the-hash-traffic win banked against the
hash that can afford to lose the bits.
