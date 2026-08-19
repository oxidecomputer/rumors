# B0.5 — the uniformity envelope: how loose is the landed window charge?

> **Context on this branch (2026-07-22).** This is an analysis artifact
> from the single-socket campaign, which was declined in favor of the
> `Link` transport this branch ships. Its durable content is
> transport-independent: the uniform-occupancy population and reply-size
> analysis (§2–§6) and the **L(N)** simultaneously-heavy-stage divisor
> (§10), which apply here as the principled basis for dividing a session
> memory budget across the seventeen per-stream transport receive
> windows — a future design task on this branch. Everything the note
> calls "landed" — the `charged(K, N)`/`K_flat` byte-window solve in
> `window.rs`, `design/byte-window-plan.md`, `design/single-socket.md` —
> lives on the campaign branch, not here: this branch's `window.rs`
> prices a node-denominated budget (`Peer::max_in_flight_nodes`), and
> no charge formula below is this branch's mechanism. The K_flat/K_sharp
> tables stand as analysis; the "adopt"/"adoptable form" verdict was an
> instruction to the campaign, and adoption was declined with it.

Spike note per `design/byte-window-plan.md` §B0.5 (2026-07-22). Gates
nothing; touches no production code. The companion script
`design/b05-envelope-sim.py` (stdlib + numpy, seeds 0..99 recorded in
the file) reproduces every number in this note; run it with
`python3 design/b05-envelope-sim.py`.

**Model of record** (byte-window-plan §0): uniform-hash,
authenticated-honest-peer. Content addresses are uniform 32-byte
hashes; hostile-peer regimes are off-model; no pricing argument below
rests on adversary economics. Every claim is tagged \[derived\] (with
premises), \[measured\] (with the run that produced it), or
\[estimated\].

**Verdict up front: adopt per-stage pricing, in the integer form.**
The adoptable integer charge (§7) widens the derived window
**1.85–2.51×** at the tabulated large declarations and **24×** at
256³; the exact-Chernoff ceiling it tracks is 2.05–2.70× (large) and
254× (256³). That is far past the >20% adoption bar. The budget is
finite and binding at every declaration from N ≈ 256³ up — enormously
slack at small N (order 100× the flat charge), never unbounded. §9
has the concrete charge formula; §10 derives **L(N)**, the
simultaneously-heavy-stage count byte-window-plan §B0.7's budget
division consumes.

## 1. The question

The landed charge (`window.rs`) prices every counted in-flight scope at
the full maximal skeleton — fan² `(radix, hash)` entries ≈ 1.06 MiB
plus a full fan of priced references, 1,209,344 B per scope — and
counts scopes per level boundary as `min(K, N/256^j)`:

```text
charged(K, N) = K + Σ_{j≥2} min(K, N / 256^j)
K_flat = max K with charged(K, N) × 1,209,344 ≤ budget − 17 × 1,114,624
```

Under uniform hashing both factors are loose near the frontier: listing
fullness thins (a reply carries entries only for slots that are
*occupied*, and occupancy at depth j is Binomial with mean N/256^j),
and only alternate depths park on one side of the wire (the stride-2
stream structure). This note quantifies the looseness and derives the
sharpened solve.

## 2. Geometry anchors (all re-verified against the campaign branch's code at the time of writing)

- **Keys are 32-byte content addresses**; the trie has branching factor
  256 and depth 32, one radix byte per level (`src/tree.rs` module docs;
  `Path::for_leaf`). Listing entries are `(u8, Hash)` with the 16-byte
  *Merkle* hash — 17 B packed (`window.rs::LISTING_ENTRY_BYTES`). The
  key length sets the depth range; the entry size sets the byte prices.
