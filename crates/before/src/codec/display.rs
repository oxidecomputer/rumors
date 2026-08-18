use crate::idbits::{IdNode, IdReader};

use super::{BitsMut, BitsView};

/// While rendering an open id node, which child the walk is inside.
///
/// One phase bit per open node on the pending stack: [`LEFT_PHASE`] sits over
/// the right child's presence bit (still needed when the left child completes);
/// [`RIGHT_PHASE`] alone (closing the node needs nothing more).
const LEFT_PHASE: bool = true;

/// The complement of [`LEFT_PHASE`]: the node's right child is being written,
/// and its completion closes the node.
const RIGHT_PHASE: bool = false;

/// Write an id tree in the paper's grammar with `sep` between a node's two
/// children (`", "`).
///
/// Leaves render as `0`/`1`, nodes as `(l<sep>r)`; the empty `0` id (no bits)
/// renders as `0`. Iterative: the walk reads each 2-bit tag through the metered
/// id cursor ([`IdReader`], the seam every id walk shares, so the scan meter
/// sees the traversal), and its control state is one to two bits per open node
/// on a bit stack — a deep id costs bits, never stack frames or grown segments.
pub(crate) fn write_id(
    bits: BitsView<'_>,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
) -> core::fmt::Result {
    let mut reader = IdReader::root(bits);
    // Per open node: a phase bit on top ([`LEFT_PHASE`]/[`RIGHT_PHASE`]); under
    // a left phase, the right child's presence bit.
    let mut pending = BitsMut::new();
    // Whether the child to render next is present (decode the cursor) or an
    // absent `0` (the cursor holds no bits for it).
    let mut present = true;
    loop {
        if present {
            match reader.read() {
                // Only the empty root reads `Empty`: an absent child inside the
                // tree takes the `else` branch without touching the cursor.
                IdNode::Empty => f.write_str("0")?,
                IdNode::Full => f.write_str("1")?,
                IdNode::Internal { left, right } => {
                    f.write_str("(")?;
                    pending.push(right);
                    pending.push(LEFT_PHASE);
                    present = left;
                    continue;
                }
            }
        } else {
            f.write_str("0")?; // an absent child renders `0`
        }
        // A subtree just completed: finish every node it closes, then step into
        // the innermost pending right child.
        loop {
            match pending.pop() {
                None => return Ok(()), // the root is complete
                Some(LEFT_PHASE) => {
                    let right = pending
                        .pop()
                        .expect("a left phase holds its right presence bit");
                    f.write_str(sep)?;
                    pending.push(RIGHT_PHASE);
                    present = right;
                    break;
                }
                Some(_) => f.write_str(")")?, // a right phase: the node closes
            }
        }
    }
}
