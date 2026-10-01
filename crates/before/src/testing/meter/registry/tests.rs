//! Checks consistency between the family registry and its derived tables.

use std::collections::BTreeSet;

use super::{FamilyId, Shape};

/// Every shape constructor, for the citation pin below.
///
/// The complete shape set used to check registry citations.
const ALL_SHAPES: [Shape; 68] = [
    Shape::Dense,
    Shape::Bigroot,
    Shape::Hugeleaf,
    Shape::CliffComb,
    Shape::JumpComb,
    Shape::WideToothComb,
    Shape::CliffFan,
    Shape::CancellingChain,
    Shape::Harmonic,
    Shape::AltSpine,
    Shape::ScatteredId,
    Shape::IdSpine,
    Shape::NestedFullId,
    Shape::NestedLeftFullId,
    Shape::WideTail,
    Shape::Staircase,
    Shape::MemoChain,
    Shape::MemoChainId,
    Shape::MemoComb,
    Shape::MemoCombId,
    Shape::MemoFanout,
    Shape::MemoOscillating,
    Shape::MemoChurn,
    Shape::MemoChurnId,
    Shape::DescendingRaises,
    Shape::DescendingRaisesId,
    Shape::RevealComb,
    Shape::RevealCombHifloor,
    Shape::RevealCombId,
    Shape::PureComb,
    Shape::PureCombId,
    Shape::AscendCliff,
    Shape::AscendCliffPlateau,
    Shape::AscendCliffId,
    Shape::DominatedUndercut,
    Shape::DominatedUndercutId,
    Shape::SeamPlunge,
    Shape::SeamPlungeControl,
    Shape::SeamStop,
    Shape::SeamStopControl,
    Shape::LatentLadder,
    Shape::FreezePosition,
    Shape::PromotionRearm,
    Shape::PromotionRearmMate,
    Shape::DenseSuffix,
    Shape::DenseSuffixMate,
    Shape::WideArming,
    Shape::HoistedWindow,
    Shape::WeightComb,
    Shape::FreezeParade,
    Shape::LoneFreeze,
    Shape::ToothTail,
    Shape::PunctureProduct,
    Shape::PlateauPuncture,
    Shape::ArmingTrain,
    Shape::JumpPair,
    Shape::ConcurrentPair,
    Shape::StaggerComb,
    Shape::StaggerId,
    Shape::StaggerPopulation,
    Shape::MeetShade,
    Shape::MaskDriftTriple,
    Shape::MaskDriftQuadruple,
    Shape::CollapseHole,
    Shape::CopyHole,
    Shape::RaiseHole,
    Shape::SiteHole,
    Shape::MaskedHoleTriple,
];

/// The roster array and the per-variant index agree: every family sits in
/// [`FamilyId::ALL`] at its committed position, so the roster order every
/// instrument derives (board render order included) is pinned.
#[test]
fn roster_order_is_committed() {
    for (i, family) in FamilyId::ALL.iter().enumerate() {
        assert_eq!(
            family.index(),
            i,
            "{family:?} sits at roster position {i} but declares index {}",
            family.index()
        );
    }
}

/// Family names are unique so reports cannot merge two families.
#[test]
fn family_names_are_unique() {
    let names: BTreeSet<&str> = FamilyId::ALL.iter().map(|f| f.name()).collect();
    assert_eq!(
        names.len(),
        FamilyId::ALL.len(),
        "two families share a name of record"
    );
}

/// Every shape constructor is cited by at least one family's spec.
///
/// Every shape constructor appears in at least one family specification.
#[test]
fn every_shape_is_cited_by_a_family() {
    let cited: BTreeSet<Shape> = FamilyId::ALL
        .iter()
        .flat_map(|f| f.spec().shapes.iter().copied())
        .collect();
    for shape in ALL_SHAPES {
        assert!(
            cited.contains(&shape),
            "{shape:?} is a registered constructor no family cites: add it to its \
             family's spec (or add the family)"
        );
    }
}

/// Its declared bundle reach is nonzero everywhere and the roster is nonempty:
/// the board cannot silently lose its family axis or carry a zero-reach column.
#[test]
fn every_family_reaches_board_operations() {
    let mut columns = 0usize;
    for family in FamilyId::board() {
        assert!(
            family.spec().cells > 0,
            "{family:?} declares a board column with zero reach"
        );
        columns += 1;
    }
    assert_eq!(
        columns,
        FamilyId::ALL.len(),
        "the board roster is incomplete"
    );
}
