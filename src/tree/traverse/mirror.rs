//! Bidirectional alternating mirror-sync between two replicas of the typed tree.

mod local;
mod message;
pub mod protocol;

#[cfg(test)]
mod test;
