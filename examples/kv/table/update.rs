use super::{Key, Value};

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Update {
    pub key: Key,
    pub value: Option<Value>,
}

impl Update {
    pub fn insert(key: Key, value: Value) -> Self {
        let value = Some(value);
        Self { key, value }
    }

    pub fn remove(key: Key) -> Self {
        Self { key, value: None }
    }
}
