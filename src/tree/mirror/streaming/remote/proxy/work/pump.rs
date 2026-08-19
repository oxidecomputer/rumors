//! Work-owned translation between typed reply streams and wire frames.
//!
//! The shape deliberately follows the materialized `Work`: every outbound
//! encoder becomes an independently runnable task, and each method returns the
//! receiver-side stream or next-phase scope queue fed by that task. No state
//! outside this module handles an internal sender.
//!
//! Three channels carry the dataflow:
//!
//! - flushed local questions flow into decoding, sized by the session
//!   [`Window`](crate::tree::mirror::streaming::window::Window);
//! - scopes derived from decoded replies flow into the next phase, also
//!   window-sized;
//! - decoded replies flow outward through [`Work::respond`] on a one-slot
//!   edge, an in-order relay that bounds decoded replies in flight at one
//!   per stage.
//!
//! A complete wire reply precedes its local questions; a decoded reply
//! precedes its dependent scopes. Each edge's capacity rationale lives at
//! its constructor in [`queues`].

use std::pin::Pin;

use async_stream::try_stream;
use futures::{Stream, StreamExt};

use super::Work;
use crate::link::{Acceptor, Connector};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::Receiver,
        convert::Convert,
        materialized::{SupplyLedger, children_of},
        message::{Reaction, Reply},
        protocol::{BoxResponses, Requests},
        remote::{
            adapter::{
                DecodeError, Decoded, Scope, decode_leaf_reply, decode_reply, early_supplies,
                opening_reply,
            },
            proxy::Error,
            streams::{ReceiverFinish, StreamReceiver, StreamSender},
        },
    },
    typed::{
        Prefix,
        height::{Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};

use super::{encode, queues};

impl<B, T, R, W, A> Work<B, T, R, W, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: serde::de::DeserializeOwned + Send + Sync + 'static,
    R: Send,
    W: Send,
    A: Acceptor,
{
    /// Replay the remote initiator's opening question from its greeting.
    ///
    /// The question's content — the remote's root-fan listing — already
    /// crossed inside the greeting, so no wire frame exists at this stage:
    /// the reply and its root scope are synthesized from the retained
    /// listing. The initiator-direction opening stream carries the remote's
    /// early supplies instead; the first descending transition claims and
    /// decodes it against the root-level requests it answers, so the root
    /// merge-join below never waits on the supply bulk.
    pub fn initiator(
        &mut self,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Scope<UnderRoot>>,
    ) {
        let (next_scopes, scopes) =
            queues::next_scopes::<_, UnderRoot>(self.window.capacity(UnderRoot::HEIGHT));
        let progress = self.progress;
        let listing = std::mem::take(&mut self.peer_listing);
        let responses = try_stream! {
            let (reply, scope) = opening_reply(listing);
            yield_reply_scopes!(
                progress, UnderRoot, 1;
                yield reply;
                next_scopes => [scope];
            );
        };
        (self.respond(responses), scopes)
    }

    /// Proxy the responder opening and return its lower scope queue.
    pub fn opening_responder<C: Connector>(
        &mut self,
        requests: impl Requests<B, T, UnderRoot>,
        mut incoming: StreamReceiver<A::Rx, T>,
        outgoing: StreamSender<C, T>,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Scope<UnderUnderRoot>>,
    ) {
        let progress = self.progress;
        let (local_questions, mut questions) =
            queues::local_questions::<_, UnderRoot>(self.window.capacity(UnderRoot::HEIGHT));
        let peer_listing = std::mem::take(&mut self.peer_listing);
        self.spawn(encode::opening(
            self.backend(),
            self.budget,
            requests,
            local_questions,
            outgoing,
            peer_listing,
            progress,
        ));
        let (next_scopes, scopes) =
            queues::next_scopes::<_, UnderUnderRoot>(self.window.capacity(UnderUnderRoot::HEIGHT));
        let backend = self.backend();
        let version_bytes = self.peer_version_bytes;
        let ledger = self.peer_supplies.clone();
        let responses = try_stream! {
            while let Some(scope) = questions.recv().await {
                let Decoded { reply, questions } =
                    decode_reply::<B, T, UnderUnderRoot, _>(
                        backend.clone(), version_bytes, ledger.clone(), scope, &mut incoming,
                    ).await?;
                yield_reply_scopes!(
                    progress, UnderUnderRoot, questions.len();
                    yield reply;
                    next_scopes => questions;
                );
            }
            reject_extra(&mut incoming).await?;
        };
        (self.respond(responses), scopes)
    }

    /// Proxy one ordinary two-height transition and return its lower scopes.
    ///
    /// `early` is the opening-supply stream, armed only on the first
    /// initiator-representing transition: there, each of the local
    /// responder's root-level requests pairs its (empty) wire reply with
    /// the whole node the remote supplied at the opening, exploded into
    /// the per-child supplies the walk absorbs — the same reply shape a
    /// wire-borne answer would have carried.
    pub fn internal_replies<C: Connector, H>(
        &mut self,
        requests: impl Requests<B, T, S<S<H>>>,
        scopes: Receiver<Scope<S<S<H>>>>,
        mut incoming: StreamReceiver<A::Rx, T>,
        outgoing: StreamSender<C, T>,
        early: Option<StreamReceiver<A::Rx, T>>,
    ) -> (
        BoxResponses<B, T, S<H>, Error<B::Error>>,
        Receiver<Scope<H>>,
    )
    where
        H: Height,
        S<H>: Convert,
        S<S<H>>: Convert,
        S<S<S<H>>>: Height,
    {
        let progress = self.progress;
        let (local_questions, mut questions) =
            queues::local_questions::<_, S<H>>(self.window.capacity(<S<H>>::HEIGHT));
        self.spawn(encode::replies(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            local_questions,
            progress,
        ));
        let (next_scopes, scopes) = queues::next_scopes::<_, H>(self.window.capacity(H::HEIGHT));
        let backend = self.backend();
        let version_bytes = self.peer_version_bytes;
        let ledger = self.peer_supplies.clone();
        let responses = try_stream! {
            let mut early = Early::<B, T, S<S<H>>, A::Rx>::new(version_bytes, ledger.clone(), early);
            while let Some(scope) = questions.recv().await {
                if early.armed() && scope.is_request() {
                    // A root-level request: its content crossed at the
                    // opening, so the pairing reply here arrives empty and
                    // the early stream carries the node — or neither does,
                    // when pruning removed the whole subtree.
                    let parent = scope.parent();
                    let Decoded { reply, questions: asked } = decode_reply::<B, T, H, _>(
                        backend.clone(), version_bytes, ledger.clone(), scope, &mut incoming,
                    ).await?;
                    debug_assert!(asked.is_empty(), "an empty request opens no lower scope");
                    let (root, radix) = parent.pop();
                    let mut replies = reply.replies;
                    if let Some(node) = early.advance_to(&backend, root, radix).await? {
                        let children = children_of(&backend, parent, node)
                            .await
                            .map_err(|error| Error::Decode(DecodeError::Backend(error)))?;
                        replies.extend(
                            children
                                .into_iter()
                                .map(|(radix, child)| Reaction::Supply(radix, child)),
                        );
                    }
                    yield_reply_scopes!(
                        progress, H, 0;
                        yield Reply { replies };
                        next_scopes => Vec::<Scope<H>>::new();
                    );
                    continue;
                }
                let Decoded { reply, questions } = decode_reply::<B, T, H, _>(
                    backend.clone(), version_bytes, ledger.clone(), scope, &mut incoming,
                ).await?;
                yield_reply_scopes!(
                    progress, H, questions.len();
                    yield reply;
                    next_scopes => questions;
                );
            }
            early.finish().await?;
            reject_extra(&mut incoming).await?;
        };
        (self.respond(responses), scopes)
    }

    /// Proxy the leaf-parent transition and return its terminal leaf scopes.
    pub fn leaf_replies<C: Connector>(
        &mut self,
        requests: impl Requests<B, T, S<Z>>,
        scopes: Receiver<Scope<S<Z>>>,
        mut incoming: StreamReceiver<A::Rx, T>,
        outgoing: StreamSender<C, T>,
    ) -> (BoxResponses<B, T, Z, Error<B::Error>>, Receiver<Scope<Z>>) {
        let progress = self.progress;
        let (local_questions, mut questions) =
            queues::local_questions::<_, Z>(self.window.capacity(Z::HEIGHT));
        self.spawn(encode::replies(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            local_questions,
            progress,
        ));
        let (next_scopes, scopes) = queues::next_scopes::<_, Z>(self.window.capacity(Z::HEIGHT));
        let backend = self.backend();
        let version_bytes = self.peer_version_bytes;
        let ledger = self.peer_supplies.clone();
        let responses = try_stream! {
            while let Some(scope) = questions.recv().await {
                let Decoded { reply, questions } = decode_leaf_reply(
                    backend.clone(), version_bytes, ledger.clone(), scope, &mut incoming,
                ).await?;
                yield_reply_scopes!(
                    progress, Z, questions.len();
                    yield reply;
                    next_scopes => questions;
                );
            }
            reject_extra(&mut incoming).await?;
        };
        (self.respond(responses), scopes)
    }

    /// Drive the final local answers for a remote initiator to completion.
    pub async fn complete_initiator<C: Connector>(
        self,
        requests: impl Requests<B, T, Z>,
        scopes: Receiver<Scope<Z>>,
        outgoing: StreamSender<C, T>,
    ) -> Result<(R, W), Error<B::Error>> {
        let progress = self.progress;
        let finish = encode::terminal(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            None,
            progress,
        );
        let ((), read, write) = self.execute(finish).await?;
        Ok((read, write))
    }

    /// Drive the responder's final bidirectional leaf exchange.
    pub fn complete_responder<C: Connector>(
        mut self,
        requests: impl Requests<B, T, Z>,
        scopes: Receiver<Scope<Z>>,
        mut incoming: StreamReceiver<A::Rx, T>,
        outgoing: StreamSender<C, T>,
    ) -> (
        BoxResponses<B, T, Z, Error<B::Error>>,
        impl Future<Output = Result<(R, W), Error<B::Error>>> + Send,
    )
    where
        R: Send,
        W: Send,
        A: Send,
    {
        let progress = self.progress;
        let (local_questions, mut questions) =
            queues::local_questions::<_, Z>(self.window.capacity(Z::HEIGHT));
        self.spawn(encode::terminal(
            self.backend(),
            self.budget,
            requests,
            scopes,
            outgoing,
            Some(local_questions),
            progress,
        ));
        let backend = self.backend();
        let version_bytes = self.peer_version_bytes;
        let ledger = self.peer_supplies.clone();
        let responses = try_stream! {
            while let Some(scope) = questions.recv().await {
                let Decoded { reply, questions } = decode_leaf_reply(
                    backend.clone(), version_bytes, ledger.clone(), scope, &mut incoming,
                ).await?;
                if !questions.is_empty() {
                    Err(Error::TerminalQuery)?;
                }
                progress.decoded_reply::<Z>(0);
                yield reply;
            }
            reject_extra(&mut incoming).await?;
        };
        let responses = self.respond(responses);
        let completion = async move {
            let ((), read, write) = self.execute(async { Ok(()) }).await?;
            Ok((read, write))
        };
        (responses, completion)
    }
}

/// The initiator's opening-supply stream, claimed lazily and consumed one
/// radix group at a time against the responder's root-level requests.
///
/// Both sides run in ascending radix order, so a single lookahead slot
/// pairs them: a group ahead of the requested radix means the requested
/// subtree pruned away, a group behind it answers no request and fails the
/// session. An armed cursor whose stage sees no request never polls the
/// receiver, so the transport stream is never claimed — the lazy-claim
/// discipline every level follows.
struct Early<B, T, G, Rx>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    G: Height,
{
    /// The peer's greeting-declared `max_version_bytes`, enforced on
    /// every supplied version the opening stream decodes.
    version_bytes: u64,
    /// The session's declared-`set_len` allowance, charged per record
    /// the opening stream decodes.
    ledger: SupplyLedger,
    receiver: Option<StreamReceiver<Rx, T>>,
    supplies:
        Option<Pin<Box<dyn Stream<Item = Result<(u8, B::Node<G>), DecodeError<B::Error>>> + Send>>>,
    lookahead: Option<(u8, B::Node<G>)>,
    exhausted: bool,
}

impl<B, T, G, Rx> Early<B, T, G, Rx>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: serde::de::DeserializeOwned + Send + Sync + 'static,
    G: Convert,
    S<G>: Height,
    Rx: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    /// Arm the cursor with the opening-supply stream's receiver, if this
    /// stage is the one that owns it.
    fn new(
        version_bytes: u64,
        ledger: SupplyLedger,
        receiver: Option<StreamReceiver<Rx, T>>,
    ) -> Self {
        Self {
            version_bytes,
            ledger,
            receiver,
            supplies: None,
            lookahead: None,
            exhausted: false,
        }
    }

    /// Whether this stage pairs root-level requests with opening supplies.
    fn armed(&self) -> bool {
        self.receiver.is_some() || self.supplies.is_some() || self.lookahead.is_some()
    }

    /// Resolve the request for `radix`: its supplied node, or `None` when
    /// the initiator's pruning left nothing under it.
    async fn advance_to(
        &mut self,
        backend: &B,
        root: Prefix<S<G>>,
        radix: u8,
    ) -> Result<Option<B::Node<G>>, Error<B::Error>> {
        loop {
            if let Some((next, node)) = self.lookahead.take() {
                if next == radix {
                    return Ok(Some(node));
                }
                if next > radix {
                    self.lookahead = Some((next, node));
                    return Ok(None);
                }
                // Behind the request cursor: this group was never asked
                // about at the root, so nothing will ever absorb it.
                return Err(Error::UnaskedReply);
            }
            if self.exhausted {
                return Ok(None);
            }
            let supplies = match &mut self.supplies {
                Some(supplies) => supplies,
                None => {
                    let receiver = self
                        .receiver
                        .take()
                        .expect("an unarmed cursor resolves no request");
                    self.supplies
                        .get_or_insert(Box::pin(early_supplies::<B, T, G, _>(
                            backend.clone(),
                            self.version_bytes,
                            self.ledger.clone(),
                            root,
                            receiver,
                        )))
                }
            };
            match supplies.next().await {
                Some(item) => self.lookahead = Some(item?),
                None => self.exhausted = true,
            }
        }
    }

    /// Require every opening supply to have answered a root-level request.
    async fn finish(&mut self) -> Result<(), Error<B::Error>> {
        if self.lookahead.is_some() {
            return Err(Error::UnaskedReply);
        }
        if let Some(supplies) = &mut self.supplies
            && !self.exhausted
            && supplies.next().await.transpose()?.is_some()
        {
            return Err(Error::UnaskedReply);
        }
        Ok(())
    }
}

/// Require a finished incoming logical stream after all expected replies.
///
/// A stream that was never claimed — its level asked no question — is
/// finished vacuously; a claimed stream must have delivered its end control
/// with no reply to spare.
async fn reject_extra<Rx, T, E>(incoming: &mut StreamReceiver<Rx, T>) -> Result<(), Error<E>>
where
    Rx: tokio::io::AsyncRead + Unpin + Send + 'static,
    T: serde::de::DeserializeOwned + Send + Sync + 'static,
{
    match incoming.finish().await {
        ReceiverFinish::Clean => Ok(()),
        ReceiverFinish::ExtraReply => Err(Error::UnaskedReply),
    }
}
