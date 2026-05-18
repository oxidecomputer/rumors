use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};

use message::Message;
use tree::{Action, Tree, mirror};

mod imbl_borsh;
mod message;
mod tree;
mod version;

#[derive(Clone, Debug)]
pub struct Local<T>(Tree<T>);

#[derive(Clone, Debug)]
pub struct Remote<R, W>(pub R, pub W);

pub use tree::Key;

pub use version::Version;

impl<T: BorshDeserialize + BorshSerialize> Local<T> {
    pub fn for_party(party: impl AsRef<[u8]>) -> Self {
        Local(Tree::for_party(party))
    }

    pub fn messages(
        &mut self,
        messages: impl IntoIterator<Item = T>,
        mut on_message: impl FnMut(Key, &Version, &Arc<T>),
    ) {
        self.0.act(
            messages.into_iter().map(Message::from).map(Action::Insert),
            |v, k, m| {
                m.as_ref().map(|m| on_message(k, v, m.as_ref()));
            },
        );
    }

    pub fn garbage(&mut self, garbage: impl IntoIterator<Item = Key>) {
        self.0
            .act(garbage.into_iter().map(Action::Forget), |_, _, _| {});
    }

    pub fn process(&mut self, other: Local<T>, on_message: impl FnMut(Key, &Version, &Arc<T>)) {}
}
