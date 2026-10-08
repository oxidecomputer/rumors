
/// The largest depth of the parties [`small_families`] draws from.
const SMALL_PARTY_DEPTH: usize = 2;

/// The longest family [`small_families`] builds.
const SMALL_FAMILY_LEN: usize = 3;

/// Every sequence of at most [`SMALL_FAMILY_LEN`] non-empty normal-form
/// parties of depth at most [`SMALL_PARTY_DEPTH`], repeats included.
fn small_families() -> Vec<Vec<tree::Party>> {
    let ids: Vec<tree::Party> = all_normal_ids(SMALL_PARTY_DEPTH)
        .into_iter()
        .filter(|id| *id != tree::Party::Leaf(false))
        .collect();
    let mut families = vec![Vec::new()];
    let mut longest = families.clone();
    for _ in 0..SMALL_FAMILY_LEN {
        longest = longest
            .iter()
            .flat_map(|family: &Vec<tree::Party>| {
                ids.iter().map(move |id| {
                    let mut grown = family.clone();
                    grown.push(id.clone());
                    grown
                })
            })
            .collect();
        families.extend(longest.iter().cloned());
    }
    families
}

/// The laws' multiplicity comparison agrees with the tree oracle's on every
/// pair of small families, in both orders.
///
/// The property test above checks one weaker comparison per constructed
/// family, so a comparison combining two weaker checks can pass it: equal
/// total measure and equal most-owned points together accept `[A ∪ B ∪ C,
/// A ∪ B, A]` against `[A ∪ B ∪ C, A ∪ C, A]` for disjoint quarters `A`, `B`,
/// and `C`. The small families reach every multiplicity of at most
/// [`SMALL_FAMILY_LEN`] at each quarter of the interval. The sweep groups them
/// by the laws' layers: each family must match its group's first member under
/// both comparisons, and the first members of any two groups must differ
/// under both.
#[test]
fn same_multiplicity_agrees_with_the_tree_oracle_on_small_families() {
    let families = small_families();
    let mut first_members: Vec<Vec<&tree::Party>> = Vec::new();
    let mut groups: HashMap<Vec<Party>, usize> = HashMap::new();
    for family in &families {
        let family: Vec<&tree::Party> = family.iter().collect();
        let lifted: Vec<Party> = family.iter().map(|p| from_oracle_party(p)).collect();
        let layers = owner_layers(&lifted.iter().collect::<Vec<_>>())
            .expect("each remainder is disjoint from its layer");
        let group = *groups.entry(layers).or_insert_with(|| {
            first_members.push(family.clone());
            first_members.len() - 1
        });
        assert_eq!(
            both_verdicts(&family, &first_members[group]),
            (true, true),
            "{family:?} has the layers of {:?}",
            first_members[group],
        );
    }
    for lhs in &first_members {
        for rhs in &first_members {
            if !core::ptr::eq(lhs, rhs) {
                assert_eq!(both_verdicts(lhs, rhs), (false, false), "{lhs:?} against {rhs:?}");
            }
        }
    }
}
