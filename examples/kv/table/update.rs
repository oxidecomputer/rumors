#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Update {
    key: Key,
    value: Option<Value>,
}
