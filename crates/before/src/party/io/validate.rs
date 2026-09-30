//! Validates canonical party trees.
//!
//! Validation reads one tree from a sequential bit reader. It records whether
//! each completed child is wholly owned, because a branch with two wholly
//! owned children must collapse to one owned region. Two bits per unfinished
//! branch retain exactly the state needed to enforce that rule, keeping memory
//! proportional to the input even for deeply nested trees.

mod frames;

use crate::bits::{BitRead, BitsReader};
use crate::error::Decode;

use frames::{Frame, Frames};

/// Validate one party tree at the reader and return its end position.
pub fn prefix(mut reader: BitsReader<'_>) -> Result<u64, Decode> {
    from_reader(&mut reader)?;
    Ok(reader.position())
}

/// Validate one party tree and return its end position.
#[cfg(all(test, feature = "borsh"))]
pub fn prefix_from_reader<C: BitRead>(reader: &mut C) -> Result<u64, Decode>
where
    Decode: From<C::Error>,
{
    from_reader(reader)?;
    Ok(reader.position())
}

/// Validate one party tree from a sequential bit reader.
pub fn from_reader<C: BitRead>(reader: &mut C) -> Result<(), Decode>
where
    Decode: From<C::Error>,
{
    let mut stack = Frames::default();
    loop {
        let left = reader.read_bit()?;
        let right = reader.read_bit()?;

        // A parent needs only to know whether the completed subtree is wholly
        // owned: two wholly owned children would make that parent noncanonical.
        let mut owned = match (left, right) {
            (true, true) => {
                stack.push(Frame::BothNeedLeft);
                continue;
            }
            (true, false) | (false, true) => {
                stack.push(Frame::OneChild);
                continue;
            }
            (false, false) => true,
        };

        // Attach the completed subtree to its parent. Completing a parent may
        // in turn complete its parent, so continue until another child is
        // required or the root closes.
        loop {
            match stack.pop() {
                None => return Ok(()),
                Some(Frame::BothNeedLeft) => {
                    stack.push(Frame::BothNeedRight { left_owned: owned });
                    break;
                }
                Some(Frame::BothNeedRight { left_owned }) => {
                    if left_owned && owned {
                        return Err(Decode::NotCanonical);
                    }
                    owned = false;
                }
                Some(Frame::OneChild) => owned = false,
            }
        }
    }
}
