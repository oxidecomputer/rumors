use crate::{Party, Span, Version};

/// Encode two concurrent versions of equal length: one shared history, plus
/// one more tick on a different share in each.
///
/// Each share's count is a distinct large power of two, so one more tick
/// changes no stored width. Both versions are far wider than the shared
/// reference count a buffer's first clone allocates, so a freed input buffer
/// and that count never compete for one freed block.
fn concurrent_twins() -> (Vec<u8>, Vec<u8>) {
    let shares: [Party; 8] = Party::seed().into();
    let mut history = Version::new();
    for (j, share) in (0u32..).zip(&shares) {
        share.ticks(&mut history, 1u128 << (64 + 8 * j));
    }
    let one_more = |share: &Party| {
        let mut version = history.clone();
        share.tick(&mut version);
        version.encode()
    };
    let (e, f) = (one_more(&shares[1]), one_more(&shares[3]));
    assert_eq!(e.len(), f.len(), "one more tick changes no stored width");
    (e, f)
}

/// Every fold that drops duplicate inputs keeps a new owned input whose buffer
/// lands at the address of an input the fold has already dropped.
///
/// Each fold reads `[e, e, f]`, every input decoded into its own buffer only
/// when the fold pulls it, so nothing outside the fold keeps an input's buffer
/// alive. `f` is concurrent with `e` and encodes to the same length. Combining
/// `e` with its equal twin keeps one buffer and frees the other, and the
/// allocator typically places `f`'s buffer at that freed address. A duplicate
/// filter that compared `f` against the address of an input it no longer
/// holds would discard `f` as a clone of `e`; every fold must include `f`.
///
/// The hazard needs the allocator to reuse the address, which this test cannot
/// force; it reports a filter that compares against a dropped input wherever
/// the allocator does reuse it.
#[test]
fn owned_inputs_at_a_freed_address_are_not_duplicates() {
    let (e, f) = concurrent_twins();
    let decode = |bytes: &[u8]| Version::decode(bytes).expect("fixture bytes are canonical");
    let inputs = || [&e, &e, &f].into_iter().map(|bytes| decode(bytes));
    let (ve, vf) = (decode(&e), decode(&f));
    assert!(ve.concurrent(&vf), "dropping f must change every fold");
    let (join, meet, hull) = (&ve | &vf, &ve & &vf, ve.span(&vf));
    let point = Span::at(&ve);

    let folds = [
        ("Sum", inputs().sum::<Version>() == join),
        ("FromIterator", inputs().collect::<Version>() == join),
        ("Version::join_all", ve.join_all(inputs()) == join),
        ("Version::meet_all", ve.meet_all(inputs()) == meet),
        ("Version::span_all", ve.span_all(inputs()) == hull),
        ("Span::union_all", point.union_all(inputs()) == hull),
        (
            "Span::join_all",
            point.join_all(inputs()) == Span::at(&join),
        ),
        (
            "Span::meet_all",
            point.meet_all(inputs()) == Span::at(&meet),
        ),
        (
            "Span::intersect_all",
            point.intersect_all(inputs().map(Span::from)).is_none(),
        ),
    ];
    let wrong: Vec<&str> = folds
        .iter()
        .filter(|(_, right)| !right)
        .map(|(name, _)| *name)
        .collect();
    assert!(
        wrong.is_empty(),
        "these folds of [e, e, f] differ from the fold of [e, f], as when a \
         duplicate filter drops f: {wrong:?}",
    );
}
