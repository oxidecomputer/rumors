//! The registry's own pins: the seams between the roster and its data
//! tables that the compiler cannot check, held by tests instead.

use std::collections::BTreeSet;

use super::{Bands, Coverage, FamilyId, Shape, AXIS_BANDS};

/// Every shape constructor, for the citation pin below.
///
/// Completeness rides the same review discipline as [`FamilyId::ALL`]:
/// a variant missing here escapes only the citation pin, never the
/// compiler ties (its constructor arm in `Shape::builder` is still
/// forced).
const ALL_SHAPES: [Shape; 55] = [
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
    Shape::FreezePosition,
    Shape::PromotionRearm,
    Shape::PromotionRearmMate,
    Shape::DenseSuffix,
    Shape::DenseSuffixMate,
    Shape::WideArming,
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
];

/// The roster array and the per-variant index agree: every family sits
/// in [`FamilyId::ALL`] at its committed position, so the roster order
/// every instrument derives (board render order included) is pinned.
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

/// Family names are unique: the name is the board column header and the
/// bench cell key, so a collision would silently merge two columns.
#[test]
fn family_names_are_unique() {
    let names: BTreeSet<&str> = FamilyId::ALL.iter().map(|f| f.name()).collect();
    assert_eq!(
        names.len(),
        FamilyId::ALL.len(),
        "two families share a name of record"
    );
}

/// Every shape constructor is cited by at least one family's spec — the
/// named parity survivor for shape citation: membership of a [`Shape`]
/// in a family's `shapes` row is data, not types, so this pin is what
/// keeps a constructor from riding the registry door with no family
/// answering for it.
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

/// Band citations are unique across the family specs and the axis-band
/// table, and every `Priced` roster is nonempty: one band name resolves
/// to exactly one registry answer, and a family claiming bands must
/// name at least one.
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

/// The board roster is exactly the families answering `Board`, its
/// declared bundle reach is nonzero everywhere, and the roster is
/// nonempty — the filter every board sweep derives its family axis
/// from cannot silently go empty or carry a zero-reach column.
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

/// Every envelope-only ruling carries a dated, non-empty reason: NA is
/// an explicit answer, never an omission.
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
