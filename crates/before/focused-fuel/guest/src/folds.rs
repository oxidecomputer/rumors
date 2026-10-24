//! Probes of the balanced folds over populations whose merges stay as large
//! as the inputs they carry, so a fold's logarithmic factor shows.
//!
//! The staggered population `SG(n, 64)` (`Shape::StaggerPopulation`) joins
//! maximally interleaving region sets at every balanced merge; its ids are
//! pairwise disjoint. Each probe's units are the population's encoded bytes.

use before::testing::meter::registry::Shape;
use before::{Clock, Party, Version};

use crate::measured_per;

/// The blocks each staggered operand carries, held fixed while the
/// population grows so its bytes grow nearly linearly with its size.
const TEETH: usize = 64;

/// Builds the staggered versions and ids of `size` members.
fn stagger(size: usize) -> (Vec<Version>, Vec<Party>) {
    let (versions, ids) = Shape::StaggerPopulation.population(size, TEETH);
    (
        versions.iter().map(|encoded| encoded.version()).collect(),
        ids.iter()
            .map(|encoded| Party::decode(&encoded.bytes[..]).expect("generated ids are canonical"))
            .collect(),
    )
}

/// The encoded bytes of `versions`.
fn bytes(versions: &[Version]) -> u64 {
    versions
        .iter()
        .map(|version| version.encode().len() as u64)
        .sum()
}

/// Measures `join_all` over the staggered versions of `size` members.
pub fn join_all(size: usize) -> u64 {
    let (mut versions, _) = stagger(size);
    let units = bytes(&versions);
    let receiver = versions.remove(0);
    measured_per(
        units,
        || receiver.join_all(&versions),
        |join| {
            assert!(
                versions.iter().all(|member| *member <= join),
                "join covers all"
            )
        },
    )
}

/// Measures `meet_all` over the staggered notch population of `size`
/// members.
///
/// Member `i` is the unit plateau projected onto the complement of the
/// staggered id `i`, so it carries notches of height zero exactly over that
/// id's blocks. A meet keeps every operand's notches, so the balanced merges
/// grow as the stagger joins do.
pub fn meet_all(size: usize) -> u64 {
    let (_, ids) = stagger(size);
    let mut plateau = Version::new();
    Party::seed().tick(&mut plateau);
    let mut notches: Vec<Version> = ids
        .iter()
        .map(|id| {
            let rest = Party::seed()
                .without(id)
                .expect("a staggered id is a proper subregion of the seed");
            (&plateau / &rest).to_version()
        })
        .collect();
    let units = bytes(&notches);
    let receiver = notches.remove(0);
    measured_per(
        units,
        || receiver.meet_all(&notches),
        |meet| {
            assert!(
                notches.iter().all(|member| meet <= *member),
                "meet lies below all"
            )
        },
    )
}

/// Measures `span_all` over the staggered versions of `size` members.
pub fn span_all(size: usize) -> u64 {
    let (versions, _) = stagger(size);
    let units = bytes(&versions);
    let (receiver, items) = versions.split_first().expect("the population is nonempty");
    let join = receiver.join_all(items);
    measured_per(
        units,
        || receiver.span_all(items),
        |span| assert_eq!(span.hi(), &join, "the hull's end is the join"),
    )
}

/// Measures `Party::join_all` over the staggered ids of `size` members.
pub fn party_join_all(size: usize) -> u64 {
    let (_, mut ids) = stagger(size);
    let units = ids.iter().map(|id| id.encode().len() as u64).sum();
    let mut receiver = ids.remove(0);
    measured_per(
        units,
        || {
            let joined = receiver.join_all(ids).is_ok();
            (joined, receiver)
        },
        |(joined, _)| assert!(joined, "staggered ids are pairwise disjoint"),
    )
}

/// Measures `Clock::join_all` over clocks pairing each staggered id with its
/// staggered version.
pub fn clock_join_all(size: usize) -> u64 {
    let (versions, ids) = stagger(size);
    let mut clocks: Vec<Clock> = ids
        .into_iter()
        .zip(versions)
        .map(|(id, version)| Clock::from_parts(id, version))
        .collect();
    let units = clocks.iter().map(|clock| clock.encode().len() as u64).sum();
    let mut receiver = clocks.remove(0);
    measured_per(
        units,
        || {
            let joined = receiver.join_all(clocks).is_ok();
            (joined, receiver)
        },
        |(joined, _)| assert!(joined, "staggered ids are pairwise disjoint"),
    )
}
