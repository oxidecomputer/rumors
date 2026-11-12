use rumors::UnorderedMessages;

use crate::table::{Table, Update};

pub(crate) struct View {
    pub(crate) updates: UnorderedMessages<Update>,
    pub(crate) table: Table,
}
