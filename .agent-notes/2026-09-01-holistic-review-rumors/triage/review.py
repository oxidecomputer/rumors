#!/usr/bin/env python3
"""Local review packets for the triage lanes.

    review.py packet --base REF --head REF --annotations TSV --meta MD --out MD [--repo DIR]
    review.py replies PACKET.md

`packet` renders `git diff BASE...HEAD` hunk by hunk with the lane agent's
annotations interleaved under each hunk (the rows of the TSV whose path and
new-side line fall inside it), preceded by the coordinator-written sections
in META. Every hunk with no annotation is flagged, and the exit status is 1
when any is, so an unjustified change cannot reach Finch unnoticed. A
risk-ordered table of contents precedes the diff; the order is taken from
the annotation's `ruling` and `note` fields (see `risk_class`).

`replies` extracts every line beginning with the reply marker from a
packet Finch has edited, with the hunk header and the annotation it sits
under, as a numbered list the coordinator hands to the lane agent.

Annotation TSV columns: path, line, entry, ruling, note. The note may carry
literal `\\n` sequences, unescaped on render.
"""

import argparse
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPLY = ">> finch:"
COMMENT_START = "<!-- annotation -->"

HUNK_RE = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")


def run(args, cwd):
    return subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True).stdout


def read_annotations(path):
    rows = []
    for n, raw in enumerate(Path(path).read_text().splitlines(), 1):
        if not raw.strip() or raw.startswith("#"):
            continue
        parts = raw.split("\t")
        if len(parts) != 5:
            sys.exit(f"{path}:{n}: expected 5 tab-separated fields, found {len(parts)}")
        p, line, entry, ruling, note = parts
        rows.append(dict(path=p, line=int(line), entry=entry, ruling=ruling,
                         note=note.replace("\\n", "\n"), used=False))
    return rows


def risk_class(row):
    """Order of the table of contents: lower sorts first."""
    note = row["note"].lower()
    if "deviat" in note or "instead of the stated" in note:
        return 0, "deviations from a stated resolution"
    if "negative control" in note or "should_panic" in note or "fails on the current" in note:
        return 1, "new tests and negative controls"
    if row["path"].startswith("src/") and "/tests" not in row["path"] and not row["path"].endswith("tests.rs"):
        return 2, "production edits"
    if re.match(r"\s*nit\b", note):
        return 4, "nits"
    return 3, "tests and prose"


def parse_diff(text):
    """Yield (path, hunk_header, hunk_lines, new_start, new_len)."""
    path = None
    hunk = None
    for line in text.splitlines():
        if line.startswith("diff --git"):
            if hunk:
                yield hunk
                hunk = None
            path = None
        elif line.startswith("+++ "):
            path = line[4:]
            path = path[2:] if path.startswith("b/") else path
        elif line.startswith("@@"):
            if hunk:
                yield hunk
            m = HUNK_RE.match(line)
            new_start = int(m.group(3))
            new_len = int(m.group(4) or "1")
            hunk = dict(path=path, header=line, lines=[], new_start=new_start, new_len=max(new_len, 1))
        elif hunk is not None:
            hunk["lines"].append(line)
    if hunk:
        yield hunk


def packet(args):
    repo = Path(args.repo).resolve()
    diff = run(["git", "diff", f"{args.base}...{args.head}"], repo)
    rows = read_annotations(args.annotations)
    by_path = defaultdict(list)
    for r in rows:
        by_path[r["path"]].append(r)
    hunks = list(parse_diff(diff))
    out = [Path(args.meta).read_text().rstrip(), ""]
    toc = defaultdict(list)
    rendered = []
    unannotated = 0
    for i, h in enumerate(hunks, 1):
        lo, hi = h["new_start"], h["new_start"] + h["new_len"] - 1
        mine = [r for r in by_path.get(h["path"], []) if lo <= r["line"] <= hi]
        anchor = f"hunk-{i}"
        block = [f'<a id="{anchor}"></a>', f"### {h['path']} `{h['header']}`", "", "```diff", h["header"], *h["lines"], "```", ""]
        if not mine:
            block.append("**(no annotation)**")
            block.append("")
            unannotated += 1
        for r in mine:
            r["used"] = True
            cls, name = risk_class(r)
            toc[(cls, name)].append((r["entry"], r["ruling"], anchor, h["path"], r["line"]))
            block += [COMMENT_START, f"> **{r['entry']}** ({r['ruling']}), line {r['line']}:", ">", *("> " + l for l in r["note"].splitlines()), ""]
        rendered += block
    out.append("## Reading order")
    out.append("")
    for (cls, name) in sorted(toc):
        out.append(f"### {name}")
        out.append("")
        for entry, ruling, anchor, p, line in toc[(cls, name)]:
            out.append(f"- {entry} ({ruling}) at `{p}:{line}` ([hunk](#{anchor}))")
        out.append("")
    if unannotated:
        out.append(f"**{unannotated} hunk(s) carry no annotation; see the flags below.**")
        out.append("")
    stale = [r for r in rows if not r["used"]]
    if stale:
        out.append("**Annotations matching no hunk (stale line numbers or paths):**")
        out.append("")
        for r in stale:
            out.append(f"- {r['entry']} ({r['ruling']}) at `{r['path']}:{r['line']}`")
        out.append("")
    out.append("## The change")
    out.append("")
    out += rendered
    Path(args.out).write_text("\n".join(out) + "\n")
    print(f"{args.out}: {len(hunks)} hunks, {len(rows)} annotations, {unannotated} unannotated, {len(stale)} stale")
    return 1 if (unannotated or stale) else 0


def replies(args):
    text = Path(args.packet).read_text().splitlines()
    hunk = None
    annotation = None
    found = []
    for line in text:
        if line.startswith("### ") and "`@@" in line:
            hunk = line[4:]
            annotation = None
        elif line.startswith("> **") and "), line" in line:
            annotation = line[2:].strip()
        elif line.startswith("## "):
            hunk = line[3:]
            annotation = None
        elif line.lstrip().startswith(REPLY):
            found.append((hunk, annotation, line.lstrip()[len(REPLY):].strip()))
    for n, (h, a, reply) in enumerate(found, 1):
        print(f"{n}. [{h or 'preamble'}]" + (f" under {a}" if a else ""))
        print(f"   {reply}")
    if not found:
        print("no replies")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("packet")
    p.add_argument("--base", required=True)
    p.add_argument("--head", required=True)
    p.add_argument("--annotations", required=True)
    p.add_argument("--meta", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--repo", default=".")
    r = sub.add_parser("replies")
    r.add_argument("packet")
    args = ap.parse_args()
    sys.exit(packet(args) if args.cmd == "packet" else replies(args))


if __name__ == "__main__":
    main()
