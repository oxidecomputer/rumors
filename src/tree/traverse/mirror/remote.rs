//! Wire-bound counterpart to [`super::local`].
//!
//! Where `local::Exchange` realizes the protocol trait family by traversing an
//! in-memory zipper, `remote::Exchange<P, T, R, W, H>` realizes it as a proxy
//! of the *counterparty*: each protocol method serializes its incoming request
//! into the writer and deserializes the counterparty's response from the
//! reader. The struct carries only a paired `(reader, writer)` plus a phantom
//! tag pinning the protocol height: all of the actual state lives on the
//! counterparty's side of the wire.
//!
//! # Direction
//!
//! When the local responder calls `b.exchange(m)` on its remote-initiator
//! proxy `b`, the `request` `m` is *our* outgoing message --- written to the
//! wire --- and the return is the remote initiator's response, read back. The
//! per-trait table:
//!
//! | Trait                 | Self height | Writes to wire       | Reads from wire      |
//! |-----------------------|-------------|----------------------|----------------------|
//! | [`Initiator`]         | `Root`      | --                   | [`Initiate`]         |
//! | [`Responder`]         | `Root`      | [`Initiate`]         | [`Opening`]          |
//! | [`OpenInitiator`]     | `Root`      | [`Opening`]          | [`Exchange<U^2>`]    |
//! | [`Exchange`]          | `S<S<H>>`   | [`Exchange<S<H>>`]   | [`Exchange<H>`]      |
//! | [`CloseInitiator`]    | `S<S<Z>>`   | [`Exchange<S<Z>>`]   | [`Closing`]          |
//! | [`CompleteResponder`] | `S<Z>`      | [`Closing`]          | [`Complete`]         |
//! | [`CompleteInitiator`] | `Z`         | [`Complete`]         | --                   |
//!
//! [`Initiate`]: message::Initiate
//! [`Opening`]: message::Opening
//! [`Exchange<U^2>`]: message::Exchange
//! [`Exchange<S<H>>`]: message::Exchange
//! [`Exchange<H>`]: message::Exchange
//! [`Exchange<S<Z>>`]: message::Exchange
//! [`Closing`]: message::Closing
//! [`Complete`]: message::Complete
//!
//! # In-band termination
//!
//! The protocol's own emptiness predicate drives session termination: a side
//! has converged when its outgoing message has `requested.is_empty() &&
//! uncertain.is_empty()`. Each protocol method reads its response, inspects
//! the appropriate predicate (per the table in [`super::protocol`]), and
//! yields [`Step::Continue`] or [`Step::Done`] accordingly. The stream is
//! never closed by the protocol itself: a `(reader, writer)` pair can host
//! multiple back-to-back sync sessions.

use std::convert::Infallible;
use std::marker::PhantomData;

use borsh::io::{Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::tree::typed::{
    Node,
    height::{Height, Root, S, Z},
};

use super::message::{self, UnderRoot, UnderUnderRoot};
use super::protocol::{self, Step};

/// Errors raised by [`Exchange`]'s protocol-trait impls. Covers both I/O
/// failures on the underlying reader/writer and framing errors surfaced by
/// borsh during deserialization (out-of-range counts, non-canonical orderings,
/// etc.).
#[derive(Debug)]
pub enum Error {
    /// An underlying reader/writer error, or a borsh framing error encountered
    /// while parsing a message off the wire.
    Io(borsh::io::Error),
}

impl From<borsh::io::Error> for Error {
    fn from(e: borsh::io::Error) -> Self {
        Error::Io(e)
    }
}

/// A wire-bound proxy of the counterparty at protocol height `H`. Holds the
/// underlying reader/writer and a phantom tag pinning the height; the
/// counterparty's actual zipper lives on the far side of the wire.
pub struct Exchange<P, T, R, W, H: Height> {
    reader: R,
    writer: W,
    #[allow(clippy::type_complexity)]
    _phantom: PhantomData<fn() -> (P, T, H)>,
}

impl<P, T, R, W> Exchange<P, T, R, W, Root> {
    /// Wrap a `(reader, writer)` pair as a [`Exchange`], ready to start
    /// the protocol.
    pub fn start(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            _phantom: PhantomData,
        }
    }
}

