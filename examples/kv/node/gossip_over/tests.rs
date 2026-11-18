use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;

use crate::node::Node;
use crate::table::{Key, Update};

#[tokio::test]
async fn peers_agree_even_after_writing_at_once() -> Result<()> {
    use tokio::io::BufReader;

    fn connect_continuously(a: &Arc<Node>, b: &Arc<Node>) {
        let (a_end, b_end) = tokio::io::duplex(64 * 1024);
        for (node, end) in [(a.clone(), a_end), (b.clone(), b_end)] {
            let (read, write) = tokio::io::split(end);
            tokio::spawn(async move { node.gossip_over(BufReader::new(read), write).await });
        }
    }

    let bookmarks = tempfile::tempdir()?;
    let alice = Node::new(bookmarks.path());
    let bob = Node::new(bookmarks.path());

    alice.seed().await?;
    connect_continuously(&alice, &bob);
    bob.wait_for_replica().await;

    let key: Key = "greeting".try_into()?;
    let alice_hi = [Update::insert(key.clone(), "hi".try_into()?)];
    let bob_hey = [Update::insert(key.clone(), "hey".try_into()?)];
    tokio::try_join!(alice.update(&alice_hi), bob.update(&bob_hey))?;

    while alice.get(&key) != bob.get(&key) {
        sleep(Duration::from_millis(10)).await;
    }

    Ok(())
}
