//! Behavioral specification of the reply/frame adapter.
//!
//! [`properties`] states the adapter's laws and sweeps the complete type-level
//! height ladder. [`malformed`] pins the smaller set of wire shapes which must
//! be rejected before those laws can apply. [`opening`] covers the one
//! deliberately exceptional reply in the protocol. [`runs`] states the
//! supply-run batching contract the byte budget imposes on the encoder.
//! [`parking`] pins the memory accounting that makes a parked decoded reply
//! O(fan) handles rather than a subtree. [`fan_occupancy`] pins the
//! reader/assembler channel's occupancy ceiling — the supply-decode
//! envelope's charge premise.

use before::Version;

use crate::{
    message::Message,
    tree::{
        mirror::streaming::{materialized::SupplyLedger, remote::codec::LeafRun},
        typed::{Hash, Path, hash::MERKLE_HASH_LEN},
    },
};

mod backend_errors;
mod fan_occupancy;
mod malformed;
mod opening;
mod parking;
mod properties;
mod runs;

fn hash(byte: u8) -> Hash {
    Hash([byte; MERKLE_HASH_LEN])
}

/// A set-length allowance no fixture here can exhaust, for tests whose
/// subject is not the ingress supply charge.
fn unbounded() -> SupplyLedger {
    SupplyLedger::new(u64::MAX)
}

/// Build a supply run from borrowed leaf records, in the given order.
fn leaf_run(records: &[(&Version, &Message)]) -> LeafRun {
    let mut run = LeafRun::new();
    for (version, message) in records {
        run.push(version, message)
            .expect("a test record fits the run framing");
    }
    run
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("test runtime")
}

#[derive(Clone, Debug)]
struct LeafCase {
    value: u64,
    version: Version,
    message: Message,
}

impl LeafCase {
    /// A deterministic test leaf: the version scalar folds `value` and
    /// `ticks` together so distinct cases produce distinct versions — the
    /// axis paths derive from — while `value` also picks the payload.
    fn new(value: u64, ticks: u8) -> Self {
        Self {
            value,
            version: Version::try_from(value.wrapping_shl(8) | u64::from(ticks))
                .expect("every u64 scalar is a valid linear version"),
            message: Message::new(value),
        }
    }

    fn path(&self) -> Path {
        Path::for_leaf(&self.version)
    }
}
