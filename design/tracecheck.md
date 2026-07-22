# tracecheck: the executable trace validator — design of record

Status: PLAN (2026-07-21, chartered by Finch). No code exists; this
document is the deliverable. Sequencing is first-class and §6 states
it: the single-R/W transport refactor (design/single-socket-plan.md)
lands before this work begins, and tier (d) is hard-blocked on it.
Companions: formal/MODEL.md §6 (the assumption/theorem interface this
mechanizes) and §10 (its correspondence table), formal/lean/Muxprobe.lean and
EventDag.lean (the two existing
executables whose patterns it copies — tracecheck is the third
sibling).

Epistemic key as in PROGRESS.md: **[proven]** kernel-checked;
**[checked]** executable evidence; **[derived]** paper argument;
**[open]** known unknown.

## 1. Purpose: the correspondence table becomes a per-trace fact

The artifact's trust bridge between Rust and Lean is a hand-maintained
table (formal/MODEL.md §10): each `Trace::assert_valid` check
(materialized/progress.rs:62–95 — seven checks: wire ledger, dependent
ledger, lower ledger, sibling contiguity, wire contiguity, radix
order, parent placement) is asserted to correspond to a model axiom
(AX_W, AX_D1, AX_D2, AX_D3, d4, d6, and the per-channel in-order
premise). The table is maintained socially: a person reads both sides
and vouches. Every campaign to date has honored it — and three
separate campaign findings (the D3 tightening, the D4 tightening, the
D6 addition) were discovered precisely because the two sides had
drifted and only a modeling effort noticed.

tracecheck replaces the vouching with a machine check that runs on
every traced session: the Rust proptest serializes the trace it just
validated locally, and a compiled Lean binary — built from **the same
source text the kernel checked the theorems against** — re-validates
it against the actual definitions. Any disagreement, in either
direction, is an automatic per-trace misalignment finding with the
first violated guard named in the axioms' vocabulary.

Four validation tiers, in value order:

- **(a) Ledger/axiom conformance.** The trace's publication sequence
  is replayed against the `.impl` guards. Rust-valid ∧ Lean-invalid ⇒
  `assert_valid` is weaker than the axioms the theorems assume (the
  D3-class failure mode, caught mechanically this time). Lean-valid ∧
  Rust-invalid ⇒ the Rust check over-constrains (harmless for
  soundness, still a finding). This tier alone retires the table's
  social contract.
- **(b) Hypothesis-class membership.** The trace-decoded skeleton
  checked by `Skel.wellFormed` and `Mux.margin0` — the theorems' own
  hypothesis predicates, not mirrors of them. Every traced session is
  thereby certified to sit inside the class the statements of record
  quantify over (the same conjunct Muxprobe.lean's expectation suite
  asserts for its matrix skeletons).
- **(c) Full semantic replay.** The trace mapped to a model action
  sequence, run through `Model.run` with greedy receive completion
  (§3.4), required to be accepted prefix-wise and to reach `terminal`.
  Per-trace behavioral refinement of the Rust session against the
  model — the strongest alignment statement short of a simulation
  proof, renewed on every proptest run.
