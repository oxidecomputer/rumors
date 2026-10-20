//! Checks the shard merge's refusals that no well-formed sweep reaches.

use super::super::compare::BuildStamp;
use super::super::currency::{ByCurrency, Liveness};
use super::super::measure::Sample;
use super::super::ops::ops;
use super::{emit_sample, header_prefix, max_useful_shards, merge_samples};
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
/// A fuel reading counts the allocator's instructions too, so a cell
/// measured after other cells in the same instance reads what they left
/// behind; only [`max_useful_shards`] shards give every cell its own
/// instance. The refusal fires before the merge's completeness check, so a
/// single well-formed cell line from one shard suffices to reach it.
#[test]
#[should_panic(expected = "fuel readings from 1 shards")]
fn merge_refuses_fuel_readings_from_shared_instances() {
    assert_ne!(
        max_useful_shards(),
        1,
        "the probe needs a deal other than one per cell"
    );
    let scale = 1.0;
    let op = ops()[0].name;
    let family = FamilyId::board()
        .next()
        .expect("the board has a family")
        .name();
    let mut capture = Vec::new();
    capture.extend_from_slice(
        format!("{}{}\n", header_prefix(scale, 0, 1), BuildStamp::current()).as_bytes(),
    );
    capture.extend_from_slice(format!("cell\t{op}\t{family}").as_bytes());
    emit_sample(&mut capture, &fuel_sample(5)).expect("writing to a Vec succeeds");
    emit_sample(&mut capture, &fuel_sample(9)).expect("writing to a Vec succeeds");
    capture.extend_from_slice(b"\nend 1\n");
    merge_samples(scale, 1, &[capture]);
}
