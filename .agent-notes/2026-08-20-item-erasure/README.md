# Item-type erasure: `T` leaves the tree and session (part II)

The payload type erased behind `Arc<dyn Any>`: the tree and both mirror
protocols compile once into the rlib, with thin typed facades
downcasting at the crate's API boundary. Headline numbers: a minimal
one-`gossip`-call consumer pays 34,699 IR lines instead of 1,095,377
(height-erased) or 3,401,418 (pre-erasure); the warm-except-`rumors`
gate falls 249 s → 140 s; the runtime pin does not move. The sealing
lesson is recorded inside: extracting non-generic functions alone moves
nothing, because an `async fn` body codegens into whichever crate polls
it — the seal is returning the future boxed (the `dyn` coercion pins
the tower in the defining crate) plus `#[inline(never)]` against
cross-crate MIR inlining.

- [`item-erasure.md`](item-erasure.md) — the problem, the design
  options, the owner rulings, the staged implementation with per-stage
  measurements, and the acceptance numbers.

---

Retired from `design/item-erasure.md` with part II complete; the body is
the design document as it finished, already record-shaped.
