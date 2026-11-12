use rumors::error::Mismatch;
use rumors::{Count, FileBookmark, TokioBlocking};

pub(super) enum GiveWay {
    Us,
    Them,
}

pub(super) fn give_way(error: &rumors::Error<FileBookmark<TokioBlocking>>) -> Option<GiveWay> {
    let rumors::Error::Mismatch(Mismatch::Network {
        local_network,
        local_min_events,
        remote_network,
        remote_min_events,
        ..
    }) = error
    else {
        return None;
    };

    let ours = (local_min_events, local_network);
    let theirs = (remote_min_events, remote_network);
    if *ours.min(theirs).0 != Count::ZERO {
        None
    } else if ours < theirs {
        Some(GiveWay::Us)
    } else {
        Some(GiveWay::Them)
    }
}
