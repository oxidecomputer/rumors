//! Working-form tests: `repack ∘ unpack == identity` and yields canonical bytes.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::working::WorkingVersion;
use super::{Batch, Version};
use crate::testing::bridge::{from_oracle_party, from_oracle_version, to_oracle_version};
use crate::testing::complexity::{assert_linear_scaling, steps_of, MIN_SCALE};
use crate::testing::generators::{
    arb_oracle_party_nonempty, arb_oracle_version, arb_shape, shape_party, shape_version, Shape,
};
use crate::testing::grow_brute_force::{all_inflations, best_inflation};
use crate::testing::optrace::{leq as oracle_leq, run, versions, world_strategy};
use crate::Party;

/// `a <= b` under the impl causal order.
fn le(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

// ───────────────────────────── working form ─────────────────────────────

/// `unpack` lays out a known event tree as preorder topology + base arrays.
#[test]
fn unpack_layout() {
    use crate::oracle::Version as V;
    // (0, 1, 0): internal root, two leaves.
    let v = from_oracle_version(&V::node(0u64, V::leaf(1u64), V::leaf(0u64)));
    let w = WorkingVersion::unpack(v.as_bits());
    assert_eq!(w.len(), 3);
    assert_eq!(
        w.topo.iter().by_vals().collect::<Vec<_>>(),
        [true, false, false]
    );
    assert_eq!(w.base, [0u32, 1, 0].map(crate::codec::Base::from));
}

proptest! {
    /// `repack(unpack(v)) == v` and the repacked bytes are canonical (equal to
    /// `v`'s own encoding).
    #[test]
    fn working_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let v = from_oracle_version(&vs[i % n]);

        let work = WorkingVersion::unpack(v.as_bits());
        let repacked = Version::from_bits(work.repack());

        prop_assert!(repacked == v);
        prop_assert_eq!(repacked.encode(), v.encode());
    }
}

// ───────────────────────────── causal order ─────────────────────────────

