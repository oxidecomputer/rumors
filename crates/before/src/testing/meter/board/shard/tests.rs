//! Checks the shard merge's refusals that no well-formed sweep reaches.

use super::super::compare::BuildStamp;
use super::super::currency::{ByCurrency, Liveness};
use super::super::measure::Sample;
use super::super::ops::ops;
use super::super::sampling::Sampling;
use super::{cell_shard, emit_sample, header_prefix, max_useful_shards, merge_samples, owns};
use crate::testing::meter::registry::FamilyId;

/// A sample of one byte that read `fuel` fuel and nothing else, with no
/// liveness obligation.
fn fuel_sample(fuel: u64) -> Sample {
    Sample {
        denom_bytes: 1,
        exp_denom_bytes: 1,
        floors: ByCurrency::from_fn(|_| Liveness::NotApplicable {
            reason: "no liveness obligation in this probe",
        }),
        models: ByCurrency::from_fn(|_| None),
        readings: ByCurrency {
            heap: None,
            scan: None,
            touch: None,
            fuel: Some(fuel),
        },
    }
}

/// The merge refuses fuel readings from any deal but one cell per shard.
///
/// State outside the heap, such as a lazily initialized static, carries
/// from one cell to the next within an instance, so a cell measured after
/// others in the same instance could read what they left behind; only
/// [`max_useful_shards`] shards give every cell its own instance. The
/// refusal fires before the merge's completeness check, so a single
/// well-formed cell line from one shard suffices to reach it.
#[test]
#[should_panic(expected = "fuel readings from 1 shards")]
fn merge_refuses_fuel_readings_from_shared_instances() {
    assert_ne!(
        max_useful_shards(),
        1,
        "the probe needs a deal other than one per cell"
    );
    let sampling = Sampling::Scaled(1.0);
    let op = ops()[0].name;
    let family = FamilyId::board()
        .next()
        .expect("the board has a family")
        .name();
    let mut capture = Vec::new();
    capture.extend_from_slice(
        format!(
            "{}{}\n",
            header_prefix(sampling, 0, 1),
            BuildStamp::current()
        )
        .as_bytes(),
    );
    capture.extend_from_slice(format!("cell\t{op}\t{family}").as_bytes());
    emit_sample(&mut capture, &fuel_sample(5)).expect("writing to a Vec succeeds");
    emit_sample(&mut capture, &fuel_sample(9)).expect("writing to a Vec succeeds");
    capture.extend_from_slice(b"\nend 1\n");
    merge_samples(sampling, 1, &[capture]);
}

/// In the one-cell deal, [`cell_shard`] names, for every grid cell, the one
/// shard that owns it, and it names no shard for an unknown operation or
/// family.
#[test]
fn cell_shard_names_the_owning_shard() {
    let table = ops();
    let count = max_useful_shards();
    let families: Vec<FamilyId> = FamilyId::board().collect();
    for (family_position, family) in families.iter().enumerate() {
        for (op_position, op) in table.iter().enumerate() {
            let index = cell_shard(op.name, family.name())
                .unwrap_or_else(|| panic!("{} x {} is a grid cell", op.name, family.name()));
            assert!(
                owns(index, count, op_position, family_position, table.len()),
                "shard {index} does not own {} x {}",
                op.name,
                family.name()
            );
        }
    }
    let (op, family) = (table[0].name, families[0].name());
    assert_eq!(cell_shard("no-such-operation", family), None);
    assert_eq!(cell_shard(op, "no-such-family"), None);
}
