# Revision notes

- Preserved the V1 → V2 → single-socket sequence. Corrected the crossing
  comparison, clarified levels and terminal cases, added a worked exchange,
  and introduced the per-outgoing-reply publication order before the count.
- Separated the reply count, decoder independence, and statistical byte
  sizing. Stated the existing walk's progress result as an inherited
  prerequisite, rather than retaining an incomplete proof.
- Made the arrival invariant consistent throughout; separated attainable
  peak occupancy from the depth-one deadlock control. Expanded both the
  ordinary and opening wait cycles.
- Clarified the memory boundary, confidence assumptions, structural caps,
  and auxiliary-state accounting. Recast the numerical example as an
  illustrative per-party payload calculation, not a measured total.
- Aligned the appendix's events, scope custody, pipe topology, API rulings,
  commit boundaries, and acceptance conditions. Corrected the claims about
  priority and loss isolation.

The code-specific part of the height-mapping issue remains marked
`[verify: …]`: the supplied files do not establish which height convention
each `Progress` event and `Window::capacity` call uses. The mathematical
mapping is explicit. No other review fix was deliberately omitted; the
full baseline liveness proof remains outside scope under the review's
permitted inherited-prerequisite approach.