impl<P, T, R, W, H: Height> Exchange<P, T, R, W, H> {
    /// Wrap a `(reader, writer)` pair as a [`Exchange`].
    fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            _phantom: PhantomData,
        }
    }
}

impl<P, T, R, W, H: Height> protocol::Stage for Exchange<P, T, R, W, H> {
    type Height = H;
    /// The reconciled tree lives on the local side; the proxy yields no value.
    type Output = ();
    type Error = Error;
}

// One protocol-trait impl block per trait, each at the specific height it
// pertains to. Together with the [`protocol::AfterExchange`] blanket impls,
// they discharge every transition in the protocol's height schedule.

impl<P, T, R, W> protocol::Initiator<P, T> for Exchange<P, T, R, W, Root>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: Read,
    W: Write,
    Node<P, T, UnderRoot>: BorshDeserialize,
{
    type Next = Exchange<P, T, R, W, Root>;

    fn initiator(mut self) -> Result<Step<message::Initiate, Self::Next, Infallible>, Error> {
        // No write: the real initiator (on the far side of the wire) has
        // already shipped its `Initiate` and we are reading it now.
        let msg = message::Initiate::deserialize_reader(&mut self.reader).map_err(Error::Io)?;
        // `Initiator::initiator` is statically `Continue`: the `Output` slot
        // is `Infallible`, so `Done` is uninhabitable here.
        Ok(Step::Continue {
            msg,
            next: Exchange::new(self.reader, self.writer),
        })
    }
}

impl<P, T, R, W> protocol::Responder<P, T> for Exchange<P, T, R, W, Root>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: Read,
    W: Write,
    Node<P, T, UnderRoot>: BorshDeserialize,
{
    type Next = Exchange<P, T, R, W, UnderRoot>;

    fn responder(
        mut self,
        request: message::Initiate,
    ) -> Result<Step<message::Opening, Self::Next, ()>, Error> {
        request.serialize(&mut self.writer).map_err(Error::Io)?;
        self.writer.flush().map_err(Error::Io)?;

        // The responder always emits an `Opening`, possibly empty. We can no
        // longer infer termination from an empty `Opening` alone: it can mean
        // either "the trees are equal" or "the responder has no children but
        // we (the initiator) might still have data to provide." Always
        // `Continue` and let the next stage's `open_initiator` decide.
        let response = message::Opening::deserialize_reader(&mut self.reader).map_err(Error::Io)?;
        Ok(Step::Continue {
            msg: response,
            next: Exchange::new(self.reader, self.writer),
        })
    }
}

impl<P, T, R, W> protocol::OpenInitiator<P, T> for Exchange<P, T, R, W, Root>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: Read,
    W: Write,
    Node<P, T, UnderRoot>: BorshDeserialize,
{
    type Next = Exchange<P, T, R, W, UnderUnderRoot>;

    fn open_initiator(
        mut self,
        request: message::Opening,
    ) -> Result<Step<message::Exchange<P, T, UnderUnderRoot>, Self::Next, ()>, Error> {
        request.serialize(&mut self.writer).map_err(Error::Io)?;
        self.writer.flush().map_err(Error::Io)?;

        // We always await a response: even an empty `Opening` can prompt the
        // counterparty to send back a non-trivial `providing` (the "we have,
        // they lack" Left case when we are the empty side).
        let response =
            message::Exchange::<P, T, UnderUnderRoot>::deserialize_reader(&mut self.reader)
                .map_err(Error::Io)?;

        if response.requested.is_empty() && response.uncertain.is_empty() {
            Ok(Step::Done {
                msg: response,
                output: (),
            })
        } else {
            Ok(Step::Continue {
                msg: response,
                next: Exchange::new(self.reader, self.writer),
            })
        }
    }
}

