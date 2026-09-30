//! Laws of variadic version and span operations.
//!
//! These laws compare balanced folds with sequential definitions and exercise
//! every relevant list arity and ordering.

use super::*;

laws! {
    /// Laws over a list of versions, at any arity.
    ///
    /// The seedless iterator join entry points (`Sum` and `FromIterator`, owned
    /// and borrowed) against their sequential pair-operator oracle, and their
    /// order-independence. The list length is the quantified variable no
    /// fixed-arity group can reach: the drivers sweep it across every
    /// structural boundary of the balanced counter the folds run on — the
    /// identity and lone-input short-circuits, the first leaf combine, the
    /// closing drain, and the merged–merged carries that first fire at arity
    /// four — so no combine arm sits beyond the suite's reach under any future
    /// reshaping of the fold.
    pub static VERSION_LIST: (xs: &[Version]);

    /// `sum`/`collect` are the sequential pair fold: at every arity, every
    /// seedless iterator entry point equals `|` folded left-to-right from the
    /// identity ([`Version::new`]).
    ///
    /// The right-hand side is the bound pair operator, never a fold entry
    /// point, so the two sides cannot share a broken combine arm; the balanced
    /// regrouping inside the entry points is exactly what the equation
    /// quantifies away. At arity zero the equation *is* the empty edge (the
    /// empty sum is the empty version), at one the lone input, at two the pair
    /// operator itself.
    fn version_sum_is_the_sequential_pair_fold {
        let sequential = xs.iter().fold(Version::new(), |acc, x| &acc | x);
        xs.iter().sum::<Version>() == sequential
            && xs.iter().cloned().sum::<Version>() == sequential
            && xs.iter().collect::<Version>() == sequential
            && xs.iter().cloned().collect::<Version>() == sequential
    }

    /// The seedless join entry points are order-independent at every arity: every
    /// rotation and the reversal of the list fold to the same join.
    ///
    /// Each rotation hands the balanced counter a different grouping of the
    /// same population (which elements coalesce at which weights depends only
    /// on arrival order), so an arm that favors one grouping diverges from the
    /// rest of the orbit. Together with the sequential-fold law and the pair
    /// operator's commutativity and associativity, the full permutation orbit
    /// is pinned.
    fn version_sum_is_order_invariant {
        let join: Version = xs.iter().sum();
        xs.iter().rev().sum::<Version>() == join
            && (1..xs.len()).all(|r| xs[r..].iter().chain(&xs[..r]).sum::<Version>() == join)
    }
}

// ──────────────────── Version: a receiver and items ────────────────────

