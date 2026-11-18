use anyhow::Result;

use crate::node::Node;
use crate::table::{Key, Update, Value};

#[tokio::test]
async fn a_node_reads_its_own_writes() -> Result<()> {
    let bookmarks = tempfile::tempdir()?;
    let node = Node::new(bookmarks.path());
    node.seed().await?;

    let key: Key = "greeting".try_into()?;
    let value: Value = "hello".try_into()?;

    node.update(&[Update::insert(key.clone(), value.clone())])
        .await?;

    assert_eq!(node.get(&key), Some(value));

    Ok(())
}