proptest! {
    /// Complexity. The causal order is `O(n + m)`: comparing `a` against `b = a
    /// | extra` drives the bounded lazy-skip at scale. `a <= b` always holds
    /// (so the walk traverses fully, no early `false`), and where `extra` added
    /// structure that `a` lacks, `a`'s leaf aligns with `b`'s subtree, so `b`'s
    /// dominated subtree is skipped once under that leaf. Building `a` and
    /// `extra` from independent shapes maximizes such misalignments. Steps stay
    /// linear from `scale` to `4 * scale`.
    #[test]
    fn leq_is_linear(
        shape_a in arb_shape(),
        shape_b in arb_shape(),
        scale in MIN_SCALE..256,
    ) {
        let measure = |s: usize| {
            let a = shape_version(shape_a, s);
            let extra = shape_version(shape_b, s);
            let b = a.clone() | extra; // a <= b always; b has subtrees where a has leaves
            steps_of(|| {
                let _ = a.partial_cmp(&b);
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Differential. The impl causal order agrees with the oracle's on every
    /// generated pair; this subsumes the order laws since the oracle satisfies
    /// them (its `version_partial_order` property) and the impl matches it
    /// exactly.
    #[test]
    fn compare_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let (oa, ob) = (&vs[i % n], &vs[j % n]);
        let (ia, ib) = (from_oracle_version(oa), from_oracle_version(ob));
        prop_assert_eq!(ia.partial_cmp(&ib), oa.partial_cmp(ob));
    }
}

proptest! {
    /// The order laws on impl versions directly: reflexive, antisymmetric,
    /// transitive; `==` ⇔ `Some(Equal)`; concurrency ⇔ `None`.
    #[test]
    fn order_laws(ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let (a, b, c) = (
            from_oracle_version(&vs[i % n]),
            from_oracle_version(&vs[j % n]),
            from_oracle_version(&vs[k % n]),
        );

        prop_assert_eq!(a.partial_cmp(&a), Some(Ordering::Equal)); // reflexive
        if le(&a, &b) && le(&b, &a) {
            prop_assert!(a == b); // antisymmetric
        }
        if le(&a, &b) && le(&b, &c) {
            prop_assert!(le(&a, &c)); // transitive
        }
        prop_assert_eq!(a == b, a.partial_cmp(&b) == Some(Ordering::Equal));
        let concurrent = !le(&a, &b) && !le(&b, &a);
        prop_assert_eq!(concurrent, a.partial_cmp(&b).is_none());
    }
}

proptest! {
    /// The comparison matrix agrees: `cmp(a,b)`, `cmp(a.batch(),b)`,
    /// `cmp(a,b.batch())`, and `cmp(a.batch(),b.batch())` all equal the bare
    /// version comparison (a fresh batch reflects its version).
    #[test]
    fn representation_parity(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let base = a.partial_cmp(&b);

        let mut ba = a.clone();
        let mut bb = b.clone();
        let batch_a = ba.batch();
        let batch_b = bb.batch();

        prop_assert_eq!(batch_a.partial_cmp(&b), base); // Batch vs Version
        prop_assert_eq!(a.partial_cmp(&batch_b), base); // Version vs Batch
        prop_assert_eq!(batch_a.partial_cmp(&batch_b), base); // Batch vs Batch
        prop_assert_eq!(a == b, batch_a == batch_b); // PartialEq matrix agrees
    }
}

proptest! {
    /// Parity holds once the working form is *materialized*, exercising the
    /// equality short-circuit's working-form arms. Merging the join identity
    /// (`Version::new()`) forces `work = Some(..)` without changing the value,
    /// so each batch now compares as a working form. The matrix — materialized
    /// vs materialized (Working/Working), materialized vs packed (the mixed arm
    /// that declines and falls through) — still equals the bare version
    /// comparison.
    #[test]
    fn materialized_batch_parity(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let base = a.partial_cmp(&b);

        let mut ba = a.clone();
        let mut bb = b.clone();
        let mut batch_a = ba.batch();
        let mut batch_b = bb.batch();
        batch_a.join(&Version::new()); // materialize the working form, value unchanged
        batch_b.join(&Version::new());

        prop_assert_eq!(batch_a.partial_cmp(&b), base); // Working vs Packed (mixed)
        prop_assert_eq!(a.partial_cmp(&batch_b), base); // Packed vs Working (mixed)
        prop_assert_eq!(batch_a.partial_cmp(&batch_b), base); // Working vs Working
        prop_assert_eq!(a == b, batch_a == batch_b); // PartialEq matrix agrees
    }
}

/// Assert one comparison-matrix cell agrees with `expected`: its `partial_cmp`
/// (`PartialOrd`) and `==`/`!=` (`PartialEq`), plus the four ordering operators
/// `partial_cmp` derives. Generic over the operand types, so each call resolves
/// to exactly the impl for `(L, R)` — `assert_cmp_cell(&a, b, ..)` exercises the
/// `&Lhs`/`Rhs` cell, `assert_cmp_cell(a, &b, ..)` the `Lhs`/`&Rhs` cell, `&`/`&`
/// the std blanket — with no method-resolution ambiguity to mask which cell ran.
/// A cell wired into a delegation cycle overflows the stack here rather than
/// diverging silently in production.
fn assert_cmp_cell<L, R>(lhs: L, rhs: R, expected: Option<Ordering>) -> Result<(), TestCaseError>
where
    L: PartialEq<R> + PartialOrd<R>,
{
    prop_assert_eq!(lhs.partial_cmp(&rhs), expected);
    prop_assert_eq!(lhs == rhs, expected == Some(Ordering::Equal));
    prop_assert_eq!(lhs != rhs, expected != Some(Ordering::Equal));
    prop_assert_eq!(lhs < rhs, expected == Some(Ordering::Less));
    prop_assert_eq!(lhs > rhs, expected == Some(Ordering::Greater));
    prop_assert_eq!(
        lhs <= rhs,
        matches!(expected, Some(Ordering::Less | Ordering::Equal))
    );
    prop_assert_eq!(
        lhs >= rhs,
        matches!(expected, Some(Ordering::Greater | Ordering::Equal))
    );
    Ok(())
}

proptest! {
    /// The full comparison matrix over {Version, Batch}² — every owned and
    /// borrowed form of each operand, covering all twenty-four generated
    /// `PartialEq`/`PartialOrd` impls plus the `&Lhs`/`&Rhs` std blanket forms —
    /// agrees with the oracle's verdict on the same pair. Pinning every cell to
    /// one source of truth is the "the cells can't drift out of sync" guarantee;
    /// invoking every cell is the "no cell recurses forever" guarantee. Each
    /// `Batch` operand is a fresh batch over a clone (read-only here, so reused
    /// across the operators within a cell).
    #[test]
    fn compare_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = vs[i % n].partial_cmp(&vs[j % n]); // oracle: the source of truth
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version × Version (owned/owned, &/owned, owned/&, &/& blanket).
        assert_cmp_cell(a.clone(), b.clone(), expected)?;
        assert_cmp_cell(&a, b.clone(), expected)?;
        assert_cmp_cell(a.clone(), &b, expected)?;
        assert_cmp_cell(&a, &b, expected)?;

        // Version × Batch.
        { let mut cb = b.clone(); assert_cmp_cell(a.clone(), cb.batch(), expected)?; }
        { let mut cb = b.clone(); assert_cmp_cell(&a, cb.batch(), expected)?; }
        { let mut cb = b.clone(); let rb = cb.batch(); assert_cmp_cell(a.clone(), &rb, expected)?; }
        { let mut cb = b.clone(); let rb = cb.batch(); assert_cmp_cell(&a, &rb, expected)?; }

        // Batch × Version.
        { let mut ca = a.clone(); assert_cmp_cell(ca.batch(), b.clone(), expected)?; }
        { let mut ca = a.clone(); let lb = ca.batch(); assert_cmp_cell(&lb, b.clone(), expected)?; }
        { let mut ca = a.clone(); assert_cmp_cell(ca.batch(), &b, expected)?; }
        { let mut ca = a.clone(); let lb = ca.batch(); assert_cmp_cell(&lb, &b, expected)?; }

        // Batch × Batch.
        { let mut ca = a.clone(); let mut cb = b.clone();
          assert_cmp_cell(ca.batch(), cb.batch(), expected)?; }
        { let mut ca = a.clone(); let mut cb = b.clone(); let rb = cb.batch();
          assert_cmp_cell(ca.batch(), &rb, expected)?; }
        { let mut ca = a.clone(); let mut cb = b.clone(); let lb = ca.batch();
          assert_cmp_cell(&lb, cb.batch(), expected)?; }
        { let mut ca = a.clone(); let mut cb = b.clone(); let lb = ca.batch(); let rb = cb.batch();
          assert_cmp_cell(&lb, &rb, expected)?; }
    }
}

// ───────────────────────────── event mutation ─────────────────────────────

/// `Version::new()` is the empty history and the two-sided identity for `|`.
#[test]
fn new_is_join_identity() {
    use crate::oracle::Version as V;
    let empty = Version::new();
    assert!(empty == from_oracle_version(&V::leaf(0u64))); // empty history is Leaf(0)
    assert!(Version::default() == empty); // Default delegates to new()
    for v in [
        V::leaf(0u64),
        V::leaf(7u64),
        V::node(1u64, V::leaf(0u64), V::leaf(2u64)),
    ] {
        let iv = from_oracle_version(&v);
        assert!(empty.clone() | iv.clone() == iv);
        assert!(iv.clone() | empty.clone() == iv);
    }
}

proptest! {
    /// The impl `tick` matches the oracle's `event` for every
    /// clock's own `(party, version)` (the party owns the regions tick may inflate).
    #[test]
    fn tick_matches_oracle(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (party, version) = cs[i % n].trees();

        let mut oracle_after = version.clone();
        oracle_after.tick(party);

        let mut iv = from_oracle_version(version);
        iv.tick(&from_oracle_party(party));

        prop_assert!(iv == from_oracle_version(&oracle_after));
    }
}

proptest! {
    /// Differential. The impl version join (`|`) matches the oracle's `join`.
    #[test]
    fn merge_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let oracle_join = vs[i % n].clone() | vs[j % n].clone();
        let merged = from_oracle_version(&vs[i % n]) | from_oracle_version(&vs[j % n]);
        prop_assert!(merged == from_oracle_version(&oracle_join));
    }
}

proptest! {
    /// Differential. The impl version meet (`&`) matches the oracle's `meet`,
    /// dual to [`merge_matches_oracle`].
    #[test]
    fn meet_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let oracle_meet = vs[i % n].clone() & vs[j % n].clone();
        let met = from_oracle_version(&vs[i % n]) & from_oracle_version(&vs[j % n]);
        prop_assert!(met == from_oracle_version(&oracle_meet));
    }
}