- **(d) The σ*ₖ-engine judge** [blocked on the single-R/W refactor,
  §6]. The implementation's inference-ledger decisions validated
  against `inevitableA` (the causal closure — the engine's liveness
  spec per the T8 hand-off), and the B5 announced-skeleton
  reconstruction cross-checked Rust (`tests/skeleton.rs::announced`,
  :499) against Lean (`aviewOf`). This gives the implementation plan's
  reconciliation point ("S1 adopts the causal guard set verbatim;
  divergence adjudicated") an automated judge.

## 2. The trust posture

Compiled-Lean verdicts carry **compiler trust** — the `native_decide`
trust class, which this campaign bans for proofs and which tracecheck
does not smuggle back in. The validator never claims kernel authority;
no statement of record may cite a tracecheck run. Its value is
narrower and different: the evidence tier and the proof tier **share
source text**, so they cannot drift apart silently. Today the Rust
tests and the Lean theorems are aligned by the correspondence table; after
tracecheck they are aligned by the Lean elaborator.

The compiler-fidelity residue — "does the compiled binary compute what
the kernel-checked definitions denote?" — gets the house treatment
already established by the three `native_decide` cross-validation pins
(`positives_complete`, `phantom_walk_rejected`, `inv_along_positives`:
the only non-kernel trust on the positive side, each cross-pinned):
a small set of committed traces is validated BOTH by the compiled
binary and by kernel `decide` over the same decoded terms. A
divergence there would indict the compiler, loudly, on a pinned input.

One honest gap survives, by design: the **codec** (§3.3). Rust events
must be transcribed into model actions, and that transcription is code
someone wrote, not a theorem. The gap does not vanish; it shrinks from
a table of semantic claims to a serializer — property-tested from both
sides, and two orders of magnitude smaller than what it replaces.

## 3. Architecture

### 3.1 The executable

`lake exe tracecheck <batch-file> [--golden <tsv>] [--update]` — a
third `lean_exe` beside eventdag and muxprobe (lakefile.toml:17–31),
same conventions: reads one batch file of N traces, emits one stable
TSV verdict line per trace per tier (the `Cell.line` idiom,
Muxprobe.lean:68–69), exits nonzero on any violation or golden drift.
One process per batch, never per trace — startup is paid once.

Verdict format (stable, greppable):

    <trace-id> \t <tier> \t ok|FAIL \t <detail>

where `<detail>` on failure names the first violated guard in the
axioms' vocabulary and the offending position, e.g.
`d6: parent resolution departed with wire unsent, scope=0a3f, pos=214`
— the vocabulary a person needs to open Model.lean and the
correspondence table at the right row.

### 3.2 The serialization format

Defined from the actual event vocabulary (progress.rs:19–50): a trace
is a header line (`rootH`, endpoint ids, trace-id) followed by one
line per event:

    <work> \t <scope-hex> \t <kind> \t [pending]

with `<kind>` ∈ {`W` (Wire), `IQ` (InitialQuery), `RES` (Resolution,
pending attached), `DW` (DependentWork), `RDY` (Ready), `PAR`
(ParentResolution, pending attached)} — six kinds, verbatim the Rust
enum. Batch file = concatenated trace blocks separated by blank lines.
Plain TSV over JSON deliberately: the eventdag/muxprobe idiom, diffable
goldens, no parser dependency on either side.

### 3.3 The codec — the residual transcription gap

Rust side: `Trace → batch block` is a ~50-line serializer next to
`with_trace` (progress.rs:396). Lean side: `batch block → (Skel,
List Action)` in a small **library** module (`Tracecheck/Codec.lean`,
not exe-only — the kernel cross-pins of §2 must be able to `decide`
over decoded pinned traces, which requires the codec in the importable
library).

The skeleton half of the decode already exists in Rust
(`tests/skeleton.rs::decode`, :319 — Trace → Skel with the model's
count and parity laws audited en route) and its Lean twin is
mechanical: scope classification is D iff a `Resolution` was published
for it, R iff `Ready` at an internal prefix, leaf requests are `Ready`
at full-depth paths (skeleton.rs:311–318). The action half maps each
event to its model-action counterpart via (scope-prefix → BFS id,
height) — computable because the decoded skeleton fixes BFS order:

| Rust `Kind` (progress.rs:19) | Model counterpart | 1:1? |
|---|---|---|
| `Wire` | the scope's wire-send arm | yes — AX_W's subject |
| `InitialQuery` | the initiator's opener action | yes |
| `Resolution{pending}` | answerer walk resolution fire | yes; `pending` cross-checked against `dOf + rOf` (the decode already does this Rust-side) |
| `DependentWork` | child-query publication | yes — AX_D1/D2's subject |
| `Ready` | whole-subtree provision resolve | yes at internal prefixes; leaf-path `Ready` maps to the leaf-request supply arm |
| `ParentResolution{pending}` | asker parent-summary fire (the d6 subject) | yes |

"1:1 by construction" is not luck: `assert_valid`'s ledgers were built
to mirror the axioms (the correspondence table's whole point), so the event
vocabulary was co-designed with the model's action alphabet. The codec
is thin because the correspondence it encodes was engineered to be
thin. [derived — the exact per-arm mapping is stage-0 spike work; any
event that fails to map cleanly is a finding, not a codec hack.]

Property tests, both directions: Rust round-trip (serialize → parse →
re-serialize byte-identical); Lean decode-total-on-valid (every trace
whose Rust `assert_valid` passed decodes without the truncation
token). The Rust `decode`'s audits (count/parity laws) and the Lean
codec's checks overlap deliberately — each side's decoder validates
the other's output in tier (a) runs.

### 3.4 Receive completion (tier c)

The Rust trace records **publications only** — no receives
(progress.rs:1: "trace of the walk's progress-critical publications").
`Model.run` consumes both. Tier (c) therefore replays trace-ordered
sends with **greedy receive completion**: after each send, fire every
enabled receive/close action until none remains, then take the next
traced send. This is the deterministic-completion idiom the
executables already use (Gen's `orderGreedy`/drain family;
Model.lean's `drain`), and it is faithful: the model's receives are
Kahn-deterministic given the send order (per-channel FIFO, no
payload branching), so if ANY completion reaches terminal the greedy
one does [derived — from the confluence the artifact's drain lemmas
already exercise; made precise in stage 2, and any counterexample is
a finding about the model, which is the tool working].

## 4. Rust-side integration

A test-support module (`tests/tracecheck.rs`) that: (1) locates the
prebuilt binary (built by a `just tracecheck-build` recipe caching
into `formal/lean/.lake/build/bin/`, the muxprobe pattern —
justfile:187–200); (2) accumulates traces from proptest cases into a
batch file in the target dir; (3) invokes the binary once per batch
and maps FAIL lines back to the originating case (trace-id = case
seed, so failures replay).

Sampling policy:

- `just all` (the no-rot sweep): **every** traced session validated,
  tiers (a)–(c).
- Inner proptest loop (`cargo nextest run`): sampled — every Nth case
  (N ≈ 16) plus **every failing case** and **every
  proptest-regressions replay** (the committed seeds always validate;
  they are the acceptance choreography's traces of record, including
  the historical stall seeds once the single-socket transport lands).
- Binary missing (fresh clone, no elan): skip with a loud one-line
  note in the sampled path; hard error in `just all`.

Failure UX: the nextest failure message quotes the verdict line
verbatim — guard name, scope, position — plus the batch-file path for
re-running `lake exe tracecheck` by hand.

## 5. Performance budgets [open until stage 0 measures]

Reference points: `just muxprobe` ≈ 85 s for its full matrix
(hundreds of complete session runs plus commit scans); compiled Lean
binaries start in milliseconds-to-tens (eventdag's per-invocation
overhead is unnoticeable next to its sweep). Per-trace tier-(a)/(c)
cost is one replay, linear in events with a small per-step scan —
the shrunk regression traces are hundreds of events; random proptest
traces are similar. Working estimates: ≤ 10 ms per trace replay,
batches of 500–1000 per invocation, so full validation of a
1,000-case proptest run ≈ 10–20 s — acceptable in `just all`,
which is why the inner loop samples. Stage 0 measures: binary startup
with the library's statics, per-event replay cost at rootH 32 (the
real Rust traces, not the model instances' rootH 6), decode cost, and
batch-size sweet spot — BEFORE any gate placement is committed.

## 6. Staging — sequencing constraint first

**Nothing in this plan begins until the single-R/W transport refactor
lands** (Finch's directive, 2026-07-21). Tiers (a)–(c) are technically
buildable against the materialized trace today, but they compete with
the refactor for the same module's attention and review bandwidth, and
tier (d) — the tier that pays for the plan — needs the refactor's
artifacts to exist: the σ*ₖ engine's ledger (its decisions are what
the judge validates), the greeting's window-advertisement fields (the
K the ledger is judged against), and the eager-conversion decode path
(the observation source, per the payload finding: the engine's
inference is fed by frame CONTENTS, so the judge must see what the
decoder saw).

- **Stage 0 — spike.** The binary + codec for tier (a) only, on the
  committed regression traces and one proptest batch; the §5
  measurements; a written go/no-go on gate placement. ~300–500 Lean +
  ~200 Rust; 1 agent-session.
- **Stage 1 — tiers (a)+(b), gate-wired.** Golden file for the pinned
  traces; the kernel cross-pins (§2); `just tracecheck` recipe; `just
  all` integration; the sampled inner-loop hook. ~400–700 Lean +
  ~300 Rust + justfile; 1–2 sessions.
- **Stage 2 — tier (c).** Greedy receive completion, terminal +
  conservation checks, the confluence note made precise. ~300–600
  Lean; 1 session.
- **Stage 3 — tier (d).** The engine-ledger judge against
  `inevitableA`; the B5 Rust-vs-Lean announced-skeleton cross-check;
  the single-socket acceptance line item ("the historical stall seeds
  don't just complete — their traces validate"). ~500–900 Lean + the
  Rust harness; 1–2 sessions. **Hard-blocked on the refactor.**

Total ≈ 1.5–2.7k lines across 4–6 agent-sessions.

## 7. Negative space — what tracecheck is not

- **Not a proof.** No kernel claims, ever; no statement of record may
  cite it (§2). It is evidence-tier tooling that shares source with
  the proof tier.
- **Not a payload validator.** The model erases payloads; validation
  covers structure, order, and the label stratum that B5
  reconstructs — never message bytes beyond labels. Content
  correctness stays with the existing snapshot and convergence tests.
- **Not a replacement for `assert_valid`.** The in-process Rust check
  stays as the fast first line with its superior failure locality;
  tracecheck is the alignment check behind it. Removing the Rust check
  would reintroduce the drift problem one level up.
- **Not a generator.** Proptest owns generation; tracecheck validates
  what the Rust produced. (A Lean-side generator would test the model
  against itself — muxprobe already does that.)
- **Not online.** Batch, post-hoc, by design — no validation on the
  session's hot path, no live judging of production traffic.
- **Not a transport conformance suite.** Framing, window accounting,
  and socket behavior belong to the single-socket plan's acceptance
  harness; tracecheck sees the protocol layer's events only.

## 8. Open questions for Finch

1. **Disagreement severity.** When Rust-valid ∧ Lean-invalid appears
   in the sampled inner loop, hard-fail the test or collect-and-warn
   (hard-fail only in `just all`)? Recommendation: hard-fail
   everywhere — a misalignment is a finding worth stopping for, and
   sampling already bounds the noise surface.
2. **Codec home.** Library module (enables kernel cross-pins; adds
   ~1 s to `lake build`) vs exe-only (cheaper builds, no `decide`
   over decoded traces)? Recommendation: library module — the
   cross-pins are the trust story's load-bearing half.
3. **Golden scope.** Pin verdicts for ALL committed
   proptest-regressions traces, or only the historical stall seeds?
   Recommendation: all — regressions exist because they once bit, and
   their trace-validation verdicts are exactly the invariants they
   protect.
4. **Transcript tier.** Extend tier (d) to validate the wire-label
   `Transcript` (`announced`, skeleton.rs:499) against Lean's
   `aviewOf` as a fifth check, or keep tier (d) ledger-only?
   Recommendation: extend — it is the payload finding's constitutive
   bridge (B5) run continuously, and the decoder exists on both sides.
5. **CI posture for the Lean toolchain in Rust CI.** Require elan +
   the prebuilt binary in the Rust test image (heavier image,
   always-on validation) vs `just all`-only (lighter, validation on
   the sweep cadence)? Recommendation: `just all`-only until stage 1's
   measurements; revisit if a misalignment ever slips the sweep.