laws! {
    /// Laws over a version (the receiver) and a list of versions (the items),
    /// at any arity.
    ///
    /// The shape the receiver-seeded folds ([`Version::join_all`],
    /// [`Version::meet_all`], [`Version::span_all`]) quantify over: the
    /// receiver is the guaranteed first element that keeps each fold total, so
    /// the family under law is `{receiver} ∪ items` — never empty. The item
    /// count is swept across the same fold boundaries as the [`VERSION_LIST`]
    /// laws', the reach no fixed-arity group has.
    pub static VERSION_AND_LIST: (receiver: &Version, items: &[Version]);

    /// `join_all` is the sequential pair fold: at every arity, the operation
    /// equals `|` folded left-to-right from the receiver.
    ///
    /// The right-hand side is the bound pair operator, not `join_all`,
    /// so the two sides cannot share a broken combine arm; the balanced
    /// regrouping inside the entry point is exactly what the equation quantifies
    /// away. At zero items the equation *is* the lone-input edge (the join of
    /// the receiver alone is the receiver), at one item the pair operator
    /// itself.
    fn join_all_is_the_sequential_pair_fold {
        receiver.join_all(items) == items.iter().fold(receiver.clone(), |acc, x| &acc | x)
    }

    /// `meet_all` is the sequential pair fold: at every arity, the operation
    /// equals `&` folded left-to-right from the receiver — total at every
    /// arity, because the receiver seeds the identityless meet.
    fn meet_all_is_the_sequential_pair_fold {
        receiver.meet_all(items) == items.iter().fold(receiver.clone(), |acc, x| &acc & x)
    }

    /// The n-ary lattice folds are rotation-independent at every arity: which
    /// element rides as the receiver is irrelevant, and so is item order.
    ///
    /// Every rotation of the family — each element taking one turn as the
    /// receiver, the rest following in rotated order — and the reversal fold
    /// to the same join and the same meet. Each rotation hands the balanced
    /// counter a different grouping of the same population (which elements
    /// coalesce at which weights depends only on arrival order), so an arm
    /// that favors one grouping diverges from the orbit. Together with the
    /// sequential-fold laws and the pair operators' commutativity and
    /// associativity, the full permutation orbit is pinned.
    fn fold_all_is_rotation_invariant {
        let family: Vec<&Version> = core::iter::once(receiver).chain(items).collect();
        let join = receiver.join_all(items);
        let meet = receiver.meet_all(items);
        let rotations = (1..family.len()).all(|r| {
            let rotated = || family[r + 1..].iter().chain(&family[..r]).copied();
            family[r].join_all(rotated()) == join && family[r].meet_all(rotated()) == meet
        });
        let reversed = {
            let (last, front) = family.split_last().expect("the receiver is always present");
            last.join_all(front.iter().rev().copied()) == join
                && last.meet_all(front.iter().rev().copied()) == meet
        };
        rotations && reversed
    }

    /// The n-ary span at every arity: endpoints definitionally the n-ary meet
    /// and join over `{receiver} ∪ items`, every input within.
    ///
    /// The endpoints are [`Version::meet_all`] and [`Version::join_all`] over
    /// the same family — the accessors read exactly them back — and every input
    /// places within the hull: never [`Before`](Placement::Before) or
    /// [`After`](Placement::After), since the meet bounds each input from below
    /// and the join from above. At zero items the family is the receiver alone
    /// and the hull is the coincident `[receiver, receiver]`; at one item it is
    /// the pair hull (`span_is_the_pair_hull` pins those same edges from the
    /// binary entry point's side). The hull fold carries both lattice directions
    /// through one balanced counter, so a combine arm that reads the wrong
    /// endpoint of a merged group breaks exactly one side of this equation at
    /// exactly the arities that reach the arm.
    fn span_all_is_the_family_hull {
        let hull = receiver.span_all(items);
        let family = || core::iter::once(receiver).chain(items);
        let meet = receiver.meet_all(items);
        let join = receiver.join_all(items);
        let definitional = hull == Span::new(&meet, &join).unwrap();
        let accessors = *hull.lo() == meet && *hull.hi() == join;
        let contained =
            family().all(|v| !matches!(hull.place(v), Placement::Before | Placement::After));
        definitional && accessors && contained
    }

    /// The n-ary span is rotation-independent at every arity: which element
    /// rides as the receiver is irrelevant, and so is item order.
    ///
    /// Every rotation of the family — each element taking one turn as the
    /// receiver, the rest following in rotated order — and the reversal build
    /// the same hull. Each rotation regroups the hull fold's balanced counter
    /// differently, so an arm wrong under one grouping diverges from the orbit.
    fn span_all_is_rotation_invariant {
        let family: Vec<&Version> = core::iter::once(receiver).chain(items).collect();
        let hull = receiver.span_all(items);
        let rotations = (1..family.len()).all(|r| {
            let items = family[r + 1..].iter().chain(&family[..r]).copied();
            family[r].span_all(items) == hull
        });
        let reversed = {
            let (last, front) = family.split_last().expect("the receiver is always present");
            last.span_all(front.iter().rev().copied()) == hull
        };
        rotations && reversed
    }

    /// The n-ary span entry points are their binary operators folded left-to-right
    /// over `{seed} ∪ items`, at every arity.
    ///
    /// The balanced regrouping inside each entry point is exactly what the equation
    /// quantifies away, and the right-hand sides are the bound binary
    /// operators, never the entry points.
    ///
    /// The containment entry points run from the receiver's coincident span (union)
    /// and from the family hull (intersection — a wide seed keeps the nonempty
    /// path exercised deep into the fold, while disjoint item spans still reach
    /// [`None`]); the pointwise entry points run from the coincident seed.
    fn span_folds_match_the_sequential_operators {
        let seed = receiver.span(receiver);
        let hull = receiver.span_all(items);
        let spans = item_spans(items);
        let union = seed.union_all(&spans) == spans.iter().fold(seed.clone(), |acc, s| &acc + s);
        // The sequential reference folds *through* `Option` with no early exit,
        // deliberately: the entry point defers its verdict to the end, and the
        // equation quantifies over the same completed fold (`try_fold` would
        // exit at the first `None` — a different reference).
        #[allow(clippy::manual_try_fold)]
        let intersect = hull.intersect_all(&spans)
            == spans
                .iter()
                .fold(Some(hull.clone()), |acc, s| acc.and_then(|a| &a * s));
        let join = seed.join_all(&spans) == spans.iter().fold(seed.clone(), |acc, s| &acc | s);
        let meet = seed.meet_all(&spans) == spans.iter().fold(seed.clone(), |acc, s| &acc & s);
        union && intersect && join && meet
    }

    /// The n-ary span entry points are item-order-independent at every arity: every
    /// rotation and the reversal of the item list fold to the same span (or the
    /// same [`None`]).
    ///
    /// Each rotation regroups every entry point's balanced counter differently, so a
    /// combine arm wrong under one grouping diverges from the orbit — the
    /// span-entry point instance of `fold_all_is_rotation_invariant`.
    fn span_folds_are_rotation_invariant {
        let seed = receiver.span(receiver);
        let hull = receiver.span_all(items);
        let spans = item_spans(items);
        let union = seed.union_all(&spans);
        let intersect = hull.intersect_all(&spans);
        let join = seed.join_all(&spans);
        let meet = seed.meet_all(&spans);
        let agrees = |ordered: &mut dyn Iterator<Item = &Span<'static>>| {
            let ordered: Vec<&Span<'static>> = ordered.collect();
            seed.union_all(ordered.iter().copied()) == union
                && hull.intersect_all(ordered.iter().copied()) == intersect
                && seed.join_all(ordered.iter().copied()) == join
                && seed.meet_all(ordered.iter().copied()) == meet
        };
        agrees(&mut spans.iter().rev())
            && (1..spans.len()).all(|r| agrees(&mut spans[r..].iter().chain(&spans[..r])))
    }

    /// A union of coincident spans is the version hull: on points the
    /// containment entry point restricts to [`Version::span_all`] exactly, so
    /// the two entry points can never drift apart on the shapes both serve.
    fn span_union_of_points_is_span_all {
        let points: Vec<Span<'static>> = items.iter().map(|v| v.span(v)).collect();
        receiver.span(receiver).union_all(&points) == receiver.span_all(items)
    }

    /// Summing or collecting an iterator of spans is the union fold.
    ///
    /// Both collection forms equal the receiver-seeded n-ary union over the
    /// same inputs, owned and borrowed alike, and the empty iterator yields
    /// [`None`] (union has no identity span). Version items are their
    /// coincident point spans, so a collected iterator of versions is the
    /// hull of the whole collection.
    fn span_sum_and_collect_are_the_union_fold {
        let seed = receiver.span(receiver);
        let spans = item_spans(items);
        let expected = Some(seed.union_all(&spans));
        let inputs = || core::iter::once(&seed).chain(&spans);
        let sum: Option<Span> = inputs().sum();
        let collected: Option<Span> = inputs().collect();
        let owned: Option<Span> = inputs().map(Span::clone).sum();
        let empty: Option<Span> = core::iter::empty::<&Span>().sum();
        // Version items are their coincident point spans, so summing or
        // collecting versions is the hull of the whole collection —
        // borrowed and owned items alike.
        let hull = Some(receiver.span_all(items));
        let versions = || core::iter::once(receiver).chain(items);
        let by_ref: Option<Span> = versions().sum();
        let by_value: Option<Span> = versions().cloned().collect();
        sum == expected
            && collected == expected
            && owned == expected
            && empty.is_none()
            && by_ref == hull
            && by_value == hull
    }

    /// Multiplying out an iterator of spans is the intersection fold.
    ///
    /// The `Product` entry point equals the receiver-seeded n-ary intersection
    /// over the same inputs, owned and borrowed alike, and the empty iterator
    /// yields [`None`] (intersection has no identity span).
    fn span_product_is_the_intersect_fold {
        let hull = receiver.span_all(items);
        let spans = item_spans(items);
        let expected = hull.intersect_all(&spans);
        let inputs = || core::iter::once(&hull).chain(&spans);
        let product: Option<Span> = inputs().product();
        let owned: Option<Span> = inputs().map(Span::clone).product();
        let empty: Option<Span> = core::iter::empty::<&Span>().product();
        product == expected && owned == expected && empty.is_none()
    }
}

/// The item spans the fold-entry point laws quantify over: a deterministic mix of
/// coincident and wide spans over the items.
///
/// The mix drives the entry points' point and wide combine arms alike, at every
/// counter boundary the list sweep reaches.
fn item_spans(items: &[Version]) -> Vec<Span<'static>> {
    items
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i % 2 == 0 {
                v.span(v)
            } else {
                items[i - 1].span(v)
            }
        })
        .collect()
}

// ───────────────────────────── Party: one value ─────────────────────────────
