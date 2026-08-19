use crate::tree::typed::{ErasedPrefix, Hash, Prefix};

/// The local knowledge needed to interpret one future prefix-free reply.
///
/// `parent` names the scope whose children the reply discusses — its byte
/// length is the scope's height witness, exactly one level above the
/// children (see [`erased`](crate::tree::mirror::streaming::erased)) —
/// and `children` preserves the positional radices from the `Query` which
/// created it. Supplies remain self-keying and therefore do not advance
/// `next`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    parent: ErasedPrefix,
    children: Vec<u8>,
    next: usize,
}

impl Scope {
    /// Record the question represented by `listing` at `parent`.
    pub fn new(parent: ErasedPrefix, listing: &[(u8, Hash)]) -> Self {
        Self {
            parent,
            children: listing.iter().map(|(radix, _)| *radix).collect(),
            next: 0,
        }
    }

    /// The parent prefix against which keyed supplies are validated.
    pub fn parent(&self) -> ErasedPrefix {
        self.parent
    }

    /// Whether this scope is a whole-node request: an empty listing, "I
    /// lack this node entirely — send everything".
    pub fn is_request(&self) -> bool {
        self.children.is_empty()
    }

    /// Resolve the next positional reaction to its child radix and prefix.
    pub fn next(&mut self) -> Option<(u8, ErasedPrefix)> {
        let radix = *self.children.get(self.next)?;
        self.next += 1;
        Some((radix, self.parent.push(radix)))
    }

    /// Resolve a keyed supply to its claimed child prefix.
    pub fn supplied(&self, radix: u8) -> ErasedPrefix {
        self.parent.push(radix)
    }

    /// Retain the one leaf position requested by a terminal empty query.
    ///
    /// `prefix` is the requested leaf's full path, so the derived scope
    /// sits one level above the leaves.
    pub fn leaf(prefix: ErasedPrefix) -> Self {
        debug_assert_eq!(
            prefix.height(),
            0,
            "a terminal request names a full leaf path",
        );
        let (parent, radix) = prefix.pop();
        Self {
            parent,
            children: vec![radix],
            next: 0,
        }
    }

    /// Record the initiator's opening question about the root's children.
    pub fn opening(listing: &[(u8, Hash)]) -> Self {
        Self::new(Prefix::new().erase(), listing)
    }
}
