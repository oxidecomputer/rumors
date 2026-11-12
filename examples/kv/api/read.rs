async fn read(
    State(node): State<Arc<Node>>,
    extract::Path(key): extract::Path<Key>,
) -> Result<String, StatusCode> {
    node.get(&key)
        .map(Value::into_inner)
        .ok_or(StatusCode::NOT_FOUND)
}
