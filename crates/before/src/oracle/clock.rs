//! The oracle stamp: [`Clock`], a [`Party`] paired with a [`Version`].

use std::ops::{BitOr, BitOrAssign};

use crate::codec::Base;

use super::{OverlapError, Party, Version};

/// The reference [`Clock`](crate::Clock): the paper's recursive trees,
/// mirroring the optimized type's API one-to-one so the differential tests
/// can drive both with the same script.
///
/// Contracts live on the real type; this one is deliberately naive (and
/// `Clone`, so tests can branch histories the linear type forbids).
#[derive(Clone, Debug)]
pub struct Clock {
    party: Party,
    version: Version,
}

impl Clock {
    pub fn seed() -> Self {
        Self::from_parts(Party::seed(), Version::new())
    }

    pub fn from_parts(party: Party, version: Version) -> Self {
        Clock { party, version }
    }

    pub fn into_parts(self) -> (Party, Version) {
        (self.party, self.version)
    }

    pub fn party(&self) -> &Party {
        &self.party
    }

    pub fn version(&self) -> Version {
        self.version.clone()
    }

    /// `version() / party()`: this clock's own contribution to its version
    /// (the history within the region it owns). The reference for
    /// [`Clock::own_version`](crate::Clock::own_version).
    pub fn own_version(&self) -> Version {
        self.version() / self.party()
    }

    pub fn tick(&mut self) {
        self.version.tick(&self.party);
    }

    pub fn fork(&mut self) -> Clock {
        let child = self.party.fork();
        Clock {
            party: child,
            version: self.version.clone(),
        }
    }

    pub fn join(&mut self, other: Clock) -> Result<(), Clock> {
        let (op, ov) = other.into_parts();
        match self.party.join(op) {
            Ok(()) => {
                self.version |= ov;
                Ok(())
            }
            Err(op) => Err(Clock::from_parts(op, ov)),
        }
    }

    pub fn sync(&mut self, other: &mut Clock) -> Result<(), OverlapError> {
        if !self.party.is_disjoint(&other.party) {
            return Err(OverlapError);
        }
        let theirs = std::mem::replace(&mut other.party, Party::Leaf(false));
        self.party.join(theirs).expect("disjoint, just checked");
        other.party = self.party.fork();
        let merged = self.version.clone() | other.version.clone();
        self.version = merged.clone();
        other.version = merged;
        Ok(())
    }

    pub fn has_seen(&self, msg: &Version) -> bool {
        msg.leq(&Base::ZERO, &self.version, &Base::ZERO)
    }

    pub fn happens_before(&self, other: &Clock) -> bool {
        self.version < other.version
    }

    pub fn concurrent_with(&self, other: &Clock) -> bool {
        self.version.partial_cmp(&other.version).is_none()
    }

    pub fn send(&mut self) -> Version {
        self.tick();
        self.version()
    }

    pub fn receive(&mut self, msg: Version) {
        self.version |= msg;
        self.tick();
    }

    pub fn trees(&self) -> (&Party, &Version) {
        (&self.party, &self.version)
    }
}

impl BitOr<Version> for Clock {
    type Output = Clock;
    fn bitor(mut self, rhs: Version) -> Clock {
        self.version |= rhs;
        self
    }
}

impl BitOr<Clock> for Version {
    type Output = Clock;
    fn bitor(self, mut rhs: Clock) -> Clock {
        rhs.version |= self;
        rhs
    }
}

impl BitOrAssign<Version> for Clock {
    fn bitor_assign(&mut self, rhs: Version) {
        self.version |= rhs;
    }
}
