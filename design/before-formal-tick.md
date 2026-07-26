# Formal verification of tick: total equivalence and amortized linearity

A standalone campaign, separated from the before-hardening campaign by the
project owner's direction (2026-07-26). This document is the campaign's
design doc of record: mission, claims, method, phases, gates, and
boundaries. It is fully self-contained; the hardening campaign's artifacts
are inputs, never dependencies of this document's meaning.

## 1. Mission

The hardest asymptotic argument in the hardening campaign was T-tick: that
`tick` (the paper's `event` — fill then grow) is computable over skyline
streams in amortized O(n + m) Accum digit touches. It took seven
adversarial specification rounds to reach an implementation for which the
claim survives attack, and its two refutations were both *missing rows in
a funding enumeration* — exactly the class of hole a kernel-checked proof
closes and prose cannot. The mission: mechanize the two claims below in
Lean, to the standard of the project's formal-verification practices, so
that the argument's completeness no longer rests on adversarial exhaustion.

## 2. The claims

Both claims are **total**. Neither carries an invariant hypothesis: the
crate's usage rules (linearity of parties, one universe) are *semantic*
safety rules — violating them costs meaning, never soundness or
performance — so nothing here is conditioned on them. (Owner's ruling,
2026-07-26: semantic claims are conditional on the invariants; cost and
equivalence claims are total.)

### Theorem A — unconditional equivalence, including rejections

> The skyline implementation and the paper's reference implementation of
> tick denote **the same total function** at the byte level:
> both are total functions `Bytes → Result<Bytes, Error>`, and they are
> equal — agreeing on every accepted input's output bytes AND on exactly
> which inputs are rejected.

No canonicality hypothesis: the rejection set is part of the proved
artifact, not a side condition. Consequences and sub-obligations:

- **The canonicality specification is formalized**: the accepted set of
  byte strings is characterized (minimal topology, natural heights, exact
  stream, bounded zero pad), and both sides reject its complement.
- **Error classification**: whether the two sides must agree on *which*
  error (truncated vs non-canonical vs trailing) or merely on rejection
  is a statement-document decision (Phase 0), recorded with rationale
  before any proof is attempted.
- **The coding bijection** (canonical stream ↔ minimal tree) is a lemma
  of the accepted-set characterization, not an axiom.
- This theorem kernel-pins the crate's bedrock identity claim — decode
  strictly rejects non-canonical input, which is what
  byte-equality-as-semantic-equality (`Eq`/`Hash` over raw bytes) rests
  on.

### Theorem B — amortized linearity of the transcription

> The tick algorithm — as a Lean transcription of the landed discipline
> (the anchor web, the frame ledger, the latent register Λ, invariant I4′
> width conservation, the fold-direction rule) — performs
> **O(n + m) digit touches per call, amortized within the call**, in a
> formal cost model whose unit is the Accum digit touch, via the I4′
> digit-count potential.

"Amortized within the call" means what the tick cost spec means: the
total touches of one tick are bounded by c₁·(n + m) + c₂ for fixed
constants, where n and m are the two operands' packed bit sizes; single
interior events may cost Θ(width), funded by the input that built the
width. Stretch scope (Phase 2 decision, not obligation): the orbit lemma
(iterated tick non-compounding) as a corollary over call sequences.

### What the theorems bind — the honesty clause

**Theorem B proves the transcription linear, not the Rust kernel.** The
bridge from the Lean transcription to the shipped code remains the
empirical fleet: the differential suites against the recursive oracle,
wire byte-identity pins, the mutation-verified meter pins in the gate,
the release-profile amplification board, and the fuzz-fit instruction
bands. The statement of record must say this in so many words
(statement-faithfulness: never weaker than stated, never stronger than
proven). Theorem A narrows the gap from the other side — it binds the
*reference semantics* to the *skyline semantics* at the byte level, so
the empirical fleet's oracle-differential leg is guarding a formally
unique function — but no claim in this campaign asserts properties of
compiled Rust.

## 3. Inheritance from the hardening campaign

Inputs (read, cited, never re-litigated here):

- `design/before-tick-cost-spec.md` — T-tick, invariants I1–I4′ (the
  digit-count potential, the complete hop table with per-hop funding),
  lemmas L0–L6, and §9's seven adversarial rounds. The hop table is the
  natural skeleton of Theorem B's case analysis; its two historical
  refutations (reveal-comb round 5, ascending-cliff round 7) are the
  motivating precedent for mechanization.
- `crates/before/reference/itc2008.md` — the paper's fill/grow/event
  equations; the semantic ground truth Theorem A's reference side
  transcribes.
- `crates/before/src/oracle/version.rs` — the in-tree recursive oracle:
  the executable reference the Lean reference model is calibrated
  against.
- The committed adversarial families and their pins — calibration
  vectors for the probe tier (a transcription that disagrees with the
  kernel on `reveal_comb(3,3)` is wrong before any proof begins).
- The model tier's lineage (`emit_model.py`, `cured_model.py`, the
  attack-round drivers): prior art for executable validation of the
  discipline; the Lean transcription supersedes them as the model of
  record for cost, once calibrated.

The existing Lean conventions in this repository apply unchanged: code
may cite the artifact **by theorem or definition name only** (never file
path); the model design document and progress notes are never cited from
code; kernel-checked statements carry the invariant inline where cited.

## 4. Method

The project's three standing formal-verification practices govern:

