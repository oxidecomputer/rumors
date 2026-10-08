# auditor-l4 status
- Now: addendum report for the usize-invariance clause; lane at diminishing returns.
- Confirmed contract defects in lane: none. Harnesses silent and calibrated (integrator 70k + 600 deep cases; Rank values 40k; Count exhaustive).
- Doc defect (low): rank.rs:10-11, :130-131 misstate normalization. briefs/simplification-rank-normalization-prose.md
- Machinery: briefs/machinery-wasm32-rank-sum-pin.md (Sum has no 32-bit pin; calibrated), briefs/machinery-wasm32-trap-diagnosis.md (trap outcome conflates panic and allocation abort; calibrated).
- Observation for the owner: on wasm32, [deep, half].sum() aborts on allocation while deep + half and [half, deep].sum() succeed (observations.md O6).
- Cross-lane L6 lead (inferred): Rank::decode wasm32 capacity-overflow panic. cross-lane-l6-rank-decode-wasm32.md
- usize invariance (new clause): no target-dependent value behavior in lane. Brief: briefs/simplification-rank-width-invariant-routing.md (Rank +/checked_sub route by usize; accumulator route dead on 64-bit). Design proposal O8 (Count TryFrom usize), observation O9 (span * 4).
- Blocking questions: none.
