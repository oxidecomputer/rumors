use proptest::prelude::*;
use rumors::testing::property::divided_proptest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;

/// Runs `future` on a current-thread runtime with I/O enabled.
fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a current-thread runtime builds")
        .block_on(future)
}

/// One established end: its halves and its socket's tally.
type End = (ReadHalf, WriteHalf, Arc<Tally>);

/// Both ends of one loopback connection, established through `transport`'s
/// stack, the accepting end first.
async fn connected(transport: Transport) -> (End, End) {
    connected_warm(transport, None).await
}

/// Both ends of one loopback connection, established through `transport`'s
/// stack after a warm-up of `warm_bytes` each way, if any, the accepting end
/// first.
async fn connected_warm(transport: Transport, warm_bytes: Option<u64>) -> (End, End) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let address = listener.local_addr().expect("a bound address");
    // Each end's TLS handshake makes a large future, larger still in the
    // debug profile. Boxed, neither the join below nor the test's own
    // future holds one inline, so the test thread's stack holds them in
    // every profile.
    let accepted = Box::pin(async {
        let (stream, _) = listener.accept().await.expect("accept");
        establish(stream, transport, Role::Accepted, warm_bytes)
            .await
            .expect("the accepting end establishes")
    });
    let connected = Box::pin(async {
        let stream = TcpStream::connect(address).await.expect("connect");
        establish(stream, transport, Role::Connected, warm_bytes)
            .await
            .expect("the connecting end establishes")
    });
    tokio::join!(accepted, connected)
}

/// Every transport the axes describe.
fn transports() -> impl Strategy<Value = Transport> {
    any::<bool>().prop_map(|tls| Transport { tls })
}

/// The divisor of the case count of a property whose cases each establish a
/// loopback connection, over TLS in about half of them, and move up to tens of
/// kilobytes across it.
///
/// Beyond its bytes, a case costs a TCP handshake on a fresh runtime, and over
/// TLS a key exchange and a signature, so it runs one case per this many of the
/// configured count.
const CONNECTION_DIVISOR: u32 = 16;

divided_proptest! {
    #![divisor(CONNECTION_DIVISOR)]

    /// Over every stack, the bytes one end writes and flushes arrive at the
    /// other end intact and in order.
    ///
    /// The sockets' tallies, started after the handshake, count those
    /// bytes: exactly over plain TCP, and more over TLS, whose records
    /// frame and authenticate them.
    #[test]
    fn every_stack_carries_bytes_intact(
        transport in transports(),
        payload in proptest::collection::vec(any::<u8>(), 1..20_000),
    ) {
        let (received, sender, receiver) = block_on(async {
            let ((mut a_read, _a_write, a_tally), (_b_read, mut b_write, b_tally)) =
                connected(transport).await;
            let write = async {
                b_write.write_all(&payload).await.expect("write");
                b_write.flush().await.expect("flush");
            };
            let mut received = vec![0; payload.len()];
            let read = async { a_read.read_exact(&mut received).await.expect("read") };
            tokio::join!(write, read);
            (received, b_tally.counts(), a_tally.counts())
        });
        prop_assert_eq!(&received, &payload);
        let length = payload.len() as u64;
        if transport.tls {
            prop_assert!(sender.sent_bytes > length, "{sender:?}");
            prop_assert!(receiver.received_bytes > length, "{receiver:?}");
        } else {
            prop_assert_eq!(sender.sent_bytes, length);
            prop_assert_eq!(receiver.received_bytes, length);
        }
    }
}

/// Writes and flushes `sent` while reading the peer's payload, which must equal
/// `expected`.
async fn exchange(read: &mut ReadHalf, write: &mut WriteHalf, sent: &[u8], expected: &[u8]) {
    let send = async {
        write.write_all(sent).await.expect("write");
        write.flush().await.expect("flush");
    };
    let mut received = vec![0; expected.len()];
    let receive = async { read.read_exact(&mut received).await.expect("read") };
    tokio::join!(send, receive);
    assert_eq!(received, expected, "the payload arrives intact");
}