proptest! {
    /// Every assigning / batch join surface on `Version` yields the same result
    /// as `a | b`, which `merge_matches_oracle` already pins to the oracle's
    /// `join`. Covers `Version |= Version`, the `From<&mut Version>` batch
    /// conversion, and the `Batch |= &Version` operator (committed on drop) —
    /// none of which the by-value `|` differential reaches.
    #[test]
    fn version_assign_join_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() | vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // `Version |= Version`.
        let mut assign = a.clone();
        assign |= b.clone();
        prop_assert!(assign == expected);

        // `Batch |= &Version`, over a batch built via `From<&mut Version>`, committed on
        // drop.
        let mut batched = a.clone();
        {
            let mut batch: Batch = (&mut batched).into();
            batch |= &b;
        }
        prop_assert!(batched == expected);
    }
}

proptest! {
    /// The full `|` (BitOr) matrix over {Version, Batch}² — every owned and
    /// borrowed form of each operand — equals the oracle's `join`, which
    /// `merge_matches_oracle` already pins for the bare Version×Version case. A
    /// fresh `Batch` reflects its `Version`, so each of the sixteen
    /// representation/reference cells must agree. Each `Batch` operand gets its
    /// own clone in a tight scope: an owned-`Batch` operand is consumed by `|`
    /// and commits (unchanged) on drop, so a fresh one is built per cell.
    #[test]
    fn join_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() | vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version × Version (four reference forms).
        prop_assert!(a.clone() | b.clone() == expected);
        prop_assert!(&a | b.clone() == expected);
        prop_assert!(a.clone() | &b == expected);
        prop_assert!(&a | &b == expected);

        // Version × Batch.
        { let mut bb = b.clone(); prop_assert!(a.clone() | bb.batch() == expected); }
        { let mut bb = b.clone(); prop_assert!(&a | bb.batch() == expected); }
        { let mut bb = b.clone(); let r = bb.batch(); prop_assert!(a.clone() | &r == expected); }
        { let mut bb = b.clone(); let r = bb.batch(); prop_assert!(&a | &r == expected); }

        // Batch × Version.
        { let mut aa = a.clone(); prop_assert!(aa.batch() | b.clone() == expected); }
        { let mut aa = a.clone(); prop_assert!(aa.batch() | &b == expected); }
        { let mut aa = a.clone(); let l = aa.batch(); prop_assert!(&l | b.clone() == expected); }
        { let mut aa = a.clone(); let l = aa.batch(); prop_assert!(&l | &b == expected); }

        // Batch × Batch.
        { let mut aa = a.clone(); let mut bb = b.clone();
          prop_assert!(aa.batch() | bb.batch() == expected); }
        { let mut aa = a.clone(); let mut bb = b.clone(); let r = bb.batch();
          prop_assert!(aa.batch() | &r == expected); }
        { let mut aa = a.clone(); let mut bb = b.clone(); let l = aa.batch();
          prop_assert!(&l | bb.batch() == expected); }
        { let mut aa = a.clone(); let mut bb = b.clone(); let l = aa.batch(); let r = bb.batch();
          prop_assert!(&l | &r == expected); }
    }
}

