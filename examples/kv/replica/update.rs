use rumors::EncodeError;

use crate::table::Update;

use super::Replica;

impl Replica {
    pub(crate) fn update(&self, updates: &[Update]) -> Result<(), EncodeError> {
        self.rumors.send_all(updates.iter().cloned())?;
        self.catch_up();
        Ok(())
    }
}
