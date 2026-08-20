use tokio::io::AsyncReadExt;

use super::{CountedRead, Recorder};

/// The received-byte counter adds exactly the bytes each poll delivers,
/// split deliveries into one partially filled buffer included.
///
/// The delta is measured against the buffer's fill level before the
/// poll, never its absolute level after it: a `read_exact` that fills
/// the same buffer across two polls counts each delivery once.
#[test]
fn counted_read_counts_split_deliveries() {
    let recorder = Recorder::default();
    let mut read = CountedRead::new((&b"ab"[..]).chain(&b"c"[..]), recorder.clone());
    let mut buf = [0u8; 3];
    pollster::block_on(read.read_exact(&mut buf)).expect("three bytes arrive");
    assert_eq!(&buf, b"abc");
    assert_eq!(recorder.snapshot().bytes_received, 3);
}
