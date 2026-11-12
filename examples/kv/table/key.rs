use nutype::nutype;

#[rustfmt::skip]
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, TryFrom, Into, Serialize, Deserialize)
)]
pub(crate) struct Key(String);
