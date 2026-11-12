async fn delete(
    State(node): State<Arc<Node>>,
    extract::Path(key): extract::Path<Key>,
) -> Result<StatusCode, (StatusCode, String)> {
    update(&node, &[Update { key, value: None }]).await
}