impl<P, T, R, W, H> protocol::Exchange<P, T> for Exchange<P, T, R, W, S<S<H>>>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: Read,
    W: Write,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    Node<P, T, S<H>>: BorshDeserialize,
    // Assumed at impl-validation time so we don't have to case-analyze `H`
    // here: at use sites `H` is concrete and one of the three blanket impls
    // in `super::protocol` discharges it.
    Exchange<P, T, R, W, H>: protocol::AfterExchange<P, T, H>,
{
    type Next = Exchange<P, T, R, W, H>;

    fn exchange(
        mut self,
        request: message::Exchange<P, T, S<H>>,
    ) -> Result<Step<message::Exchange<P, T, H>, Self::Next, ()>, Error> {
        request.serialize(&mut self.writer).map_err(Error::Io)?;
        self.writer.flush().map_err(Error::Io)?;

        // If the message we just sent will cause the other party to be done,
        // they won't ever respond, so don't await their response.
        if request.requested.is_empty() && request.uncertain.is_empty() {
            return Ok(Step::Done {
                msg: message::Exchange::default(),
                output: (),
            });
        }

        let response = message::Exchange::<P, T, H>::deserialize_reader(&mut self.reader)
            .map_err(Error::Io)?;

        if response.requested.is_empty() && response.uncertain.is_empty() {
            Ok(Step::Done {
                msg: response,
                output: (),
            })
        } else {
            Ok(Step::Continue {
                msg: response,
                next: Exchange::new(self.reader, self.writer),
            })
        }
    }
}

impl<P, T, R, W> protocol::CloseInitiator<P, T> for Exchange<P, T, R, W, S<S<Z>>>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: Read,
    W: Write,
{
    type Next = Exchange<P, T, R, W, Z>;

    fn close_initiator(
        mut self,
        request: message::Exchange<P, T, S<Z>>,
    ) -> Result<Step<message::Closing<P, T>, Self::Next, ()>, Error> {
        request.serialize(&mut self.writer).map_err(Error::Io)?;
        self.writer.flush().map_err(Error::Io)?;

        // If the message we just sent will cause the other party to be done,
        // they won't ever respond, so don't await their response.
        if request.requested.is_empty() && request.uncertain.is_empty() {
            return Ok(Step::Done {
                msg: message::Closing::default(),
                output: (),
            });
        }

        let response =
            message::Closing::<P, T>::deserialize_reader(&mut self.reader).map_err(Error::Io)?;

        // `CloseInitiator` is the protocol's natural endgame: always `Done`.
        Ok(Step::Done {
            msg: response,
            output: (),
        })
    }
}

impl<P, T, R, W> protocol::CompleteResponder<P, T> for Exchange<P, T, R, W, S<Z>>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: Read,
    W: Write,
{
    fn complete_responder(
        mut self,
        request: message::Closing<P, T>,
    ) -> Result<Step<message::Complete<P, T>, Infallible, ()>, Error> {
        request.serialize(&mut self.writer).map_err(Error::Io)?;
        self.writer.flush().map_err(Error::Io)?;

        // If the message we just sent will cause the other party to be done,
        // they won't ever respond, so don't await their response.
        if request.requested.is_empty() {
            return Ok(Step::Done {
                msg: message::Complete::default(),
                output: (),
            });
        }

        let response =
            message::Complete::<P, T>::deserialize_reader(&mut self.reader).map_err(Error::Io)?;

        // `CompleteResponder` is statically `Done`: the `Next` slot is
        // `Infallible`, so `Continue` is uninhabitable here.
        Ok(Step::Done {
            msg: response,
            output: (),
        })
    }
}

impl<P, T, R, W> protocol::CompleteInitiator<P, T> for Exchange<P, T, R, W, Z>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize,
    R: Read,
    W: Write,
{
    fn complete_initiator(
        mut self,
        request: message::Complete<P, T>,
    ) -> Result<Step<(), Infallible, Self::Output>, Error> {
        // Final write; the real initiator absorbs this and is done.
        request.serialize(&mut self.writer).map_err(Error::Io)?;
        self.writer.flush().map_err(Error::Io)?;
        Ok(Step::Done {
            msg: (),
            output: (),
        })
    }
}
