//! Behavioral specification of the reply/frame adapter.
//!
//! [`properties`] states the adapter's laws and sweeps the complete type-level
//! height ladder. [`malformed`] pins the smaller set of wire shapes which must
//! be rejected before those laws can apply. [`opening`] covers the one
//! deliberately exceptional reply in the protocol.

use before::Version;

use crate::{
    message::Message,
    tree::typed::{Hash, Path},
};

mod malformed;
mod opening;
mod properties;

fn hash(byte: u8) -> Hash {
    Hash([byte; 16])
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
    message: Message<u64>,
}

impl LeafCase {
    fn new(value: u64, ticks: u8) -> Self {
        Self {
            value,
            version: Version::try_from(u64::from(ticks)).expect("u8 is a valid linear version"),
            message: Message::new(value),
        }
    }

    fn path(&self) -> Path {
        Path::for_leaf(&self.version, self.message.as_slice())
    }
}