proptest! {
    /// The full `|=` (BitOrAssign) matrix: {Version, Batch} left operands against
    /// {Version, Batch} right operands, every reference form, all landing on the
    /// oracle's `join`. A `Batch` left operand commits on drop, so it is scoped
    /// and its underlying `Version` checked afterward.
    #[test]
    fn join_assign_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() | vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version |= Version / &Version.
        { let mut x = a.clone(); x |= b.clone(); prop_assert!(x == expected); }
        { let mut x = a.clone(); x |= &b; prop_assert!(x == expected); }

        // Version |= Batch / &Batch.
        { let mut x = a.clone(); let mut bb = b.clone(); x |= bb.batch(); prop_assert!(x == expected); }
        { let mut x = a.clone(); let mut bb = b.clone(); let r = bb.batch(); x |= &r; prop_assert!(x == expected); }

        // Batch |= Version / &Version (committed on drop).
        { let mut x = a.clone(); { let mut bx = x.batch(); bx |= b.clone(); } prop_assert!(x == expected); }
        { let mut x = a.clone(); { let mut bx = x.batch(); bx |= &b; } prop_assert!(x == expected); }

        // Batch |= Batch / &Batch (committed on drop).
        { let mut x = a.clone(); let mut bb = b.clone();
          { let mut bx = x.batch(); bx |= bb.batch(); } prop_assert!(x == expected); }
        { let mut x = a.clone(); let mut bb = b.clone(); let r = bb.batch();
          { let mut bx = x.batch(); bx |= &r; } prop_assert!(x == expected); }
    }
}

proptest! {
    /// The join matrix holds once the `Batch` operands are *materialized* to
    /// working form, exercising `snapshot`'s repack and `merge_view` joining a
    /// Working-form incoming view (the fresh-batch cells above all read packed
    /// views). Merging the join identity (`Version::new()`) forces
    /// `work = Some(..)` without changing the value.
    #[test]
    fn materialized_join_parity(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() | vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // `&Batch | &Batch`, both materialized: Working snapshot joined with a
        // Working incoming view.
        {
            let mut aa = a.clone();
            let mut bb = b.clone();
            let mut la = aa.batch();
            la.join(&Version::new());
            let mut rb = bb.batch();
            rb.join(&Version::new());
            prop_assert!(&la | &rb == expected);
        }

        // `Version |= &Batch` with the batch materialized.
        {
            let mut x = a.clone();
            let mut bb = b.clone();
            let mut rb = bb.batch();
            rb.join(&Version::new());
            x |= &rb;
            prop_assert!(x == expected);
        }
    }
}

