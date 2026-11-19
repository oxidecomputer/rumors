use proptest::prelude::*;

use super::*;

/// One network's event count and identifier bytes; counts are small, so that
/// ties and empty networks occur often.
fn precedence() -> impl Strategy<Value = (u64, [u8; 16])> {
    (0u64..4, any::<[u8; 16]>())
}

/// The other participant.
fn other(participant: Participant) -> Participant {
    match participant {
        Participant::Local => Participant::Remote,
        Participant::Remote => Participant::Local,
    }
}

proptest! {
    /// `yielding_peer` follows both rules of [`Peer`](crate::Peer)'s
    /// "Bootstrapping a network without consensus" section, and the two peers
    /// of one mismatch agree on its answer.
    ///
    /// The section's rule lets the network with the greater event count win,
    /// and the loser yields; its data-preserving variant lets the loser yield
    /// only if its network has no events. The oracle transcribes those rules
    /// from the counts directly. Which network wins a tie is unspecified, so
    /// on a tie the oracle requires only that the general rule names one peer
    /// to yield, and that the data-preserving rule names that same peer
    /// exactly when the tied count is zero. Each peer sees the mismatch from
    /// its own side, so the two peers' answers name the same peer, or neither
    /// does, under either rule.
    #[test]
    fn yielding_peer_follows_the_bootstrapping_rules(
        (local_events, local_id) in precedence(),
        (remote_events, mut remote_id) in precedence(),
        tied in any::<bool>(),
    ) {
        // The networks of a mismatch always differ.
        if remote_id == local_id {
            remote_id[15] ^= 1;
        }
        let remote_events = if tied { local_events } else { remote_events };
        let local = NetworkPrecedence::new(Network::from_bytes(local_id), Count::from(local_events));
        let remote = NetworkPrecedence::new(Network::from_bytes(remote_id), Count::from(remote_events));
        let seen_locally = NetworkMismatch { local: local.clone(), remote: remote.clone() };
        let seen_remotely = NetworkMismatch { local: remote, remote: local };

        let allowed = seen_locally.yielding_peer(DataLoss::Allow);
        let forbidden = seen_locally.yielding_peer(DataLoss::Forbid);
        if local_events == remote_events {
            let loser = allowed.expect("one peer yields a tie when data loss is allowed");
            prop_assert_eq!(forbidden, (local_events == 0).then_some(loser));
        } else {
            let (loser, loser_events) = if local_events < remote_events {
                (Participant::Local, local_events)
            } else {
                (Participant::Remote, remote_events)
            };
            prop_assert_eq!(allowed, Some(loser));
            prop_assert_eq!(forbidden, (loser_events == 0).then_some(loser));
        }
        for (data_loss, answer) in [(DataLoss::Allow, allowed), (DataLoss::Forbid, forbidden)] {
            prop_assert_eq!(
                seen_remotely.yielding_peer(data_loss),
                answer.map(other),
                "the remote peer's answer names the same peer under {:?}",
                data_loss
            );
        }
        prop_assert_eq!(DataLoss::default(), DataLoss::Forbid);
    }
}
