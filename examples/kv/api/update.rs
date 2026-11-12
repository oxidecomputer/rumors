async fn update(node: &Node, updates: &[Update]) -> Result<StatusCode, (StatusCode, String)> {
    match node.update(updates).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}"))),
    }
}
