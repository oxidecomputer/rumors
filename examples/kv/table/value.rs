use nutype::nutype;

#[rustfmt::skip]
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, TryFrom, Into, Serialize, Deserialize)
)]
pub struct Value(String);