proptest! {
    /// The full `&` (BitAnd) matrix over {Version, Batch}² — every owned and
    /// borrowed form of each operand — equals the oracle's `meet`, dual to
    /// [`join_matrix_matches_oracle`]. `meet_matches_oracle` pins the bare
    /// Version×Version cell; a fresh `Batch` reflects its `Version`, so each of
    /// the sixteen representation/reference cells must agree. Each `Batch`
    /// operand gets its own clone in a tight scope: an owned-`Batch` operand is
    /// consumed by `&` and commits (unchanged) on drop, so a fresh one is built
    /// per cell.
    #[test]
    fn meet_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() & vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version × Version (four reference forms).
        prop_assert!(a.clone() & b.clone() == expected);
        prop_assert!(&a & b.clone() == expected);
        prop_assert!(a.clone() & &b == expected);
        prop_assert!(&a & &b == expected);

        // Version × Batch.
        { let mut bb = b.clone(); prop_assert!(a.clone() & bb.batch() == expected); }
        { let mut bb = b.clone(); prop_assert!(&a & bb.batch() == expected); }
        { let mut bb = b.clone(); let r = bb.batch(); prop_assert!(a.clone() & &r == expected); }
        { let mut bb = b.clone(); let r = bb.batch(); prop_assert!(&a & &r == expected); }

        // Batch × Version.
        { let mut aa = a.clone(); prop_assert!(aa.batch() & b.clone() == expected); }
        { let mut aa = a.clone(); prop_assert!(aa.batch() & &b == expected); }
        { let mut aa = a.clone(); let l = aa.batch(); prop_assert!(&l & b.clone() == expected); }
        { let mut aa = a.clone(); let l = aa.batch(); prop_assert!(&l & &b == expected); }

        // Batch × Batch.
        { let mut aa = a.clone(); let mut bb = b.clone();
          prop_assert!(aa.batch() & bb.batch() == expected); }
        { let mut aa = a.clone(); let mut bb = b.clone(); let r = bb.batch();
          prop_assert!(aa.batch() & &r == expected); }
        { let mut aa = a.clone(); let mut bb = b.clone(); let l = aa.batch();
          prop_assert!(&l & bb.batch() == expected); }
        { let mut aa = a.clone(); let mut bb = b.clone(); let l = aa.batch(); let r = bb.batch();
          prop_assert!(&l & &r == expected); }
    }
}

proptest! {
    /// The full `&=` (BitAndAssign) matrix: {Version, Batch} left operands
    /// against {Version, Batch} right operands, every reference form, all landing
    /// on the oracle's `meet`. Dual to [`join_assign_matrix_matches_oracle`]. A
    /// `Batch` left operand commits on drop, so it is scoped and its underlying
    /// `Version` checked afterward.
    #[test]
    fn meet_assign_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() & vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version &= Version / &Version.
        { let mut x = a.clone(); x &= b.clone(); prop_assert!(x == expected); }
        { let mut x = a.clone(); x &= &b; prop_assert!(x == expected); }

        // Version &= Batch / &Batch.
        { let mut x = a.clone(); let mut bb = b.clone(); x &= bb.batch(); prop_assert!(x == expected); }
        { let mut x = a.clone(); let mut bb = b.clone(); let r = bb.batch(); x &= &r; prop_assert!(x == expected); }

        // Batch &= Version / &Version (committed on drop).
        { let mut x = a.clone(); { let mut bx = x.batch(); bx &= b.clone(); } prop_assert!(x == expected); }
        { let mut x = a.clone(); { let mut bx = x.batch(); bx &= &b; } prop_assert!(x == expected); }

        // Batch &= Batch / &Batch (committed on drop).
        { let mut x = a.clone(); let mut bb = b.clone();
          { let mut bx = x.batch(); bx &= bb.batch(); } prop_assert!(x == expected); }
        { let mut x = a.clone(); let mut bb = b.clone(); let r = bb.batch();
          { let mut bx = x.batch(); bx &= &r; } prop_assert!(x == expected); }
    }
}

proptest! {
    /// The meet matrix holds once the `Batch` operands are *materialized* to
    /// working form, exercising `snapshot`'s repack and `meet_view` meeting a
    /// Working-form incoming view (the fresh-batch cells above all read packed
    /// views). Dual to [`materialized_join_parity`]; materializing with the join
    /// identity (`Version::new()`) forces `work = Some(..)` without changing the
    /// value.
    #[test]
    fn materialized_meet_parity(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() & vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // `&Batch & &Batch`, both materialized: Working snapshot met with a
        // Working incoming view.
        {
            let mut aa = a.clone();
            let mut bb = b.clone();
            let mut la = aa.batch();
            la.join(&Version::new());
            let mut rb = bb.batch();
            rb.join(&Version::new());
            prop_assert!(&la & &rb == expected);
        }

        // `Version &= &Batch` with the batch materialized.
        {
            let mut x = a.clone();
            let mut bb = b.clone();
            let mut rb = bb.batch();
            rb.join(&Version::new());
            x &= &rb;
            prop_assert!(x == expected);
        }
    }
}

proptest! {
    /// The join lattice laws on impl values: upper bound, least upper bound,
    /// commutative/associative/idempotent, identity, and absorbing.
    #[test]
    fn lattice_laws(ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let c = from_oracle_version(&vs[k % n]);

        let ab = a.clone() | b.clone();
        prop_assert!(le(&a, &ab) && le(&b, &ab)); // upper bound

        // Least upper bound: any common upper bound dominates a|b.
        let upper = ab.clone() | c.clone();
        prop_assert!(le(&a, &upper) && le(&b, &upper));
        prop_assert!(le(&ab, &upper));

        prop_assert!(ab == (b.clone() | a.clone())); // commutative
        let lhs = (a.clone() | b.clone()) | c.clone();
        let rhs = a.clone() | (b.clone() | c.clone());
        prop_assert!(lhs == rhs); // associative
        prop_assert!((a.clone() | a.clone()) == a); // idempotent

        prop_assert!((Version::new() | a.clone()) == a); // identity

        if le(&a, &b) {
            prop_assert!((a.clone() | b.clone()) == b); // absorbing
        }
    }
}