1. **Spec-first.** A committed statement-of-record document (the
   campaign's first deliverable) fixes, in English, before any Lean:
   each theorem's exact claim; per-clause audit rules (what a reviewer
   checks each clause against); the boundaries (what is NOT claimed —
   the honesty clause above, verbatim); and the open statement decisions
   (error classification; the cost model's exact touch-counting
   semantics and its correspondence to the kernel's `touch_meter`).
   Transcription choices thereafter are dated amendments, never silent
   drift.

2. **Statement-faithfulness.** Theorem statements and docstrings must be
   exactly accurate to intent — never weaker than stated, never stronger
   than proven. A WEAKER-grade audit finding is a wrong-grade finding by
   definition. Messy, non-uniform proof internals are explicitly
   acceptable; the statements are not negotiable.

3. **Probe first, prove second, pin forever.** Executable transcriptions
   precede induction effort. The reference model and skyline model are
   both executable (`#eval`/native), calibrated against the kernel on
   the full committed family roster plus fuzzed vectors BEFORE any
   theorem is attempted. Candidate lemmas get executable validation
   first; what the probe finds, the kernel pins. Where the probe
   disagrees with the kernel, the transcription is wrong — fix it, and
   add the disagreeing vector to the calibration set.

Review protocol: the iterated adversarial loop, adapted. Statement-tier
reviews attack the statement document (can a wrong artifact satisfy these
clauses?); proof-tier reviews attack transcription fidelity (does the
Lean model implement the prose discipline, or a different design that
happens to validate? — this genre fired twice in the hardening campaign);
audit-tier reviews grade every clause EXACT/WEAKER/STRONGER against the
statement of record.

## 5. Phases and gates

**Phase 0 — scoping probe and statement of record.** Deliverables:
(a) the statement-of-record document, committed; (b) executable Lean
transcriptions of the reference model (paper semantics over trees) and
the byte-level decode/accept spec, calibrated against the oracle and the
kernel's decode on committed families + fuzzed vectors; (c) a skeleton of
the skyline model sufficient to size the work; (d) an honest effort
report: lemma inventory, the hard cases (the frame ledger's deferral, the
latent register's σ algebra, the potential argument's hop totality), and
a scope recommendation. **GATE: the owner confers on the effort report
before Phase 2 is committed to.** Phase 1 may proceed on the owner's
approval of the statement document alone.

**Phase 1 — Theorem A.** The accepted-set characterization; the coding
bijection; the commutation square (skyline tick ∘ decode ≡ decode ∘
reference event, extended to rejection agreement). Acceptance: the
theorem statement audited EXACT against the statement of record; the
executable models remain calibrated (the proof may not drift the
transcription); the rejection-set characterization cross-checked against
the kernel's decode on an adversarial vector set (including
maximally-deferred defects).

**Phase 2 — Theorem B.** The cost model (digit-touch semantics fixed in
the statement document); the I4′ potential formalized; the hop table as
the case skeleton with each row's funding argument mechanized; the
top-level amortization theorem. Acceptance: statement audited EXACT;
every hop kind of the spec's enumeration appears in exactly one case of
the mechanized analysis (totality is the point — the two historical
refutations were missing rows); the transcription re-calibrated against
the kernel's touch meter on the family roster (constants need not match;
shapes must — flat families flat, the pre-cure red families' *cured*
readings linear).

**Phase 3 — integration.** Kernel pins: the code comments that today
carry prose invariants gain theorem-name citations where a
kernel-checked statement now backs them (invariant still stated inline);
the build gate extends to the Lean artifact (the workspace's existing
formal build conventions); the hardening campaign's design docs gain
their cross-references. The statement-of-record document becomes the
durable map from theorem names to English claims.

## 6. Risks and boundaries

- **Transcription fidelity is the campaign's central risk** — a proof
  about the wrong model is worse than no proof. Mitigation is the probe
  tier's calibration discipline plus proof-tier adversarial review; the
  hardening campaign caught this genre twice by exactly those means.
- **The rejection-set formalization is new scope** Theorem A picks up by
  being unconditional. It is also the highest-leverage sub-artifact (it
  formalizes the identity bedrock). If Phase 0 sizes it as dominant, the
  phase gate is where the owner re-scopes.
- **Theorem B is campaign-sized on its own.** The potential argument is
  formalization-ready in shape (an explicit Φ, a finite hop enumeration,
  per-hop funding), but the frame ledger's deferred out-of-order writes
  and the σ-tag algebra are intricate state; the effort report must be
  honest about the induction burden, and the owner decides at the gate.
- **No claim about compiled Rust.** Stated once more because it is the
  boundary most tempting to blur: the fleet binds the kernel; the
  theorems bind the models; Theorem A binds the models to each other;
  the campaign's product is that the *argument* is machine-checked, and
  the kernel's conformance to the argument remains continuously
  empirically enforced.
- **This campaign runs separately**: its own ledger (a section appended
  to this document per phase), its own branch/worktree discipline, its
  own review loops. It starts after the hardening campaign's retrospective
  (#36) closes, and inherits nothing in-flight.

## 7. Decision record

- 2026-07-26 (owner): campaign separated from before-hardening; this
  document is its standalone charter.
- 2026-07-26 (owner): Theorem A strengthened to unconditional — the two
  implementations must be the same total function including rejections.
- 2026-07-26 (owner): two-tier claim structure — semantic claims
  conditional on the usage invariants; cost and equivalence claims
  total. The invariants are semantic safety rules, not soundness rules.
- 2026-07-26 (owner): Theorem B's honesty clause — the proof binds the
  transcription; the empirical fleet remains the kernel's guard.
