"""Apply the reviewer's runtime-selected shape mutants to shape.rs by exact swaps.

usage: python3 -I apply_shape.py <path to shape.rs>
Each swap's source text must occur exactly once. REVIEW_MUTANT selects at run time:
  cells       Cells gathered by one non-tail call frame per cell
  cells-tail  Cells gathered by a self tail call per cell
  regions     Regions gathered by a recursive descent of the party tree
  overlay     Overlay gathered by one non-tail call frame per item
Unset, every iterator runs its production walk.
"""
import sys
path = sys.argv[1]
text = open(path).read()

def swap(old, new):
    global text
    n = text.count(old)
    if n != 1:
        sys.exit(f"expected one occurrence, found {n}: {old[:60]!r}")
    text = text.replace(old, new)

# Regions: field, constructor, next.
swap("""pub struct Regions<'a> {
    walk: PartyWalk<'a>,
    finished: bool,
}""", """pub struct Regions<'a> {
    walk: PartyWalk<'a>,
    finished: bool,
    review_pre: Option<std::vec::IntoIter<Region>>,
}

/// MUTANT: name the active reviewer mutant.
fn review_mutant(name: &str) -> bool {
    std::env::var("REVIEW_MUTANT").is_ok_and(|v| v == name)
}

/// MUTANT: regions by a recursive descent, one frame per tree level.
fn review_regions(
    reader: &mut crate::party::io::PartyReader<'_>,
    depth: u64,
    out: &mut Vec<Region>,
) {
    use crate::party::io::PartyNode;
    match reader.read() {
        PartyNode::Owned => out.push(Region { owned: true, depth }),
        PartyNode::Branch(branch) => {
            if branch.has_left_child() {
                review_regions(reader, depth + 1, out);
            } else {
                out.push(Region { owned: false, depth: depth + 1 });
            }
            if branch.has_right_child() {
                review_regions(reader, depth + 1, out);
            } else {
                out.push(Region { owned: false, depth: depth + 1 });
            }
        }
    }
}""")
swap("""        Regions {
            walk: PartyWalk::open(party),
            finished: false,
        }""", """        let review_pre = review_mutant("regions").then(|| {
            let mut out = Vec::new();
            if party.is_empty() {
                out.push(Region { owned: false, depth: 0 });
            } else {
                review_regions(&mut crate::party::io::PartyReader::for_party(party), 0, &mut out);
            }
            out.into_iter()
        });
        Regions {
            walk: PartyWalk::open(party),
            finished: false,
            review_pre,
        }""")
swap("""    fn next(&mut self) -> Option<Region> {
        if self.finished {""", """    fn next(&mut self) -> Option<Region> {
        if let Some(pre) = &mut self.review_pre {
            return pre.next();
        }
        if self.finished {""")

# Overlay.
swap("""pub struct Overlay<'a> {
    version: VersionWalk<'a>,
    party: PartyWalk<'a>,
    finished: bool,
}""", """pub struct Overlay<'a> {
    version: VersionWalk<'a>,
    party: PartyWalk<'a>,
    finished: bool,
    review_pre: Option<std::vec::IntoIter<(Plateau, bool)>>,
}

/// MUTANT: overlay items gathered by one non-tail call frame per item.
fn review_overlay(
    version: &mut VersionWalk<'_>,
    party: &mut PartyWalk<'_>,
    out: &mut Vec<(Plateau, bool)>,
) {
    let plateau = Plateau {
        rise: version.take_rise(),
        depth: version.depth().max(party.depth()),
    };
    let owned = party.owned();
    let done = advance_refinement(&mut [
        &mut *version as &mut dyn Refine,
        &mut *party as &mut dyn Refine,
    ]);
    if !done {
        review_overlay(version, party, out);
    }
    out.push((plateau, owned));
}""")
swap("""        Overlay {
            version: VersionWalk::open(clock.version()),
            party: PartyWalk::open(clock.party()),
            finished: false,
        }""", """        let mut overlay = Overlay {
            version: VersionWalk::open(clock.version()),
            party: PartyWalk::open(clock.party()),
            finished: false,
            review_pre: None,
        };
        if review_mutant("overlay") {
            let mut out = Vec::new();
            review_overlay(&mut overlay.version, &mut overlay.party, &mut out);
            out.reverse();
            overlay.review_pre = Some(out.into_iter());
        }
        overlay""")
swap("""    fn next(&mut self) -> Option<(Plateau, bool)> {
        if self.finished {""", """    fn next(&mut self) -> Option<(Plateau, bool)> {
        if let Some(pre) = &mut self.review_pre {
            return pre.next();
        }
        if self.finished {""")

# Cells.
swap("""    Cells {
        walks: versions.map(VersionWalk::open),
        finished: false,
    }
}""", """    let mut cells = Cells {
        walks: versions.map(VersionWalk::open),
        finished: false,
        review_pre: None,
    };
    if review_mutant("cells") {
        let mut out = Vec::new();
        review_cells_nontail(&mut cells.walks, &mut out);
        out.reverse();
        cells.review_pre = Some(out.into_iter());
    } else if review_mutant("cells-tail") {
        let mut out = Vec::new();
        review_cells_tail(&mut cells.walks, &mut out);
        cells.review_pre = Some(out.into_iter());
    }
    cells
}

/// MUTANT: cells gathered by one non-tail call frame per cell.
fn review_cells_nontail<const N: usize>(walks: &mut [VersionWalk<'_>; N], out: &mut Vec<Cell<N>>) {
    let depth = walks.iter().map(Refine::depth).max().unwrap_or(0);
    let rises = walks.each_mut().map(VersionWalk::take_rise);
    let cell = Cell { depth, rises };
    if !advance_refinement(walks) {
        review_cells_nontail(walks, out);
    }
    out.push(cell);
}

/// MUTANT: cells gathered by a self tail call per cell.
fn review_cells_tail<const N: usize>(walks: &mut [VersionWalk<'_>; N], out: &mut Vec<Cell<N>>) {
    let depth = walks.iter().map(Refine::depth).max().unwrap_or(0);
    let rises = walks.each_mut().map(VersionWalk::take_rise);
    out.push(Cell { depth, rises });
    if !advance_refinement(walks) {
        review_cells_tail(walks, out)
    }
}""")
swap("""pub struct Cells<'a, const N: usize> {
    walks: [VersionWalk<'a>; N],
    finished: bool,
}""", """pub struct Cells<'a, const N: usize> {
    walks: [VersionWalk<'a>; N],
    finished: bool,
    review_pre: Option<std::vec::IntoIter<Cell<N>>>,
}""")
swap("""    fn next(&mut self) -> Option<Cell<N>> {
        if self.finished {""", """    fn next(&mut self) -> Option<Cell<N>> {
        if let Some(pre) = &mut self.review_pre {
            return pre.next();
        }
        if self.finished {""")
open(path, "w").write(text)
print("applied")

# size_hint: defer to the precomputed items in each mutated iterator.
text = open(path).read()
head = """    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished {"""
patched = """    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(pre) = &self.review_pre {
            return pre.size_hint();
        }
        if self.finished {"""
for impl_header in ["impl Iterator for Regions<'_>", "impl Iterator for Overlay<'_>", "impl<const N: usize> Iterator for Cells<'_, N>"]:
    if text.count(impl_header) != 1:
        sys.exit(f"impl header not unique: {impl_header}")
    at = text.index(impl_header)
    hit = text.index(head, at)
    text = text[:hit] + patched + text[hit + len(head):]
open(path, "w").write(text)
print("size_hint patched")