- **Stage anatomy** (`remote/adapter/scope.rs`, `remote/adapter/decode.rs`,
  `remote/proxy/state.rs`): the reply parked on the stream labeled
  depth d discusses the children of one **depth-(d−1) scope parent** —
  `Scope<H>` holds `parent: Prefix<S<H>>`, one height above the reply's
  own — so its **reactions sit at depth d** (one per listed child, in
  the question's positional order) and its **`Query` listing entries at
  depth d+1** (the children of the disputed depth-d child the reaction
  answers about). Every quantity below is indexed by the *stream's*
  depth d with the parent at d−1.
- **The population object** (`materialized/work/answer.rs`): the
  answering walk merge-joins the peer's listing against its own
  children and reacts to *every* pair in the union — `Match` when the
  hashes agree, `Query(listing)` when both sides occupy the child slot
  but disagree, `Query(empty)` when only the peer does (the supply
  request), `Supply` when only the local side does. Each `Query`, empty
  included, is one question, answered by exactly one reply on the next
  stage. So the population of stage d is the **queried-listing-entry
  count at depth d−1** — under full divergence, every peer entry — and
  NOT the branch count at depth d. Two structural bounds compose it:
  - an entry occupies a slot, and a slot is listed at most once, so
    stage-d population ≤ occupied depth-(d−1) slots ≤ min(256^(d−1), N);
  - an entry is listed only inside a `Query(listing)` reaction, which
    requires its *parent* to be disputed — occupied by **both**
    corpora — so stage-d population ≤ (jointly occupied depth-(d−2)
    slots) × (children per parent).

  The walk visits every level: `Backend::children` explodes one height
  at a time (`streaming/backend.rs`), so a single-child span still
  lists its one child per level — occupied slots, not branch nodes,
  are the entry currency, up to **~512× the branch count** just past
  the frontier (measured 265–506× in §6). The joint-occupancy bound is
  what turns populations back off past the *joint* frontier
  (256^(d−2) ≫ N²), and why the leaf-height stages (d = 32) are
  population-empty at every realistic declaration: a depth-31 dispute
  needs a 31-byte shared prefix with different content, a near-collision
  the model excludes — divergent leaves travel as `Supply` runs into
  backend custody instead (the `parking.rs` pins).
- **Stage structure** (`remote/codec/signal.rs`): 17 logical streams
  per direction; stream 0 carries height 31 for both speakers (for the
  initiator it is the opening *question*, replayed locally as the
  synthetic opening reply, `decode.rs::opening_reply`); successive
  streams descend two heights. Reply-carrying depths (depth = 32 −
  height):
  - initiator speaker: {2, 4, …, 30, 32};
  - responder speaker: {1, 3, …, 31, 32}.

  Each interior depth is spoken by exactly one role; the session
  descends one depth per stage, alternating speakers. A session parks
  decoded replies only on the stages its *peer* speaks; sizing must
  cover both possible peer roles (roles are elected at the handshake).
- **Landed solve replication** \[derived, pinned\]: the script
  reproduces `from_bytes` exactly and asserts K_flat(256⁵) = 4,644,
  matching the module test
  (`window/tests.rs::default_budget_derives_the_documented_window`).
  The backend advisory size is backed out from that pin:
  NODE_APPROX_MAX_BYTES = 340 is the unique integer in [0, 2048)
  consistent with K = 4,644.

## 3. Uniform occupancy, exactly \[derived\]

Premises: N keys, independent and uniform over 32-byte strings (the
content-address premise); both replicas hold ≤ N elements (the
declaration's meaning: N bounds the *deployment's set*, which both
honest replicas converge to).

For a fixed depth-j prefix, the leaf count beneath it is
L ~ Binomial(N, q), q = 256⁻ʲ, and conditional on L the continuation
bytes are iid uniform — exact, no Poissonization. Closed forms:

- **Slot occupancy**: p_occ(j) = 1 − (1 − 256⁻ʲ)ᴺ.
- **Joint occupancy**: two disjoint honest corpora are independent
  draws, so a slot is occupied by both with probability exactly
  p_occ(j)² — expected jointly occupied slots ≈ N²/256ʲ at small
  occupancy, the birthday scale that shuts populations off past the
  joint frontier.
- **Reply anatomy at stage d** (`message.rs`, §2's anatomy): the
  parked reply has one reaction per child of its depth-(d−1) parent
  the two sides jointly know (≤ occupied depth-d slots of the *union*
  corpus under the parent), and listing entries only inside `Query`
  reactions on *disputed* children — ≤ the replier's occupied
  depth-(d+1) slots under the parent (the envelope's one-sided bound;
  the disputed-children refinement is measured, untaken headroom, §6).

**Worst-case divergence** \[derived\]: every stage quantity above is
monotone in each side's corpus occupancy, and matches only prune (a
`Match` reaction ends the scope; a `Supply` reaction carries a node
handle, not a listing — its subtree streams into backend custody, per
the `parking.rs` pins). So the envelope evaluated with **both sides at
the full declared N, fully disjoint** pointwise dominates every honest
configuration — including the |A ∪ B| ≤ N constrained worst case,
which it strictly contains. D_max = N per side; no further
maximization over D is needed.

**Concentration**: all counts are sums of multinomial-occupancy
indicators, which are negatively associated (Dubhashi–Ranjan 1998) —
joint-occupancy indicators are products of two independent NA
families, increasing functions of disjoint coordinate blocks of the
concatenated NA vector, hence NA too — so Chernoff–Hoeffding bounds
apply verbatim. Failure budget: 2⁻⁴⁰ per session, allocated 2⁻⁴⁸ per
(stage, statistic). Each active stage consumes at most 9 allocations
(the occupied-slot, joint-slot, and per-parent children quantiles
behind S, plus a B1 quantile and a B2 exceeder count for each of the
three occupancy aggregates; B2's 2⁻²⁰ threshold is a quantile
property, not an event), and a stage is active only while the joint
mean N²/256^(d−2) reaches the 2⁻⁴⁸ level — at most ~24 depths for any
N ≤ 2⁶⁴ — so the union stays under 24 × 9 × 2⁻⁴⁸ < 2⁻⁴⁰. Per-parent
maxima are union-bounded over all 256ʲ candidate prefixes without
conditioning on being queried (P(queried ∧ X ≥ a) ≤ P(X ≥ a)), which
sidesteps the conditioning inflation entirely.

**What the certificate certifies**: with probability ≥ 1 − 2⁻⁴⁰ per
session, simultaneously (i) every stage's parked-reply count stays
within S(d) — the queried-listing-entry population bound — and
(ii) every parked reply's reactions, listing entries, and reference
fans stay within their per-parent quantiles and stage aggregates. The
certificate attaches to the population object of §2, not to branch
counts.

## 4. The sharpened envelope \[derived\]

Per stage (stream depth d, parents at depth d−1), the parked set is an
adversarially *scheduled* subset of at most min(K, S(d)) of the
stage's replies, with

```text
S(d) = min( occ_hi(N, d−1),  joint_hi(N, d−2) × c_q(N, d−2) ),   d ≥ 2
S(1) = 1                                (the opening reply / question)
```

where occ_hi is the 2⁻⁴⁸ quantile of occupied depth-(d−1) slots
(capped by 256^(d−1) and N — each leaf occupies one slot per level),
joint_hi the joint-occupancy quantile at depth d−2, and c_q the
per-parent children quantile (union-bounded): entries are listed only
under disputed parents, at most the joint parent count times the
per-parent fan. Three independently valid aggregate bounds per
occupancy quantity, charged at their minimum:

- **B1 (max bound)**: min(K, S) × per-parent quantile at the
  256^(d−1)-way union level;
- **B2 (threshold bound)**: min(K, S) × per-parent quantile at 2⁻²⁰,
  plus (high-probability count of parents exceeding it) × the
  deterministic per-parent cap;
- **B3 (corpus total)**: distinct parked replies own distinct
  depth-(d−1) parents with disjoint sub-slot ranges, so the stage
  total cannot exceed the corpus's occupied slot count at the sub-slot
  level: min(N, 256^(d−1+ℓ)) — deterministic. (This is the same
  identity behind byte-window-plan §B0.3's rejected-global-budget
  note: total parked entries are linear in N.)

The envelope, per peer role:

```text
E(K, N, role) = Σ_{d ∈ reply-depths(role), S(d) > 0}
                    min(K, S(d)) × C_reply
                  + C_reaction × Agg(2N, d−1, 256)     (reactions, union)
                  + 17 B      × Agg(N, d−1, 256²)      (listing entries;
                                                        none at d = 32)
              + [role = initiator] × (C_reply + C_reaction + 256 × 17 B)
                                                       (synthetic opening)
              + Σ_{d = 1..32, S(d) > 0}
                    (32 + 340) B × Agg(N, d−1, 256)    (question/scope refs)

K_sharp(N, budget) = max K with  E(K, N, role) ≤ budget − 17 × 1,114,624
                     for both peer roles
```

C_reply = 64 B \[estimated\], C_reaction = 32 B \[derived from the
decoded `Reaction` enum layout\]; §8 shows K_sharp moves ~1% when both
are quadrupled/doubled at the large declarations, so their imprecision
is immaterial where the verdict lives.

**No unconditional full-K term.** The landed charge prices its frontier
boundary (j = 1) at full K "because disputes, not corpus size, fill
it". Under the model of record, dispute counts *are* corpus-bounded at
2⁻⁴⁰ — that is precisely what S(d) certifies, and precisely what this
spike exists to establish (byte-window-plan §0: the saturable-stage
count is a consequence of uniformity, not a defensive assumption).
Populations are capped at both ends: near the root structurally
(S(2) ≤ 256, S(3) ≤ 65,536 — the fattest-priced stages hold few or
narrow replies), and past the joint frontier by the birthday decay
(S falls ~256× per depth once 256^(d−2) outruns N²).

**Smoothness and monotonicity** \[derived, verified numerically\]:
p_occ, p_occ², the quantiles, and the min()-composed aggregates are
continuous and monotone in N; min() introduces kinks (as the landed
charge's own min() already does) but no jumps. The script sweeps a
54-point grid including 256^k ± 2 for k = 2..6: K_sharp is monotone
nonincreasing (corpus-capped points treated as +∞), and the worst
relative step across adjacent declarations at the crossings is
2.8 × 10⁻⁷. The disqualifier — reintroduced cliffs — does not occur.

## 5. Results \[derived; script output\]

At the default 16 GiB budget (post-slack 15.98 GiB):

| N | K_flat | K_sharp | K_int (§7) | sharp/flat | int/flat |
|---|---|---|---|---|---|
| 256³ ≈ 1.7 × 10⁷ | 13,933 | 3,540,074 | 336,353 | **254.08** | 24.14 |
| 256⁴ ≈ 4.3 × 10⁹ | 6,966 | 18,800 | 16,833 | **2.70** | 2.42 |
| 256⁵ ≈ 1.1 × 10¹² | 4,644 | 11,949 | 11,652 | **2.57** | 2.51 |
| 10¹⁰ | 6,796 | 13,908 | 12,563 | **2.05** | 1.85 |
| 10¹² | 4,652 | 11,972 | 11,670 | **2.57** | 2.51 |

The budget binds at every tabulated declaration. At N = 256³ the
saturation envelope (every stage population-capped) is **20.14 GiB**,
only 1.26× the 15.98 GiB post-slack budget, so the solve lands at the
enormous-but-finite K = 3.54 × 10⁶ — order 100× the flat charge. On
the sweep grid the budget first binds at N ≈ 1.7 × 10⁷ ≈ 256³ (10⁷ is
still saturation-capped); below that, capacity beyond the widest stage
population is physically idle and the B0.7 knob semantics — not this
solve — own the small-N story (§10).

Per-stage envelope at the default declaration (N = 256⁵, K = 11,949;
peer as responder is the binding role; sub-KiB tail stages elided):

| depth | height | role | S | g_hi (entries) | skeleton charge | refs charge |
|---|---|---|---|---|---|---|
| 1 | 31 | resp | 1 | 65,536 | 1.07 MiB | 93 KiB |
| 2 | 30 | init | 256 | 65,536 | 274 MiB | 23.3 MiB |
| 3 | 29 | resp | 65,536 | 65,536 | **12.49 GiB** | 1.06 GiB |
| 4 | 28 | init | 1.7 × 10⁷ | 42,655 | 8.11 GiB | 1.06 GiB |
| 5 | 27 | resp | 4.3 × 10⁹ | 440 | 179 MiB | 992 MiB |
| 6 | 26 | init | 7.0 × 10¹¹ | 26 | 17.4 MiB | 106 MiB |
| 7 | 25 | resp | 1.1 × 10¹² | 9 | 6.1 MiB | 38 MiB |
| 8 | 24 | init | 3.9 × 10¹⁰ | 6 | 4.1 MiB | 25 MiB |
| 9 | 23 | resp | 1.0 × 10⁸ | 4 | 3.0 MiB | 17 MiB |
| 10 | 22 | init | 270,540 | 3 | 2.4 MiB | 12.7 MiB |

(Skeleton charges are the role that speaks the depth; refs are charged
to both roles. Totals at K = 11,949: responder role **15.98 GiB** —
binding — initiator role 11.71 GiB.)

**Where the looseness of the landed charge concentrates:**

1. **Direction split (≈ 2×, at all large N).** The landed charge
   counts three saturated boundaries; under stride-2, one *side* parks
   replies only at alternate depths, so at most ~⌈3/2⌉ of those
   boundaries can hold parked skeletons on the side being sized. The
   responder-parking side has exactly one full-fan²-priced stage
   (d = 3) where the landed charge prices three.
2. **Frontier thinning (the remaining ~1.3×, N-dependent).** The
   second stage a side parks at scale (d = 4 initiator / d = 5
   responder at the default) is priced 1.06 MiB by the landed charge
   but carries only 256² p_occ(d+1) expected entries — 41,427
   (0.67 MiB) at d = 4, 256 (4.3 KiB) at d = 5; envelopes 42,655 and
   440 respectively. Past
   the joint frontier the collapse is 100–5,000×: stages at depths
   6–10 (populations still exceeding K out to d ≈ 10) cost ~200–400 B
   per reply against the same 1.06 MiB price.
3. **Population capping at the ends (dominant at small N).** The
   d = 2 stage holds at most 256 replies ever and d = 3 at most
   65,536; past the joint frontier S(d) falls ~256× per depth. At
   N = 256³ the whole envelope saturates at 20.14 GiB, and the flat
   charge's K = 13,933 there underprices reality by two orders of
   magnitude.

The ratio is not monotone in N (2.70 at 256⁴, 2.05 at 10¹⁰, 2.57 at
256⁵): the landed charge is *least* loose just after a new boundary
saturates (at 10¹⁰ it charges 2K + 598 and the true fat stage carries
90%-full listings), *most* loose just before (at 256⁴ it charges
2K + 257 while the fat stage's listings are only 63% full). Toward
very large declarations the frontier-thinning component fades but the
direction split persists: the ratio settles near **2.2×** \[derived,
spot-checked: 2.18× at 2⁴⁸, 2.23× at 2⁵⁶\].

## 6. Simulation \[measured\]

Methodology, in three tiers (script sections; seeds 0..99):

1. **Brute force at N = 10⁵, 100 seeds.** Real uniform 32-byte keys
   (deduplicated); per-depth occupied-slot counts and per-occupied-
   parent children/grandchild-entry counts computed exactly from
   sorted keys (prefixes reduce to uint64: every statistic depends on
   ≤ 8 leading bytes at this N). The measured maximum occupied count
   and per-parent entry count stay below occ_hi and the per-parent
   quantile at every depth in all seeds (asserted, not eyeballed).
2. **Two-corpus measurement (disjoint 10⁵ vs 10⁵, 100 seeds)** — the
   corrected population object, end to end: jointly occupied slots
   match 256ʲ p_occ² exactly (40,128 measured vs 40,136 predicted at
   j = 2); **queried listing entries** (B-prefixes under jointly
   occupied parents — the stage population) measure 51,285 at j = 2,
   78,033 at j = 3, 592 at j = 4 against S envelopes of 52,141 /
   100,000 / 6,408 — asserted ≤ S in every seed. The listed/branch
   ratio measures **265× at j = 3 and 506× at j = 4** — the corrected
   population object versus the branch count the old object would
   have priced, the ~512× of §2. The disputed-children refinement
   (entries under mutually occupied children only, the true skeleton
   load) measures 0.02 vs the envelope's one-sided 1.95 at j = 2:
   frontier skeletons are Supply-dominated, the one-sided g is
   generous by design, and Supply parking is priced by the reference
   charge — this margin also absorbs the sim's abstraction (it
   validates occupancy statistics, not a wire replay; the alternating
   oracle already pins protocol behavior in-tree).
3. **Sampler tier at the tabulated N, 100 seeds.** The large-N sampler
   draws L ~ Binomial(N, 256⁻ʲ) per parent and places L balls
   uniformly into the 256² grandchild slots — *exact* by the
   conditional-uniformity of key continuations (§3), validated against
   tier 1 (p_occ to three digits, conditional entry means to three
   digits). Population estimates combine sampled conditional child
   means with the exact binomial marginals via the independence
   product (validated in tier 2). At every fat stage the prediction is
   exact to five digits (e.g. N = 10¹⁰, d = 3: predicted mean entries
   59,148.8, measured 59,149.0; measured q99.9 = 59,381 vs envelope
   59,851 — <1% loose *at the stages that carry the budget*). No
   stage at any N shows a bound violation. The >2× flags appear
   exclusively past the joint frontier, where a 2⁻⁴⁸ envelope is
   compared against a 10⁻³ empirical quantile of a mean-1 count —
   expected, and those stages price in the hundreds of bytes.

## 7. The adoptable integer charge \[derived; sweep-certified\]

The exact-Chernoff envelope has no place in `window.rs` (wire-normative
arithmetic must be exact integer math on every target; `exp` and
bisection on log-tails are not). **The adoptable formula is this
integer form — K_int, not K_sharp**: each quantile is replaced by a
u128-friendly integer bound, each sweep-verified to dominate its exact
counterpart over N ∈ {2 … 2⁵⁰}, parent depths j ∈ {0 … 32} (so the
integer envelope inherits the 2⁻⁴⁰ certificate):

```text
T(j)            = ⌈0.7 × (48 + 8j)⌉              (integer ≥ ln2 × union bits)
bern(μ, t)      = μ + isqrt(2 μ t) + t           (multiplicative-Chernoff
                                                  quantile at tail e^−t)
small(num/den, t): 0                if num × 2^(t+2) < den
                   t // (b−3) + 2   if b = bitlen(den) − bitlen(num) ≥ 5
                   (else: not sub-unit; use bern)     (Poisson-type tail)

occ_int(N, j)    = min(256^j, N)                              (deterministic)
joint_int(N, j)  = min(256^j, N, small-or-bern(N²/256^j, 48-level))
c_q_int(N, j)    = min(256, q_leaves, q_slots at fan)
S_int(N, d)      = min(occ_int(N, d−1), joint_int(N, d−2) × c_q_int(N, d−2))

q_leaves(N, j)   = small-or-bern(N/256^j at T(j))
q_slots(N, j, M) = min(M, bern(2NM // (2·256^(j+ℓ) + N) + 1, T(j)))
q(N, j, M)       = min(M, q_leaves, q_slots)
Agg(N, j, M, K)  = min( min(K, S_int) × q(N, j, M),  min(N, 256^(j+ℓ)) )

E_int as in §4 with Agg and S_int; K_int by the same binary search.
```

All divisions are integer divisions; N² needs u128 (fine for
N ≤ 2⁶⁴); `bitlen` is `u128::ilog2`-shaped; the stage sets are
compile-time constants mirroring `signal.rs` (and should be asserted
against `Stream::COUNT` the way `STAGES` already is). This is ~40
lines of const-friendly Rust in the same shape as the landed
`charged_scopes` + solve. It captures **1.85–2.51×** of the 2.05–2.70×
exact gain at the tabulated large N (90–98% of K_sharp; table in §5) —
the residual is the Bernstein/Poisson slack at mid-occupancy depths.

## 8. Sensitivity and residual looseness

- **Container constants**: quadrupling C_reply and doubling C_reaction
  moves K_sharp by ≤ 1.2% at every tabulated N ≥ 256⁴ \[measured\].
  At 256³ the swing is 42% (3.54M → 2.04M) — at small N the solve is
  container-dominated because every skeleton is thin — but the budget
  there is two orders of magnitude slack either way, so nothing the
  verdict rests on moves. Their \[estimated\] status is immaterial.
- **Decode slack**: kept at the landed 17 × 1,114,624 B. Sharpening it
  per stage (thin stages cannot produce 1.1 MiB encoded replies) is
  worth ~0.1% of the budget — declined as noise.
- **Deliberately untaken headroom** (each would tighten further, none
  is needed for the verdict): the disputed-children thinning of g
  (measured 0.02 vs 1.95 at frontier stages, §6 — Supply-dominated
  skeletons); mutual-occupancy thinning of the per-reply reaction
  breadth; top-K order statistics instead of the per-parent max at B1
  stages.
- **Where the sharpened envelope is genuinely tight**: the binding
  stage's price (d = 3 responder at large N) is the same fan² skeleton
  the landed charge uses, measured at full occupancy; nothing further
  exists there. The gain is structural (stage populations and
  occupancy), not a constant squeezed.

## 9. Verdict: adopt per-stage pricing

**Adopt**, with §7's integer charge as the concrete formula — K_int is
the adoptable object; K_sharp is the analytic ceiling it is measured
against, and its exact-Chernoff machinery stays in this note and the
script, never in code. The grounds:

1. **Size of the gain**: 1.85–2.51× in the integer-honest form
   (2.05–2.70× exact) at every tabulated realistic declaration, and
   24× (254× exact) at 256³ — far beyond the >20% bar the spike was
   chartered against, and well beyond the plan's own 1.2–1.5×
   estimate. In K-dial terms (single-socket.md §1.4) this halves-to-
   thirds the residual round trips at any window-bound divergence
   width, at zero memory cost — same budget, same envelope guarantee.
2. **Model honesty**: the charge now prices exactly what the model of
   record says a stage can hold (2⁻⁴⁰ concentration, premises stated,
   attached to the queried-listing-entry population), instead of a
   density premise the landed docs already flag as adversary-shaped
   (`window.rs` on `DEFAULT_MAX_IN_FLIGHT_OVERHEAD`: the
   path-compressed corner needs an *author*, which the model
   excludes; the byte-priced gate bounds it at the protocol level
   regardless).
3. **Cost**: ~40 lines of integer arithmetic in the same shape as the
   landed solve; smoothness and monotonicity verified (§4); the
   concentration certificate lives in this note and the companion
   script, not in code.

Constants change only in the later task byte-window-plan §B0.5 already
scopes ("constants change only where the landed derivation is tight"),
and the module tests' pinned figures move with them.

The budget **binds at every declaration from N ≈ 256³ up** — at 256³
it is enormously slack (the solve lands at ~254× the flat charge; the
saturation envelope is 20.14 GiB against the 15.98 GiB budget) but
finite and binding. Below ≈ 256³ the envelope saturates under the
budget and capacity beyond the widest stage population is physically
idle; no separate clamp policy is needed, because B0.7's knob
semantics retire the count window entirely — the meters bound bytes
directly and §10's L(N) divides the budget.

**Explicitly declined**: floating-point or log-space arithmetic in the
wire-normative charge (exactness across targets is non-negotiable);
per-stage decode-slack sharpening (0.1%); disputed-children and
mutual-occupancy refinements (real, measured, unneeded — keep as
recorded headroom for a future spike if a regime ever wants them).

## 10. L(N) — the simultaneously-heavy-stage count \[derived; B0.7's divisor\]

Byte-window-plan §B0.7 divides the byte budget across the 17
per-direction streams by the declaration: each stream advertises
budget/L(N), with the meter-enforced hard ceiling budget × 17/L(N).
This section derives L(N) from the corrected geometry. Throughout,
"budget" is the post-slack steady budget (16 GiB − 17 × 1,114,624 B =
15.98 GiB at the default), the same base the §4 solve prices against.

**The achievable-bytes object.** A_d(N) is the depth-d stage's
*achievable parked skeleton bytes*: the §4 stage term at unbounded
window (min(K, S) → S), i.e. what the stage could simultaneously hold
if only its own population and the corpus identities limited it.
Reference edges are excluded — B0.7 gives them their own meter; the
per-stream advertisement denominates parked replies.

**The threshold, derived.** The heaviness unit is the equal split
share = budget/17, and L is the *clamped fractional count*, per peer
role, worst role taken:

```text
L(N) = max(1, max_role Σ_{d ∈ reply-depths(role)} min(1, A_d(N) × 17 / budget))
```

This choice is not cosmetic; it makes the operating promise a theorem
\[derived\]:

> With per-stream advertisement adv = budget/L(N), meter-enforced,
> total parked bytes ≤ Σ_d min(A_d, adv) ≤ budget.
>
> Proof: L ≤ 17, so adv ≥ share. Split stages into big (A_d ≥ adv),
> mid (share ≤ A_d < adv), small (A_d < share): big and mid each
> contribute ≤ adv to the total and ≥ 1 to L; a small stage
> contributes A_d = share × (A_d/share) ≤ adv × (A_d/share) and
> exactly A_d/share to L. So the total ≤ adv × L = budget. ∎

The promise holds whenever the A_d envelopes hold — i.e. except with
probability < 2⁻⁴⁰ per session, on the same certificate as §3. The
hard ceiling budget × 17/L(N) is what the meters enforce regardless of
declaration error: mis-declaration degrades along that stated,
bounded overshoot factor, never a silent violation.

**Values** \[derived; script output\]:

| N | L(N) | 17/L | heavy stages (worst role) | adv = steady/L | ceiling |
|---|---|---|---|---|---|
| 256³ | 1.80 | 9.45 | resp {5} | 8.88 GiB | 151 GiB |
| 256⁴ | 3.00 | 5.66 | resp {3, 5, 7} | 5.32 GiB | 90.5 GiB |
| 256⁵ | 4.00 | 4.25 | resp {3, 5, 7, 9} | 3.99 GiB | 67.9 GiB |
| 10¹⁰ | 3.00 | 5.66 | resp {3, 5, 7} | 5.32 GiB | 90.5 GiB |
| 10¹² | 4.00 | 4.25 | resp {3, 5, 7, 9} | 3.99 GiB | 67.9 GiB |

(At the default declaration the division is **4.25× better than the
flat /17 split** — the corrected-geometry figure for B0.7's
parenthetical estimate. Spot-checks: L(2⁴⁸) = 5.00, L(2⁵⁶) = 6.00 —
one more heavy stage per 256× of N, as the joint frontier descends.)

**Smoothness and monotonicity** \[derived, verified numerically\]:
every A_d is monotone and continuous in N (composed of §3's monotone
quantiles), so L(N) is monotone nondecreasing and jump-free; on the
54-point grid the worst relative step across 256^k ± 2 crossings is
9.4 × 10⁻⁹, and L is exactly 1 below the first heavy declaration —
the settled-policy smoothness B0.7 requires.

**Threshold insensitivity** \[measured over θ ∈ {¼, ½, 1, 2, 4} ×
share\]: the integer heavy count H_θ moves by at most 1 across the
16× band at every tabulated N ≥ 256⁴ (e.g. at 256⁵ it is 4 at every
θ). At 256³ the two candidate stages sit at 1–2× share, so H_θ runs
2 → 0 across the extreme band while the fractional L(N) = 1.80 prices
them smoothly — which is exactly why the adopted object is the
clamped fractional count, not the integer count.

**Verified by simulation** \[measured\]: per seed, per stage, the
measured achievable bytes (sampled conditional per-parent statistics ×
exact slot marginals, mean level — tier 3 of §6) yield a measured
simultaneous heavy-stage count. At every tabulated N ≥ 256⁴ the
measured count equals the analytic H(×1) in every seed (3 at
256⁴/10¹⁰, 4 at 256⁵/10¹²); at 256³ it measures 1 vs analytic 1 —
the mean-level measurement and the hp-level count agree everywhere,
so heavy membership is not a quantile artifact, and L(N) upper-bounds
both by construction.
