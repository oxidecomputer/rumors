//! `project`: mask an event tree onto a party's id — keep the value wherever the
//! party owns the region, zero it everywhere else. This is the engine behind
//! [`Version / &Party`](crate::Version) ("the party's contribution to the
//! version").
//!
//! Masking cannot be a relative-base rewrite: zeroing an unowned region whose
//! value comes from an *ancestor* base would need that base undone below the
//! split, and event bases are non-negative. So the walk rebuilds from
//! **absolute** values — it threads the accumulated ancestor path sum `off` down
//! the `(id, ev)` recursion (exactly as [`max`](EvReader::max) threads its
//! running sum), emits owned regions at `off + local` and unowned regions as
//! `0`, opening every internal node at base `0` and letting the [`Builder`]'s
//! sink lift the real bases back up. `O(n + m)`.

use crate::codec::{Base, BitsSlice};
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

use crate::version::compare::{EvNode, EvReader};
use crate::version::working::WorkingVersion;

use super::{Builder, Slot};

impl EvReader<'_> {
    /// Project this event tree (`self`) onto the region owned by the id
    /// `id_bits`: keep each value where the id is full, zero it where the id is
    /// empty. Produces normal form. `O(n + m)`.
    pub(in crate::version) fn project(self, id_bits: &BitsSlice) -> WorkingVersion {
        // The broadcast case can expand one event leaf into an id-shaped
        // subtree, so allow for the id's nodes on top of the event's.
        let cap = self.node_capacity_bound() + id_bits.len();
        let mut walk = ProjectWalk {
            out: Builder::with_capacity(cap),
        };
        let mut ev = self;
        let mut id = IdReader::root(id_bits);
        descend!(0, walk.rec(&mut id, &mut ev, &Base::ZERO, 0));
        walk.out.finish()
    }
}

/// The single output builder of a [`project`](EvReader::project) walk; the
/// `&mut` readers carry the traversal state.
struct ProjectWalk {
    out: Builder,
}

impl ProjectWalk {
    /// Project the event subtree at `ev`, whose root-to-parent path sum is
    /// `off`, onto the id subtree at `id`. Advances both readers past their
    /// subtrees, emitting into `out`, and routes through the amortized
    /// stack-growth guard. Returns the output root.
    fn rec(&mut self, id: &mut IdReader, ev: &mut EvReader, off: &Base, depth: usize) -> Slot {
        match id.read() {
            // Unowned: the whole region is masked to zero; drop the event.
            IdNode::Empty => {
                ev.skip();
                self.out.leaf(Base::ZERO).into()
            }
            // Owned: keep the event verbatim, lifted into absolute value by `off`.
            IdNode::Full => self.copy_shifted(ev, off),
            // The id splits this region: descend, pushing any event base down so
            // the masked side can still reach zero.
            IdNode::Internal => match ev.read() {
                EvNode::Internal(base) => {
                    let off2 = off + &base;
                    let node = self.out.open(Base::ZERO);
                    let _left = descend!(depth + 1, self.rec(id, ev, &off2, depth + 1));
                    let right = descend!(depth + 1, self.rec(id, ev, &off2, depth + 1));
                    self.out.close_node(node, right)
                }
                EvNode::Leaf(n) => {
                    // A constant `off + n` over a region the id subdivides:
                    // broadcast it to both id children (a fresh `Zero` event on
                    // each, with the constant folded into the offset).
                    let val = off + &n;
                    let node = self.out.open(Base::ZERO);
                    let mut z1 = EvReader::Zero;
                    let mut z2 = EvReader::Zero;
                    let _left = descend!(depth + 1, self.rec(id, &mut z1, &val, depth + 1));
                    let right = descend!(depth + 1, self.rec(id, &mut z2, &val, depth + 1));
                    self.out.close_node(node, right)
                }
            },
        }
    }

    /// Emit the event subtree at `ev` lifted by `off` into absolute value: its
    /// root base becomes `off + base`, every descendant copied verbatim (already
    /// normal form). At the top level `off` is zero and this is a verbatim copy.
    fn copy_shifted(&mut self, ev: &mut EvReader, off: &Base) -> Slot {
        if *off == Base::ZERO {
            return self.out.copy_reader(ev);
        }
        match ev.read() {
            EvNode::Leaf(n) => self.out.leaf(off + &n).into(),
            EvNode::Internal(base) => {
                // A normal-form node has a zero-base child, so the sink in
                // `close_node` is a no-op here: the root keeps `off + base`.
                let node = self.out.open(off + &base);
                let _left = self.out.copy_reader(ev);
                let right = self.out.copy_reader(ev);
                self.out.close_node(node, right)
            }
        }
    }
}
