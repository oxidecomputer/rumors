//! The end-of-stream check every decoder runs once its question queue
//! closes.

use super::reject_extra;
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::channel::{QueueKind, QueueRole, channel};
use crate::tree::mirror::streaming::remote::codec::{End, Frame};
use crate::tree::mirror::streaming::remote::proxy::Error;

/// A decoder that received no question reports a frame already waiting on
/// its stream as an unasked reply, and its closed channel refuses every
/// later frame, so no unasked frame is dropped unseen.
#[test]
fn an_unquestioned_decoder_reports_a_waiting_frame() {
    let (mut frames, mut incoming) = channel(QueueRole::new(QueueKind::PipeIncoming, 0), 1);
    run_to_quiescence(async {
        frames.send(Frame::End(End::Reply)).await.unwrap();
        let result = reject_extra::<()>(false, &mut incoming).await;
        assert!(matches!(result, Err(Error::UnaskedReply)), "{result:?}");
        assert!(
            frames.send(Frame::End(End::Reply)).await.is_err(),
            "the closed channel refuses later frames"
        );
    })
    .expect("the check never waits");
}

/// A decoder that received no question finishes at once on an empty
/// stream, and its closed channel refuses a frame that arrives afterward.
#[test]
fn an_unquestioned_decoder_finishes_on_an_empty_stream() {
    let (mut frames, mut incoming) = channel(QueueRole::new(QueueKind::PipeIncoming, 0), 1);
    run_to_quiescence(async {
        let result = reject_extra::<()>(false, &mut incoming).await;
        assert!(result.is_ok(), "{result:?}");
        assert!(
            frames.send(Frame::End(End::Reply)).await.is_err(),
            "the closed channel refuses a frame that arrives afterward"
        );
    })
    .expect("the check never waits");
}