proptest! {
    /// The meet semilattice laws on impl values: lower bound, greatest lower
    /// bound, commutative/associative/idempotent, bottom absorbing, and the two
    /// lattice absorption laws tying `&` to `|`. Dual to [`lattice_laws`].
    #[test]
    fn meet_lattice_laws(ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let c = from_oracle_version(&vs[k % n]);

        let ab = a.clone() & b.clone();
        prop_assert!(le(&ab, &a) && le(&ab, &b)); // lower bound

        // Greatest lower bound: any common lower bound is dominated by a&b.
        let lower = ab.clone() & c.clone();
        prop_assert!(le(&lower, &a) && le(&lower, &b));
        prop_assert!(le(&lower, &ab));

        prop_assert!(ab == (b.clone() & a.clone())); // commutative
        let lhs = (a.clone() & b.clone()) & c.clone();
        let rhs = a.clone() & (b.clone() & c.clone());
        prop_assert!(lhs == rhs); // associative
        prop_assert!((a.clone() & a.clone()) == a); // idempotent

        prop_assert!((Version::new() & a.clone()) == Version::new()); // bottom absorbing

        // Absorption ties the two operations into a lattice.
        prop_assert!((a.clone() & (a.clone() | b.clone())) == a); // a & (a|b) == a
        prop_assert!((a.clone() | (a.clone() & b.clone())) == a); // a | (a&b) == a

        if le(&a, &b) {
            prop_assert!((a.clone() & b.clone()) == a); // a<=b ⇒ a&b == a
        }
    }
}

proptest! {
    /// `tick` strictly advances the causal order: `a < a.tick(p)`.
    #[test]
    fn monotone_tick(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (party, version) = cs[i % n].trees();
        let a = from_oracle_version(version);
        let p = from_oracle_party(party);

        let mut b = a.clone();
        b.tick(&p);
        prop_assert!(le(&a, &b)); // a <= a.tick
        prop_assert!(!le(&b, &a)); // strictly: not a.tick <= a
        prop_assert!(a != b);
    }
}

// ───────────────────────── complexity (linear scaling) ─────────────────────────

