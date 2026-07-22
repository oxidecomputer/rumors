# Coordinator note: eventdag capOne data (2026-07-21)

Fresh `lake exe eventdag` run (full output:
../eventdag-out.txt). Load-bearing observations for the panel:

1. The capOne knob (ALL channel capacities forced to 1) makes the event
   DAG CYCLIC for pyramid4 and pyramid2 ("cap1: acyclic=false", cycles
   printed) while the standard-capacity DAG is acyclic for every pinned
   positive. So intra-party capacity reduction ALONE — no shared pipe,
   no mux — already kills every schedule on some skeletons the protocol
   otherwise completes.

   Consequence for the mux model: the mux must serialize ONLY the
   cross-party wire family (already cap-1 per stream) and leave every
   endpoint-internal channel at its modeled capacity. A model that
   accidentally throttles intra-party channels proves a vacuous
   impossibility (deadlock would occur for reasons unrelated to the
   shared pipe). Any C1 witness family must be double-checked to be
   cap1-acyclic-irrelevant: its deadlock must appear WITH standard
   endpoint capacities and only the wire pipe shared.

2. Conversely for C2/σ*: liveness claims should note that wire streams
   are cap-1 already, so "pipe capacity C=1 + per-stream slot" does not
   reduce any capacity below the proven-live baseline; the only new
   constraint a mux adds is the cross-stream FIFO coupling.

3. jam, smokeChain, rMix, comb6 stay acyclic at cap1; the 100-seed fuzz
   sweep (conjecture + candidate + replay) passed on this run.
