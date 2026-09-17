//! Pairing between root-level requests and the initiator's opening supplies.
//!
//! The initiator sends whole root children which the peer's greeting shows are
//! absent, avoiding another request round trip. The responder's walk still
//! issues its requests in radix order. [`OpeningSupplies`] merge-joins those
//! requests with the separately delivered nodes and retains at most one node
//! of lookahead.

use std::{cmp::Ordering, pin::Pin};

use futures::{Stream, StreamExt};

use crate::{
    message::PayloadCodec,
    tree::{
        mirror::streaming::{
            Backend, Leaf,
            materialized::SupplyLedger,
            remote::{
                adapter::{DecodeError, early_supplies},
                proxy::Error,
                streams::StreamReceiver,
            },
        },
        typed::{ErasedPrefix, height::Z},
    },
};

#[cfg(test)]
mod tests;

/// Decoded opening-supply groups in ascending radix order.
type Supplies<B> = Pin<
    Box<
        dyn Stream<Item = Result<(u8, <B as Backend>::Erased), DecodeError<<B as Backend>::Error>>>
            + Send,
    >,
>;

/// The valid states of the opening-supply cursor.
enum State<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The stream has not yet been claimed from the transport.
    Armed(StreamReceiver),
    /// The stream is being merged with root-level requests.
    Streaming {
        /// Decoded supply groups not yet paired with requests.
        supplies: Supplies<B>,
        /// The first group at or beyond the next request.
        lookahead: Option<(u8, B::Erased)>,
    },
    /// The stream ended without another supply group.
    Exhausted,
}

/// Lazily merge the initiator's opening supplies with root-level requests.
///
/// Both inputs are ordered by radix. A supply at the requested radix answers
/// it; a later supply means pruning removed the requested subtree; an earlier
/// supply answers no request and fails the session. The stream remains
/// unclaimed when this level receives no request.
pub(super) struct OpeningSupplies<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The peer's declared maximum encoded version size.
    version_bytes: u64,
    /// The peer's remaining declared supply allowance.
    ledger: SupplyLedger,
    /// The cursor's current transport and pairing state.
    state: State<B>,
    /// The codec used to decode supplied messages.
    codec: PayloadCodec,
}

impl<B> OpeningSupplies<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Create a cursor around an unclaimed opening-supply stream.
    pub(super) fn new(
        version_bytes: u64,
        ledger: SupplyLedger,
        receiver: StreamReceiver,
        codec: PayloadCodec,
    ) -> Self {
        Self {
            version_bytes,
            ledger,
            state: State::Armed(receiver),
            codec,
        }
    }

    /// Resolve `radix` to its supplied node, if pruning retained one.
    pub(super) async fn advance_to(
        &mut self,
        backend: &B,
        root: ErasedPrefix,
        radix: u8,
    ) -> Result<Option<B::Erased>, Error<B::Error>> {
        loop {
            match &mut self.state {
                State::Exhausted => return Ok(None),
                State::Armed(_) => self.begin(backend, root),
                State::Streaming {
                    supplies,
                    lookahead,
                } => {
                    if let Some((next, node)) = lookahead.take() {
                        match next.cmp(&radix) {
                            Ordering::Equal => return Ok(Some(node)),
                            Ordering::Greater => {
                                *lookahead = Some((next, node));
                                return Ok(None);
                            }
                            // This supply group is behind the request cursor,
                            // so no later request can consume it.
                            Ordering::Less => return Err(Error::UnaskedReply),
                        }
                    }
                    match supplies.next().await {
                        Some(item) => *lookahead = Some(item?),
                        None => self.state = State::Exhausted,
                    }
                }
            }
        }
    }

    /// Require every supplied radix group to have answered a request.
    pub(super) async fn finish(&mut self) -> Result<(), Error<B::Error>> {
        let State::Streaming {
            supplies,
            lookahead,
        } = &mut self.state
        else {
            return Ok(());
        };
        if lookahead.is_some() || supplies.next().await.transpose()?.is_some() {
            return Err(Error::UnaskedReply);
        }
        self.state = State::Exhausted;
        Ok(())
    }

    /// Claim and decode the opening-supply stream on its first request.
    fn begin(&mut self, backend: &B, root: ErasedPrefix) {
        let state = std::mem::replace(&mut self.state, State::Exhausted);
        self.state = match state {
            State::Armed(receiver) => State::Streaming {
                supplies: Box::pin(early_supplies::<B, _>(
                    backend.clone(),
                    self.version_bytes,
                    self.ledger.clone(),
                    root,
                    receiver,
                    self.codec,
                )),
                lookahead: None,
            },
            state => state,
        };
    }
}
