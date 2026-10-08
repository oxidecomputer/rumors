"""Apply or revert the reviewer's switched calibration mutants by exact swap.

usage: mutate.py <worktree> apply|revert [depth100k]

`apply` backs up each touched file into ./orig/, then writes every mutant
into the tree behind `crate::reviewer_mutant(NAME)`, a switch read once from
the REVIEW_MUTANT environment variable, so one build serves every mutant.
`depth100k` also lowers STACK_SAFETY_DEPTH to 100_000.
`revert` copies the backups back; check `git diff` afterwards.
"""
import pathlib
import shutil
import sys

HERE = pathlib.Path(__file__).resolve().parent
ORIG = HERE / "orig"

HELPER_ANCHOR = """#[cfg(not(any(test, feature = "laws", feature = "meter", feature = "oracle")))]
mod testing;
"""
HELPER = HELPER_ANCHOR + """
/// Reports whether the reviewer's calibration mutant `name` is switched on
/// through the `REVIEW_MUTANT` environment variable.
pub(crate) fn reviewer_mutant(name: &str) -> bool {
    static ENABLED: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    ENABLED
        .get_or_init(|| std::env::var("REVIEW_MUTANT").ok())
        .as_deref()
        == Some(name)
}
"""

REMOVE_LOOP = """        for right in path {
            removal.descend(Selected::from_right(right));
        }

        removal.omit_selected_region();
"""
REMOVE_LOOP_MUT = """        if crate::reviewer_mutant("M26") {
            removal.m26_descend_all(path.into_iter());
        } else if crate::reviewer_mutant("M26S") {
            removal.m26s_descend_all(&mut path.into_iter());
        } else {
            for right in path {
                removal.descend(Selected::from_right(right));
            }
        }

        removal.omit_selected_region();
"""
REMOVE_DESCEND = """    /// Descend one spatial level, retaining the unselected sibling.
    fn descend(&mut self, selected: Selected) {"""
REMOVE_DESCEND_MUT = """    /// Calibration mutant M26: recursive descent, iterator by value.
    fn m26_descend_all(&mut self, mut path: impl Iterator<Item = bool>) {
        if let Some(right) = path.next() {
            self.descend(Selected::from_right(right));
            self.m26_descend_all(path);
        }
    }

    /// Calibration mutant M26-stored: recurse once per stored level, loop
    /// below an owned region.
    fn m26s_descend_all(&mut self, path: &mut impl Iterator<Item = bool>) {
        while let Some(right) = path.next() {
            self.descend(Selected::from_right(right));
            if !self.below_owned_region {
                self.m26s_descend_all(path);
                std::hint::black_box(());
                return;
            }
        }
    }

""" + REMOVE_DESCEND

PLACE_LOOP = """        advance_set(&mut set);
    }

    finish(
        set.start.as_ref().map(BoundSide::relation),
        set.end.as_ref().map(BoundSide::relation),
    )
}
"""
PLACE_LOOP_MUT = """        advance_set(&mut set);
        if set.start.is_none() && crate::reviewer_mutant("MCPD") {
            return sweep_end_alone(&mut set, &on_end, finish);
        }
    }

    finish(
        set.start.as_ref().map(BoundSide::relation),
        set.end.as_ref().map(BoundSide::relation),
    )
}

/// Calibration mutant: once the start drops, sweep the end one region per
/// call.
fn sweep_end_alone<V>(
    set: &mut Cursors<'_>,
    on_end: &impl Fn(OrderState, bool) -> ControlFlow<V, Fate>,
    finish: impl FnOnce(Option<Option<Ordering>>, Option<Option<Ordering>>) -> V,
) -> V {
    if let Some(side) = &mut set.end {
        side.read();
        match on_end(side.directions, false) {
            ControlFlow::Continue(Fate::Sweep) => {}
            ControlFlow::Continue(Fate::Drop) => set.end = None,
            ControlFlow::Break(verdict) => return verdict,
        }
    }
    let exhausted =
        set.probe.done() && set.end.as_ref().is_none_or(|side| side.cursor.done());
    if exhausted {
        return finish(None, set.end.as_ref().map(BoundSide::relation));
    }
    advance_set(set);
    std::hint::black_box(sweep_end_alone(set, on_end, finish))
}
"""

MT_START = "        while !leaves.done() {\n"
MT_END = "\n\n        // The final leaf closes every remaining subtree."
MT_REPLACEMENT = """        let _ = (&mut leaves, &mut recent_height_change);
        let mut fold = MinTicksFold {
            leaves,
            recent_height_change,
            answer,
            height_prefixes,
            subtree_minima,
        };
        if crate::reviewer_mutant("PAMT") {
            fold_rest(&mut fold);
        } else {
            while fold_step(&mut fold).is_some() {}
        }
        let MinTicksFold {
            mut answer,
            mut height_prefixes,
            mut subtree_minima,
            ..
        } = fold;"""