/// The bytes each side's warm-up sends: several of the warm-up's chunks, so it
/// outsizes the read buffer and any TLS read-ahead many times over.
const WARM_BYTES: u64 = 4 * WARM_CHUNK as u64;

/// The most plaintext one TLS record carries (RFC 8446, section 5.1).
const TLS_RECORD_PLAINTEXT: usize = 1 << 14;

/// The least a TLS 1.3 record adds to its plaintext: a five-byte header, the
/// one-byte inner content type, and a sixteen-byte AEAD tag.
const TLS_RECORD_OVERHEAD: usize = 5 + 1 + 16;

divided_proptest! {
    #![divisor(CONNECTION_DIVISOR)]

    /// Over every stack, a warm connection's tallies count everything sent
    /// after the warm-up and nothing of it, even when each side writes the
    /// moment its own setup returns, while the peer may still be finishing
    /// its warm-up.
    ///
    /// Over plain TCP each side's tally counts exactly the payloads; over
    /// TLS, at least each payload and one record's framing per record's
    /// worth of it, and less than the warm-up. A warm-up read that took any
    /// of the peer's payload would count it before the tally started, below
    /// these bounds.
    #[test]
    fn a_warm_tally_counts_only_what_follows_the_warm_up(
        transport in transports(),
        payloads in prop::array::uniform2(proptest::collection::vec(any::<u8>(), 1..20_000)),
    ) {
        let [a_payload, b_payload] = &payloads;
        let (a_counts, b_counts) = block_on(async {
            let ((mut a_read, mut a_write, a_tally), (mut b_read, mut b_write, b_tally)) =
                connected_warm(transport, Some(WARM_BYTES)).await;
            tokio::join!(
                exchange(&mut a_read, &mut a_write, a_payload, b_payload),
                exchange(&mut b_read, &mut b_write, b_payload, a_payload),
            );
            (a_tally.counts(), b_tally.counts())
        });
        for (counts, sent, received) in [
            (a_counts, a_payload, b_payload),
            (b_counts, b_payload, a_payload),
        ] {
            if transport.tls {
                for (counted, payload) in [(counts.sent_bytes, sent), (counts.received_bytes, received)] {
                    let framed = payload.len() + payload.len().div_ceil(TLS_RECORD_PLAINTEXT) * TLS_RECORD_OVERHEAD;
                    prop_assert!(counted >= framed as u64, "{counts:?}");
                    prop_assert!(counted < WARM_BYTES, "{counts:?}");
                }
            } else {
                prop_assert_eq!(counts.sent_bytes, sent.len() as u64);
                prop_assert_eq!(counts.received_bytes, received.len() as u64);
            }
        }
    }
}

/// A vectored write of a few small slices on a plain TCP stack reaches the
/// socket as one write operation.
///
/// The counting wrapper forwards vectored writes and reports the socket as
/// vectored, so a write the library hands over as slices reaches the kernel as
/// one system call, not one per slice.
#[test]
fn a_vectored_write_is_one_socket_write() {
    let counts = block_on(async {
        let (_accepted, (_b_read, mut b_write, b_tally)) =
            connected(Transport { tls: false }).await;
        assert!(
            b_write.is_write_vectored(),
            "the unbuffered TCP write half is vectored"
        );
        let slices = [
            IoSlice::new(b"one"),
            IoSlice::new(b"two"),
            IoSlice::new(b"three"),
        ];
        let written = b_write.write_vectored(&slices).await.expect("write");
        assert_eq!(written, 11, "loopback takes eleven bytes whole");
        b_tally.counts()
    });
    assert_eq!(counts.writes, 1, "{counts:?}");
    assert_eq!(counts.sent_bytes, 11, "{counts:?}");
}
