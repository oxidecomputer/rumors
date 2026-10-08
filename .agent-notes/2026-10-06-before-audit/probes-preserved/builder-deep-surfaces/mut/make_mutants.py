import json
WT = "/Users/oxide/src/rumors-slot-01/crates/before/src/"

# M1: the hull sweep recurses once per boundary instead of looping.
m1_old = """        while !(cursor_a.done() && cursor_b.done()) {
            // Decode this boundary once, then write both resulting deltas.
            let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
            let sign = diff.cmp_zero();
            directions.fold(sign);
            let depth = cursor_a.depth().max(cursor_b.depth());
            for emission in &mut outputs {
                let new_side = emission.extreme.pick(sign, emission.side);
                let old_side = emission.side;
                emission.side = new_side;
                old_side.write_delta(
                    &mut emission.out,
                    depth,
                    &diff,
                    new_side,
                    step_a.as_ref(),
                    step_b.as_ref(),
                );
            }
        }
"""
m1_new = """        // MUTANT M1: one recursive call per boundary.
        fn sweep(
            cursor_a: &mut crate::version::io::regions::VersionRegionReader<'_>,
            cursor_b: &mut crate::version::io::regions::VersionRegionReader<'_>,
            diff: &mut Accumulator,
            directions: &mut OrderState,
            outputs: &mut [Emission; 2],
        ) {
            if cursor_a.done() && cursor_b.done() {
                return;
            }
            let (step_a, step_b) = advance_diff(cursor_a, cursor_b, diff);
            let sign = diff.cmp_zero();
            directions.fold(sign);
            let depth = cursor_a.depth().max(cursor_b.depth());
            for emission in outputs.iter_mut() {
                let new_side = emission.extreme.pick(sign, emission.side);
                let old_side = emission.side;
                emission.side = new_side;
                old_side.write_delta(
                    &mut emission.out,
                    depth,
                    diff,
                    new_side,
                    step_a.as_ref(),
                    step_b.as_ref(),
                );
            }
            sweep(cursor_a, cursor_b, diff, directions, outputs);
        }
        sweep(&mut cursor_a, &mut cursor_b, &mut diff, &mut directions, &mut outputs);
"""
json.dump({"file": WT + "version/lattice.rs", "old": m1_old, "new": m1_new}, open("m1.json", "w"))

# M2: Version::shape gathers its plateaus by a recursive tree descent.
m2_old = """pub struct Plateaus<'a> {
    walk: VersionWalk<'a>,
    finished: bool,
}

impl<'a> Plateaus<'a> {
    /// Open a version's shape at its first plateau.
    pub(crate) fn of_version(version: &'a Version) -> Self {
        Plateaus {
            walk: VersionWalk::open(version),
            finished: false,
        }
    }
}

impl Iterator for Plateaus<'_> {
    type Item = Plateau;

    fn next(&mut self) -> Option<Plateau> {
        if self.finished {
            return None;
        }
        let plateau = Plateau {
            rise: self.walk.take_rise(),
            depth: self.walk.depth(),
        };
        if self.walk.done() {
            self.finished = true;
        } else {
            self.walk.advance();
        }
        Some(plateau)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished {
            (0, Some(0))
        } else {
            (1, None)
        }
    }
}
"""
m2_new = """pub struct Plateaus<'a> {
    items: std::vec::IntoIter<Plateau>,
    _version: core::marker::PhantomData<&'a Version>,
}

impl<'a> Plateaus<'a> {
    // MUTANT M2: gather the plateaus by a recursive descent of the tree.
    pub(crate) fn of_version(version: &'a Version) -> Self {
        use crate::version::io::regions::PayloadKind;
        use crate::version::io::tree::{VersionNode, VersionTreeReader};
        use num_bigint::{BigUint, Sign};
        fn leaves(reader: &mut VersionTreeReader<'_>, depth: u64, out: &mut Vec<(u64, BigUint)>) {
            match reader.node() {
                VersionNode::Branch => {
                    leaves(reader, depth + 1, out);
                    leaves(reader, depth + 1, out);
                }
                VersionNode::Leaf => out.push((depth, reader.payload())),
            }
        }
        let mut found = Vec::new();
        leaves(&mut VersionTreeReader::new(version), 0, &mut found);
        let mut kind = PayloadKind::Absolute;
        let items: Vec<Plateau> = found
            .into_iter()
            .map(|(depth, code)| {
                let (sign, magnitude) = kind.decode(code).into_parts();
                kind = PayloadKind::Delta;
                let rise = match sign {
                    Sign::Plus => Some(Rise::Up(Count(magnitude))),
                    Sign::Minus => Some(Rise::Down(Count(magnitude))),
                    Sign::NoSign => None,
                };
                Plateau { rise, depth }
            })
            .collect();
        Plateaus {
            items: items.into_iter(),
            _version: core::marker::PhantomData,
        }
    }
}

impl Iterator for Plateaus<'_> {
    type Item = Plateau;

    fn next(&mut self) -> Option<Plateau> {
        self.items.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.items.len() == 0 {
            (0, Some(0))
        } else {
            (1, None)
        }
    }
}
"""
json.dump({"file": WT + "shape.rs", "old": m2_old, "new": m2_new}, open("m2.json", "w"))
print("written")
