# Formal verification of the streaming mirror protocol

**CAVEAT LECTOR:** This development was *entirely* "vibe-verified" using
Claude models, and has not been checked by a human expert in any of these
verification tools for correctness. It exists primarily as an experiment.

Two completed campaigns live here: deadlock-freedom of the streaming
mirror protocol over independent channels, and the mux conjectures —
whether a single bounded channel can replace them by local send-order
scheduling alone (settled both ways: eagerness provably fatal, an
inference-gated local scheduler provably live, the oracle recognized in
the base proof's own witness schedule).

The documentation of record is the Lean. Start here:

- **The claims** — [`lean/StreamingMirror/Statement.lean`](lean/StreamingMirror/Statement.lean) (campaign one)
  and [`lean/StreamingMirror/Mux/Statement.lean`](lean/StreamingMirror/Mux/Statement.lean) (campaign two): every
  statement of record restated inline and kernel-re-certified on every
  build. [`lean/StreamingMirror/Mux/Charters.lean`](lean/StreamingMirror/Mux/Charters.lean) holds the two
  planned follow-ons.
- **The human story** — [`doc/exposition.typ`](doc/exposition.typ) (by argument) and
  [`doc/narrative.typ`](doc/narrative.typ) (by discovery, errata and all); both
  self-contained.
- **The models and proofs** — [`MODEL.md`](MODEL.md) (the protocol model),
  [`PROGRESS.md`](PROGRESS.md) and [`PLAN.md`](PLAN.md) (campaign one's records), the maps of the proofs
  ([`lean/StreamingMirror/Proofs/Map.lean`](lean/StreamingMirror/Proofs/Map.lean), [`Mux/Proofs/Map.lean`](lean/StreamingMirror/Mux/Proofs/Map.lean)).
- **Running the checks** — [`check.sh`](quint/check.sh), `lake build`,
  `lake exe eventdag`, plus `lake exe muxprobe` / `just muxprobe`
  (the mux campaign's golden-pinned executable matrix). The Quint
  tier's manual is [`quint/README.md`](quint/README.md); exhaustive
  model checking was never achieved there — the deadlock-freedom
  claims rest on the kernel-checked Lean artifact (full statement in
  the manual).

Archaeology: the mux campaign's full coordination record (ledger,
adjudication, audit notes, statement audit, spec, panel briefs) was
last complete in the tree at commit `a5cf8e3b`; it lives in git
history, and everything load-bearing from it is folded into the Lean
doc comments.