MT_ITEMS = """

/// Reviewer calibration: the state of the leaf fold.
struct MinTicksFold<'a> {
    leaves: VersionRegionReader<'a>,
    recent_height_change: Accumulator,
    answer: Accumulator,
    height_prefixes: minima::HeightPrefixes,
    subtree_minima: minima::SubtreeMinima,
}

/// Reviewer calibration: fold one more leaf; `None` once the stream is done.
#[inline(never)]
fn fold_step<'f, 'a>(fold: &'f mut MinTicksFold<'a>) -> Option<&'f mut MinTicksFold<'a>> {
    if fold.leaves.done() {
        return None;
    }
    let MinTicksFold {
        leaves,
        recent_height_change,
        answer,
        height_prefixes,
        subtree_minima,
    } = &mut *fold;
    let previous_depth = leaves.depth();
    let (turn_depth, height_change) = leaves.step();
    recent_height_change.add_bigint(&height_change);
    subtree_minima.advance_height(&height_change);
    for _ in 0..previous_depth - turn_depth {
        subtree_minima.close_subtree(answer, height_prefixes);
    }
    subtree_minima.open_subtrees(leaves.depth() - turn_depth);
    if recent_height_change.stored_digit_count()
        > accumulator::digit_len(height_change.magnitude()) + HEIGHT_FREEZE_ALLOWANCE_DIGITS
    {
        height_prefixes.freeze(recent_height_change);
    }
    let leaf = height_prefixes.add_leaf(recent_height_change, answer);
    subtree_minima.observe_leaf(&leaf, answer, height_prefixes);
    Some(fold)
}

/// Reviewer calibration mutant PAMT: the leaf fold, one call frame per leaf.
/// The state comes back through `fold_step`'s return value, so the frame
/// holds no saved register.
#[inline(never)]
fn fold_rest(fold: &mut MinTicksFold<'_>) {
    if let Some(fold) = fold_step(fold) {
        fold_rest(fold);
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}
"""

DEPTH = "pub(crate) const STACK_SAFETY_DEPTH: usize = 1 << 18;"
DEPTH_MUT = "pub(crate) const STACK_SAFETY_DEPTH: usize = 100_000;"

FILES = {
    "lib": "crates/before/src/lib.rs",
    "remove": "crates/before/src/party/fork/remove.rs",
    "place": "crates/before/src/version/place.rs",
    "min_ticks": "crates/before/src/version/measure/min_ticks.rs",
    "generators": "crates/before/src/testing/generators.rs",
}


def swap(text, old, new, label):
    n = text.count(old)
    if n != 1:
        sys.exit(f"{label}: expected one occurrence, found {n}")
    return text.replace(old, new)


def apply(wt, depth100k):
    ORIG.mkdir(exist_ok=True)
    keys = list(FILES) if depth100k else [k for k in FILES if k != "generators"]
    for key in keys:
        shutil.copy2(wt / FILES[key], ORIG / key)
    p = wt / FILES["lib"]
    p.write_text(swap(p.read_text(), HELPER_ANCHOR, HELPER, "helper"))
    p = wt / FILES["remove"]
    t = swap(p.read_text(), REMOVE_LOOP, REMOVE_LOOP_MUT, "remove loop")
    p.write_text(swap(t, REMOVE_DESCEND, REMOVE_DESCEND_MUT, "remove descend"))
    p = wt / FILES["place"]
    p.write_text(swap(p.read_text(), PLACE_LOOP, PLACE_LOOP_MUT, "place loop"))
    p = wt / FILES["min_ticks"]
    t = p.read_text()
    if t.count(MT_START) != 1 or t.count(MT_END) != 1:
        sys.exit("min_ticks: anchors not unique")
    i, j = t.index(MT_START), t.index(MT_END)
    t = t[:i] + MT_REPLACEMENT + t[j:]
    t = t.rstrip("\n") + MT_ITEMS
    p.write_text(t)
    if depth100k:
        p = wt / FILES["generators"]
        p.write_text(swap(p.read_text(), DEPTH, DEPTH_MUT, "depth"))
    print("applied:", ", ".join(keys))


def revert(wt):
    for key, rel in FILES.items():
        src = ORIG / key
        if src.exists():
            shutil.copy2(src, wt / rel)
            src.unlink()
    print("reverted")


if __name__ == "__main__":
    wt = pathlib.Path(sys.argv[1])
    mode = sys.argv[2]
    if mode == "apply":
        apply(wt, len(sys.argv) > 3 and sys.argv[3] == "depth100k")
    else:
        revert(wt)
