//! Test-only splitting of every reply a consumer takes at every reaction
//! boundary.
//!
//! A reply may arrive in chunks, and every consumer must act on each chunk
//! as it comes. Inside [`with_split_replies`], every request stream a walk
//! stage or a proxy encoder takes ([`Requests::erase`](super::Requests::erase))
//! hands on each chunk as one chunk per reaction, the most chunks a reply
//! can take, so a session run there exercises every consumer's handling of
//! chunks at every boundary. The split happens where the consumer takes a
//! chunk, so no producer waits any longer for its chunks to be taken. The
//! streams of a session built on this thread read the switch, so it covers
//! both endpoints of a session driven here.

use std::cell::Cell;
use std::pin::Pin;

use futures::{Stream, StreamExt, stream};

use crate::tree::mirror::streaming::erased::Reply;

// Clippy's `missing_const_for_thread_local` can reject const-block
// initializers on targets that lower `thread_local!` through fallback TLS.
// The allow keeps `-D warnings` clean on those targets.
std::thread_local! {
    /// Whether consumers built on this thread split their replies.
    #[allow(clippy::missing_const_for_thread_local)]
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// Run `f` with every reply taken by a consumer built on this thread split
/// at every reaction boundary.
pub(crate) fn with_split_replies<R>(f: impl FnOnce() -> R) -> R {
    /// Restore the enclosing setting even when `f` panics.
    struct Restore(bool);

    /// Reinstate the setting displaced by this scope.
    impl Drop for Restore {
        /// Write the saved setting back.
        fn drop(&mut self) {
            ACTIVE.with(|active| active.set(self.0));
        }
    }

    let _restore = Restore(ACTIVE.with(|active| active.replace(true)));
    f()
}

/// `requests`, split at every reaction boundary when the switch is on as
/// the consumer is built.
pub(super) fn requests<E: Send + 'static>(
    requests: impl Stream<Item = Reply<E>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Reply<E>> + Send>> {
    if ACTIVE.with(Cell::get) {
        Box::pin(requests.flat_map(|chunk| stream::iter(reactions(chunk))))
    } else {
        Box::pin(requests)
    }
}

/// `chunk` as one chunk per reaction, the last ending the reply if `chunk`
/// does and carrying its charge and its record's charge, which belongs to
/// the question a walk's chunk ends at; an empty chunk stays one empty
/// chunk.
fn reactions<E>(chunk: Reply<E>) -> Vec<Reply<E>> {
    let Reply {
        reactions,
        charge,
        last,
        record,
    } = chunk;
    if reactions.is_empty() {
        return vec![Reply {
            reactions,
            charge,
            last,
            record,
        }];
    }
    let count = reactions.len();
    let mut ending = Some((charge, record));
    reactions
        .into_iter()
        .enumerate()
        .map(|(index, reaction)| {
            let ends = index + 1 == count;
            let (charge, record) = if ends {
                ending.take().unwrap_or_default()
            } else {
                Default::default()
            };
            Reply {
                reactions: vec![reaction],
                charge,
                last: last && ends,
                record,
            }
        })
        .collect()
}