proptest! {
    /// Complexity. `tick` is `O(n + m)`: ticking a deep event tree against a deep id of
    /// the same shape (walking both at once) grows linearly with size.
    #[test]
    fn tick_is_linear(shape in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let mut v = shape_version(shape, s);
            let p = shape_party(shape, s);
            steps_of(|| {
                v.tick(&p);
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `grow`'s multi-region cost comparison is `O(n + m)`. Ticking
    /// the empty history (`Leaf(0)`) against a deep *bushy* id forces `grow`
    /// (here `fill` is a no-op: the id is a node over an event leaf), and the
    /// bushy id's many owned regions at varying depths make the probe genuinely
    /// weigh two feasible children at each branch (`cl < cr` with neither a
    /// `COST_MAX` loser). Steps stay linear from `scale` to `4 * scale`.
    #[test]
    fn grow_bushy_is_linear(scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let p = shape_party(Shape::Bushy, s);
            let mut v = Version::new(); // Leaf(0): fill is a no-op, so grow runs
            steps_of(|| {
                v.tick(&p);
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `merge` (`|`) is `O(n + m)`: joining two deep event trees of the same
    /// shape stays linear.
    #[test]
    fn merge_is_linear(shape in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let a = shape_version(shape, s);
            steps_of(|| {
                let _ = a.clone() | a.clone();
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `meet` (`&`) is `O(n + m)`: meeting two deep event trees of
    /// the same shape stays linear, dual to [`merge_is_linear`]. The operands
    /// are independent shapes so the walk genuinely descends both sides (`a & a`
    /// would short-circuit on `trivially_eq`).
    #[test]
    fn meet_is_linear(shape_a in arb_shape(), shape_b in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let a = shape_version(shape_a, s);
            let b = shape_version(shape_b, s);
            steps_of(|| {
                let _ = a.clone() & b.clone();
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

// ───────────────────────────── path-sum overflow regression ─────────────────────────────

/// A normal-form tree whose root-to-leaf path sum exceeds `u64::MAX` compares
/// correctly: with arbitrary-precision base values there is no overflow class,
/// so the answer is `Greater` in every build profile (no debug panic, no
/// release wrap that would invert the causal order). `decode`/`try_from` admit
/// such trees — `parse_ev` validates only *relative* bases and never sums a
/// path — so the comparison must thread the path sum at full precision.
#[test]
fn path_sum_beyond_u64_compares_greater() {
    let big = 1u64 << 63;
    // Normal form: the outer min(big, 0) child is the right `0` leaf; the inner node's
    // min(0, 1) child is its left `0` leaf. The left half's true value is
    // big + big + 1 = 2^64 + 1, past `u64::MAX`.
    let a = Version::try_from((big, (big, 0u64, 1u64), 0u64)).unwrap();
    let b = Version::try_from(big).unwrap(); // constant 2^63
    assert_eq!(a.partial_cmp(&b), Some(Ordering::Greater));
}

/// A stored event base above `u64::MAX` stays exact across mutation and merge.
/// This pins the small-or-big `Base` representation at the spill boundary, not
/// only path sums made from individually-small nodes.
#[test]
fn stored_base_beyond_u64_ticks_and_merges() {
    let big: Version = "18446744073709551616".parse().unwrap();
    let mut ticked = big.clone();
    ticked.tick(&Party::seed());

    assert_eq!(ticked.to_string(), "18446744073709551617");
    assert_eq!(big.clone() | ticked.clone(), ticked);
    assert_eq!(Version::decode(&ticked.encode()[..]).unwrap(), ticked);
}

// ───────────── arbitrary normal-form trees (decoupled from the op pipeline) ─────────────
//
// The op-trace differentials above only ever compare causally *related*
// versions (every member descends from one seed) on the *shapes operations
// produce*. These feed *arbitrary* normal-form event trees — random shape,
// random base magnitudes including values near/beyond `u64::MAX` — to every
// event op and diff structurally against the oracle. They are the natural home
// for the large-base (path-sum-overflow) regression class.

proptest! {
    /// `partial_cmp` on arbitrary, typically *unrelated* event-tree pairs agrees
    /// with the oracle — including the concurrent (`None`) verdict the op pipeline rarely
    /// produces, and large-base pairs whose root-to-leaf path sums exceed `u64::MAX`:
    /// with arbitrary-precision bases the answer must still match.
    #[test]
    fn causal_cmp_arbitrary(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let (ia, ib) = (from_oracle_version(&oa), from_oracle_version(&ob));
        prop_assert_eq!(ia.partial_cmp(&ib), oa.partial_cmp(&ob));
        // Symmetry of the verdict under swap, on the impl directly.
        prop_assert_eq!(
            ib.partial_cmp(&ia),
            ia.partial_cmp(&ib).map(Ordering::reverse)
        );
    }
}

proptest! {
    /// `|` (merge / LUB) on arbitrary unrelated event trees agrees with the
    /// oracle's `join`, structurally. Exercises the join's arm selection on
    /// shapes the op pipeline never builds, with large bases threaded
    /// losslessly.
    #[test]
    fn merge_arbitrary(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let merged = from_oracle_version(&oa) | from_oracle_version(&ob);
        let oracle_join = oa.clone() | ob.clone();
        prop_assert!(merged == from_oracle_version(&oracle_join));
        // The result is a normal-form tree that lowers back to the same oracle value.
        prop_assert_eq!(to_oracle_version(&merged), oracle_join);
    }
}

proptest! {
    /// `&` (meet / GLB) on arbitrary unrelated event trees agrees with the
    /// oracle's `meet`, structurally — dual to [`merge_arbitrary`]. Exercises the
    /// meet's arm selection and `close_node` sink/collapse on shapes the op
    /// pipeline never builds, with large bases threaded losslessly.
    #[test]
    fn meet_arbitrary(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let met = from_oracle_version(&oa) & from_oracle_version(&ob);
        let oracle_meet = oa.clone() & ob.clone();
        prop_assert!(met == from_oracle_version(&oracle_meet));
        // The result is a normal-form tree that lowers back to the same oracle value.
        prop_assert_eq!(to_oracle_version(&met), oracle_meet);
    }
}

proptest! {
    /// `tick` (= `fill` then, if no fill, `grow`) on an arbitrary `(id, event)`
    /// pair with *unrelated* shapes matches the oracle's `event`. This is where
    /// the `Kind` arm selection, the cost folding, and the root-ward tie-break
    /// live; feeding unrelated id/event shapes drives the `fill` full-subtree
    /// arms and the multi-region `grow` cost comparison that same-clock
    /// `(party, version)` pairs under-hit.
    #[test]
    fn tick_arbitrary(
        op in arb_oracle_party_nonempty(),
        ov in arb_oracle_version(),
    ) {
        let mut oracle_after = ov.clone();
        oracle_after.tick(&op);

        let mut iv = from_oracle_version(&ov);
        iv.tick(&from_oracle_party(&op));

        prop_assert!(iv == from_oracle_version(&oracle_after));
    }
}

// ───────────── grow optimality, impl side ─────────────
//
// The defining causality property (§3, §5.3.4): an event registers a *minimal*
// inflation. The oracle's `grow` is pinned to a brute-force search over the
// entire feasible inflation space in `oracle::tests`; these hold the packed
// impl to the same standard. `tick = fill else grow`, so when `fill` already
// simplifies the tree the grow path is not taken — `grow_matches_brute_force`
// filters to the grow case (fill a no-op) and asserts the impl's inflation
// equals the brute-force right-favoring minimum; `grow_minimal` checks the
// paper's metamorphic condition on every `tick`.

proptest! {
    /// When `tick` takes the `grow` branch (`fill` leaves the tree unchanged),
    /// the impl inflates exactly the brute-force cost-minimal, right-favoring
    /// region: `tick` lowered to the oracle equals `best_inflation` normalized.
    /// This holds the packed `grow`'s dynamic program to the full-enumeration
    /// global optimum directly — not merely to the recursive oracle (which
    /// realizes the same DP). Large bases are threaded losslessly, so the cost
    /// comparison is exact regardless of magnitude.
    #[test]
    fn grow_matches_brute_force(
        op in arb_oracle_party_nonempty(),
        ov in arb_oracle_version(),
    ) {
        // Only the grow path is under test: skip inputs where `fill` already
        // simplifies (those are covered by the tick/fill differentials). `fill`
        // is a no-op iff it returns the input unchanged. About a quarter of
        // arbitrary inputs reach grow, comfortably within proptest's reject
        // budget.
        prop_assume!(ov.fill_for_test(&op) == ov);

        let (best_tree, _cost) = best_inflation(&op, &ov).expect("non-empty id inflates");
        let expected = best_tree.normalized_for_test();

        let mut iv = from_oracle_version(&ov);
        iv.tick(&from_oracle_party(&op));

        prop_assert_eq!(to_oracle_version(&iv), expected);
    }
}

proptest! {
    /// §3 (the event condition), metamorphic form, on the impl. When `tick`
    /// takes the `grow` branch, the inflated `e'` "dominates no more than
    /// needed": no feasible single-region inflation candidate `x` of `(id, e)`
    /// satisfies `e ≤ x < e'`. This is the correctly scoped reading of the
    /// paper's `x < e' ⇒ x ≤ e` (the literal form over the dense pointwise
    /// lattice is false even for a single increment — see the oracle twin
    /// `grow_dominates_no_more_than_needed`). Run on the impl's own causal
    /// order, with the candidate set enumerated by the brute-force oracle.
    /// Cross-checked against the oracle order on the same values.
    #[test]
    fn grow_minimal(
        op in arb_oracle_party_nonempty(),
        ov in arb_oracle_version(),
    ) {
        prop_assume!(ov.fill_for_test(&op) == ov);

        let e = from_oracle_version(&ov);
        let mut eprime = e.clone();
        eprime.tick(&from_oracle_party(&op)); // grow path: tick == grow

        for (cand, _) in all_inflations(&op, &ov) {
            let cand_norm = cand.normalized_for_test();
            let cand_v = from_oracle_version(&cand_norm);
            let above_e = le(&e, &cand_v);
            let strictly_below = cand_v.partial_cmp(&eprime) == Some(Ordering::Less);
            prop_assert!(
                !(above_e && strictly_below),
                "an inflation candidate sits strictly between e and e' on the impl",
            );
            // The impl and oracle agree on `e ≤ cand` for each candidate.
            prop_assert_eq!(above_e, oracle_leq(&ov, &cand_norm));
        }
    }
}

proptest! {
    /// `decode ∘ encode == identity` over arbitrary normal-form event trees,
    /// including large-base ones: the widened Elias-gamma code round-trips
    /// every magnitude the working form can hold, and the decoded value lowers
    /// to the same oracle tree.
    #[test]
    fn decode_encode_arbitrary(ov in arb_oracle_version()) {
        let v = from_oracle_version(&ov);
        let bytes = v.encode();
        let decoded = Version::decode(&bytes[..]).expect("canonical encoding decodes");
        prop_assert!(decoded == v);
        prop_assert_eq!(to_oracle_version(&decoded), ov);
    }
}

proptest! {
    /// `as_bytes` returns exactly the canonical `encode` bytes: the stored form
    /// keeps its final partial byte zero-padded, so the raw storage slice is
    /// byte-identical to the packed encoding. Exercises the literal/`extend`
    /// construction path over arbitrary normal-form trees.
    #[test]
    fn as_bytes_matches_encode(ov in arb_oracle_version()) {
        let v = from_oracle_version(&ov);
        let encoded = v.encode();
        prop_assert_eq!(v.as_bytes(), encoded.as_slice());
    }

    /// The invariant survives the repack path too: ticking rebuilds the packed
    /// stream through the working form, which must also leave a zero-padded tail.
    #[test]
    fn as_bytes_matches_encode_after_ticks(n in 0u32..256) {
        let party = Party::seed();
        let mut v = Version::new();
        for _ in 0..n {
            v.tick(&party);
        }
        let encoded = v.encode();
        prop_assert_eq!(v.as_bytes(), encoded.as_slice());
    }
}
