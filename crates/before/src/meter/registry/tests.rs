//! Checks consistency between the family registry and its derived tables.

use std::collections::BTreeSet;

use super::{Bands, Coverage, FamilyId, Shape, AXIS_BANDS};

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

/// Band citations are unique across the family specs and the axis-band table,
/// and every `Priced` roster is nonempty: one band name resolves to exactly one
/// registry answer, and a family claiming bands must name at least one.
#[test]
fn band_citations_are_unique_and_nonempty() {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for family in FamilyId::ALL {
        match family.spec().bands {
            Bands::Priced(bands) => {
                assert!(
                    !bands.is_empty(),
                    "{family:?} claims priced bands but names none"
                );
                for band in bands {
                    assert!(
                        seen.insert(band),
                        "band `{band}` is cited twice across the registry"
                    );
                }
            }
            Bands::Unbanded { reason, .. } => {
                assert!(
                    !reason.is_empty(),
                    "{family:?} carries an empty band reason"
                );
            }
        }
    }
    for (band, disposition) in AXIS_BANDS {
        assert!(
            seen.insert(band),
            "axis band `{band}` is also cited by a family: drop one entry"
        );
        assert!(
            !disposition.is_empty(),
            "axis band `{band}` carries an empty disposition"
        );
    }
}

/// The board roster is exactly the families answering `Board`.
///
/// Its declared bundle reach is nonzero everywhere and the roster is nonempty:
/// the filter every board sweep derives its family axis from cannot silently go
/// empty or carry a zero-reach column.
#[test]
fn board_roster_derives_from_coverage_answers() {
    let mut columns = 0usize;
    for family in FamilyId::board() {
        match family.spec().coverage {
            Coverage::Board { cells } => {
                assert!(
                    cells > 0,
                    "{family:?} declares a board column with zero reach"
                );
                columns += 1;
            }
            Coverage::EnvelopeOnly { .. } => {
                panic!("{family:?} is envelope-only but appears in the board roster")
            }
        }
    }
    assert_eq!(
        columns,
        FamilyId::ALL
            .iter()
            .filter(|f| matches!(f.spec().coverage, Coverage::Board { .. }))
            .count(),
        "the board roster and the coverage answers disagree"
    );
    assert!(columns > 0, "the board roster is empty");
}

/// Every envelope-only ruling carries a dated, non-empty reason: NA is an
/// explicit answer, never an omission.
#[test]
fn envelope_only_rulings_are_dated() {
    for family in FamilyId::ALL {
        if let Coverage::EnvelopeOnly { reason, decided } = family.spec().coverage {
            assert!(
                !reason.is_empty(),
                "{family:?} has an empty envelope-only reason"
            );
            assert!(
                decided.len() == 10 && decided.chars().filter(|&c| c == '-').count() == 2,
                "{family:?}'s ruling date `{decided}` is not a YYYY-MM-DD date"
            );
        }
        if let Bands::Unbanded { decided, .. } = family.spec().bands {
            assert!(
                decided.len() == 10 && decided.chars().filter(|&c| c == '-').count() == 2,
                "{family:?}'s band ruling date `{decided}` is not a YYYY-MM-DD date"
            );
        }
    }
}
