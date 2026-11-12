use rumors::{FileBookmark, Rumors, TokioBlocking};

use crate::table::Update;

use super::Replica;

impl Replica {
    pub(crate) fn into_rumors(self) -> Rumors<Update, FileBookmark<TokioBlocking>> {
        self.rumors
    }
}
