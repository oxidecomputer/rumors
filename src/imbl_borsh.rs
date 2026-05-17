//! Borsh helpers for [`imbl`] containers.
//!
//! `imbl::OrdMap` and `imbl::OrdSet` don't ship with `borsh` impls, but they
//! have a canonical (key-sorted) iteration order that pairs naturally with
//! the borsh convention already used for [`crate::Version`]'s `OrdMap`
//! field. The helpers here factor that convention out so message types can
//! `#[derive(BorshSerialize, BorshDeserialize)]` and pin the encoding of
//! each container via `#[borsh(serialize_with = ..., deserialize_with = ...)]`.
//!
//! Wire format (shared by `OrdMap` and `OrdSet`):
//!
//! 1. `len: u32` — entry count.
//! 2. For each entry, in strictly-ascending key order, the entry's
//!    borsh encoding.
//!
//! Deserialization rejects duplicate or out-of-order keys so every value
//! has exactly one canonical serialization.

use std::cmp::Ordering;
use std::mem;

use borsh::{BorshDeserialize, BorshSerialize};
use imbl::{OrdMap, OrdSet};

/// Matches `borsh`'s own guard against zero-sized keys: deserializing a
/// `u32`-prefixed run of ZST entries would let a tiny payload allocate an
/// enormous collection. Mirrors `borsh::error::check_zst`, which is
/// private upstream.
pub fn check_zst<T>() -> borsh::io::Result<()> {
    if mem::size_of::<T>() == 0 {
        return Err(borsh::io::Error::new(
            borsh::io::ErrorKind::InvalidData,
            "Collections of zero-sized types are not allowed due to deny-of-service concerns on deserialization.",
        ));
    }
    Ok(())
}

fn len_as_u32(len: usize) -> borsh::io::Result<u32> {
    u32::try_from(len).map_err(|_| {
        borsh::io::Error::new(
            borsh::io::ErrorKind::InvalidData,
            "Collection length exceeds u32",
        )
    })
}

/// Borsh-serialize an `OrdMap<K, V>` as `u32` length followed by every
/// `(K, V)` in canonical key-ascending order.
pub fn serialize_ordmap<K, V, W>(map: &OrdMap<K, V>, writer: &mut W) -> borsh::io::Result<()>
where
    K: Ord + BorshSerialize,
    V: BorshSerialize,
    W: borsh::io::Write,
{
    check_zst::<K>()?;
    len_as_u32(map.len())?.serialize(writer)?;
    for (key, value) in map {
        key.serialize(writer)?;
        value.serialize(writer)?;
    }
    Ok(())
}

/// Borsh-deserialize an `OrdMap<K, V>`. Rejects duplicate or out-of-order
/// keys to enforce a unique canonical encoding.
pub fn deserialize_ordmap<K, V, R>(reader: &mut R) -> borsh::io::Result<OrdMap<K, V>>
where
    K: Ord + Clone + BorshDeserialize,
    V: Clone + BorshDeserialize,
    R: borsh::io::Read,
{
    check_zst::<K>()?;
    let len = u32::deserialize_reader(reader)?;
    let mut out = OrdMap::new();
    let mut prev: Option<K> = None;
    for _ in 0..len {
        let key = K::deserialize_reader(reader)?;
        let value = V::deserialize_reader(reader)?;
        if let Some(prev) = &prev {
            match prev.cmp(&key) {
                Ordering::Less => {}
                Ordering::Equal => {
                    return Err(borsh::io::Error::new(
                        borsh::io::ErrorKind::InvalidData,
                        "OrdMap contains duplicate key",
                    ));
                }
                Ordering::Greater => {
                    return Err(borsh::io::Error::new(
                        borsh::io::ErrorKind::InvalidData,
                        "OrdMap keys out of order",
                    ));
                }
            }
        }
        prev = Some(key.clone());
        out.insert(key, value);
    }
    Ok(out)
}

/// Borsh-serialize an `OrdSet<T>` as `u32` length followed by every `T`
/// in canonical ascending order.
pub fn serialize_ordset<T, W>(set: &OrdSet<T>, writer: &mut W) -> borsh::io::Result<()>
where
    T: Ord + BorshSerialize,
    W: borsh::io::Write,
{
    check_zst::<T>()?;
    len_as_u32(set.len())?.serialize(writer)?;
    for value in set {
        value.serialize(writer)?;
    }
    Ok(())
}

/// Borsh-deserialize an `OrdSet<T>`. Rejects duplicate or out-of-order
/// entries.
pub fn deserialize_ordset<T, R>(reader: &mut R) -> borsh::io::Result<OrdSet<T>>
where
    T: Ord + Clone + BorshDeserialize,
    R: borsh::io::Read,
{
    check_zst::<T>()?;
    let len = u32::deserialize_reader(reader)?;
    let mut out = OrdSet::new();
    let mut prev: Option<T> = None;
    for _ in 0..len {
        let value = T::deserialize_reader(reader)?;
        if let Some(prev) = &prev {
            match prev.cmp(&value) {
                Ordering::Less => {}
                Ordering::Equal => {
                    return Err(borsh::io::Error::new(
                        borsh::io::ErrorKind::InvalidData,
                        "OrdSet contains duplicate entry",
                    ));
                }
                Ordering::Greater => {
                    return Err(borsh::io::Error::new(
                        borsh::io::ErrorKind::InvalidData,
                        "OrdSet entries out of order",
                    ));
                }
            }
        }
        prev = Some(value.clone());
        out.insert(value);
    }
    Ok(out)
}

#[cfg(test)]
mod test;
