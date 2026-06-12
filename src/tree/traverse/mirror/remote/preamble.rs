//! The session preamble and the trailing party hand-off frame: the fixed
//! greeting a session leads with, and the one message that can follow the
//! mirror descent.
//!
//! # Preamble
//!
//! Every gossip session begins with a fixed-size preamble, exchanged
//! concurrently by both sides. It rides the same [`framing`](super::framing)
//! as all other traffic — one length-delimited frame — but at a length known
//! in advance:
//!
//! ```text
//! [ length = 25: 4B (big-endian)
//! | magic = b"RUMORS": 6B | version: 2B (big-endian) | network: 16B | intent: 1B ]
//! ```
//!
//! - **Magic** is [`crate::PROTOCOL_MAGIC`] (`b"RUMORS"`). A peer that opens
//!   the connection with anything else is rejected as [`Error::MagicMismatch`];
//!   it isn't speaking the `rumors` protocol at all.
//!
//! - **Version** is [`crate::PROTOCOL_VERSION`], a monotonic `u16`. A peer
//!   whose version differs is rejected as [`Error::VersionMismatch`].
//!
//! - **Network** is the 128-bit universe identifier, doubling as the
//!   bootstrap signal: a real (non-`ZERO`) value means an ordinary peer,
//!   while the all-zero placeholder means that side is
//!   [bootstrapping](crate::Peer::bootstrap) and holds no universe yet.
//!   When both sides carry a real network and the two differ, the session
//!   is rejected as [`Error::NetworkMismatch`]: the peers descend from
//!   different [`seed`](crate::Peer::seed)s and must not combine, even if
//!   their parties happen to look disjoint. A bootstrapping side's
//!   placeholder suppresses that check, and the provider's network becomes
//!   the value the bootstrapper adopts.
//!
//! - **Intent** declares whether the peer participates to remain (`0`) or
//!   to [retire](crate::Peer::retire) its party into us (`1`). Any other
//!   byte is rejected as [`Error::IntentInvalid`], and a peer claiming to
//!   bootstrap *and* retire in one session is rejected as
//!   [`Error::BootstrapRetireConflict`].
//!
//! Although the preamble is framed, its peer-declared length is never
//! *used*: the frame is read at the fixed size via
//! [`FrameRead::frame_exact`], and the declared length is merely validated —
//! after the magic and version, whose mismatches are the better diagnoses —
//! so a garbage peer cannot induce a huge-frame allocation before it has
//! identified itself ([`Error::PreambleLengthInvalid`]).
//!
//! The framed [`message::Handshake`](crate::tree::mirror::message::Handshake)
//! greeting that follows carries the
//! causal [`Version`](crate::Version) alone.
//!
//! Both sides drive the preamble's write and read concurrently via
//! [`futures_util::future::try_join`]; a peer that reads before writing would
//! deadlock against another peer doing the same on a connection with a tiny
//! write buffer.

use before::Party;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::network::Network;
use crate::tree::mirror::message::Intent;

use super::framing::{FrameRead, FrameWrite};
use super::{Error, recv_msg, send_msg};

/// Exchange and validate the protocol preamble frame
/// `[len(4 BE) | magic(6) | proto_version(2 BE) | network(16) | intent(1)]`
/// with a peer, before any peer-declared frame length is trusted.
///
/// This runs before the
/// [`message::Handshake`](crate::tree::mirror::message::Handshake) body and
/// the rest of the protocol, and reads the preamble frame at its *known*
/// size ([`FrameRead::frame_exact`]) rather than the peer-declared one, so a
/// non-`rumors` peer (wrong magic) or an incompatible one (wrong version) is
/// rejected before the framing ever trusts a peer-supplied length: a garbage
/// peer cannot induce a huge-frame allocation.
///
/// Both sides write and read concurrently via [`futures_util::future::try_join`]:
/// a peer that reads before writing would deadlock against another doing the
/// same on a transport whose write buffer is smaller than the preamble.
/// ([`FrameWrite::frame`] flushes, which the same concurrency relies on: the
/// peer reads our preamble before sending anything further.)
///
/// Returns [`Error::MagicMismatch`] when the peer's magic bytes are not
/// [`crate::PROTOCOL_MAGIC`], [`Error::VersionMismatch`] when the magic
/// matches but the version does not, and [`Error::PreambleLengthInvalid`]
/// when magic and version both match but the frame declares the wrong
/// length.
pub async fn preamble<R, W>(
    network: Network,
    intent: Intent,
    reader: &mut FrameRead<R>,
    writer: &mut FrameWrite<W>,
) -> Result<(Network, Intent), Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    // Preamble payload: [magic(6) | proto_version(2 BE) | network(16) | intent(1)].
    const PREAMBLE_LEN: usize = 6 + 2 + 16 + 1;

    let mut local = [0u8; PREAMBLE_LEN];
    local[..6].copy_from_slice(&crate::PROTOCOL_MAGIC);
    local[6..8].copy_from_slice(&crate::PROTOCOL_VERSION.to_be_bytes());
    local[8..24].copy_from_slice(&network.to_bytes());
    local[24] = if intent.retiring() { 1 } else { 0 };

    let write_fut = async { writer.frame(&local).await.map_err(Error::Io) };
    let read_fut = async {
        reader
            .frame_exact::<PREAMBLE_LEN>()
            .await
            .map_err(Error::Io)
    };
    let ((), (declared, remote)) = futures_util::future::try_join(write_fut, read_fut).await?;

    // Validation order is diagnosis order: magic identifies the protocol,
    // version identifies the dialect, and only then is the frame length held
    // to this dialect's fixed size — so a future, longer preamble is
    // reported as the version mismatch it is, not as a malformed frame.
    let remote_magic: [u8; 6] = remote[..6].try_into().expect("6 bytes");
    if remote_magic != crate::PROTOCOL_MAGIC {
        return Err(Error::MagicMismatch { remote_magic });
    }
    let remote_version = u16::from_be_bytes([remote[6], remote[7]]);
    if remote_version != crate::PROTOCOL_VERSION {
        return Err(Error::VersionMismatch { remote_version });
    }
    if declared as usize != PREAMBLE_LEN {
        return Err(Error::PreambleLengthInvalid { declared });
    }
    let remote_network = Network::from_bytes(remote[8..24].try_into().expect("16 bytes"));
    let remote_intent = match remote[24] {
        0 => Intent::Remain,
        1 => Intent::Retire,
        byte => return Err(Error::IntentInvalid { byte }),
    };
    // No honest peer both donates a party (retiring) and receives one
    // (bootstrapping) in a single session; the network and intent are
    // peer-supplied bytes, so the combination must be rejected here rather
    // than assumed away by callers.
    if remote_network.is_bootstrap() && remote_intent.retiring() {
        return Err(Error::BootstrapRetireConflict);
    }
    Ok((remote_network, remote_intent))
}

/// Provider side of the party hand-off that completes bootstrapping a brand-new
/// peer: fork `party` and ship the give-half as one frame, *after* the mirror
/// descent has transferred all content. [`Peer::retire`](crate::Peer::retire)
/// reuses the same trailing frame in the opposite direction: the retiree ships
/// its (whole, aliased) party last, for the absorber to [`recv_party`] and
/// join.
///
/// Bootstrapping is not a separate bulk transfer: a peer holding nothing
/// greets with the placeholder [`Network::ZERO`](crate::Network) and an
/// empty tree, then runs the ordinary
/// [mirror descent](crate::tree::mirror::local), with
/// the empty side pulling all of the provider's content through the usual
/// `providing` channel. The descent moves content but not parties, so one
/// thing remains: the provider must hand the newcomer a
/// [`Party`]. That is this single frame.
///
/// # Ordering
///
/// Forking last means a failure during the (large) descent never costs a
/// party region. If the party frame itself is lost, the provider must assume
/// it could still have been received: it is not safe to reclaim the forked
/// party, and if the frame was in fact not received, the party permanently
/// leaks out of the system. No acknowledgement could shrink that residual
/// window to zero, because a lost final message leaves the provider unable
/// to tell "peer got the party" from "peer did not" (the two-generals
/// problem); forking last is the structural minimum, and it costs no extra
/// round-trip. [`Party::fork`](before::Party::fork) splits the identifier
/// space without ticking the clock, so a party frame lost in that window
/// costs only a slice of the id space, never causal correctness: the
/// provider's retained half stays a valid, disjoint party.
///
/// The frame travels on the same [`FrameWrite`] the descent used (surfaced
/// back to the caller as the remote exchange's output), the stream's single
/// owner throughout the session.
pub(crate) async fn send_party<W>(give: Party, writer: &mut FrameWrite<W>) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
{
    send_msg(writer, &give).await
}

/// Receiving side of the hand-off: read the party the peer ships after the
/// descent (a bootstrap provider's fork, or a retiree's whole party), off the
/// same reader the descent used. See [`send_party`] for why it sits last.
pub(crate) async fn recv_party<R>(reader: &mut FrameRead<R>) -> Result<Party, Error>
where
    R: AsyncRead + Unpin,
{
    recv_msg(reader).await
}
