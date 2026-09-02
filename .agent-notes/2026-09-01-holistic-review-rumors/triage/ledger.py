#!/usr/bin/env python3
"""Triage ledger for the holistic review: seed, check, and summarize.

The six topic documents (`../correctness.md` and siblings) are the record of
what was found. This ledger is the record of what was decided about each
entry. `ledger.tsv` has one row per finding id; the seed columns are derived
from the documents and this script regenerates them, the triage columns are
filled in by hand as rulings land and lanes close. `TRIAGE.md` explains the
phases, lanes, and dispositions the columns refer to.

    ledger.py seed     rebuild the seed columns, preserving triage columns
    ledger.py check    verify coverage and vocabulary; exit 1 on any defect
    ledger.py summary  counts by phase, disposition, and severity

Seed columns: id, doc, severity, class, provenance, owner_gated, module,
patterns, owner_decision, readme_top, phase (proposed), lane (proposed), title.
Triage columns: disposition, ruling, sha, note.

Coverage is the point: the cheapest way to "finish" a 995-entry triage is
to disposition the highs and forget the lows, so `check` fails while any
id in the documents lacks a row, and refuses the dispositions the doctrine
forbids (a deferred correctness defect, a model ruling that names no
ruling). A `fix` without a landed sha is pending, not defective; triage
closes when pending and defects are both zero.
"""

import csv
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
NOTE = HERE.parent
LEDGER = HERE / "ledger.tsv"
DOCS = ["correctness", "verification", "documentation", "simplification", "performance", "api"]

SEED_COLS = ["id", "doc", "severity", "class", "provenance", "owner_gated", "module",
             "patterns", "owner_decision", "readme_top", "phase", "lane", "title"]
TRIAGE_COLS = ["disposition", "ruling", "sha", "note"]
COLS = SEED_COLS + TRIAGE_COLS

# Disposition vocabulary. `fix` and `fix-amended` are terminal only with a sha.
# `model`, `dispute`, and `dup` are terminal with a ruling reference.
# `defer` is terminal with a ruling reference naming the home, and is never
# admitted for a high-severity or correctness-class entry.
DISPOSITIONS = {"fix", "fix-amended", "model", "defer", "dispute", "dup", "open"}

ID_RE = re.compile(r"\b([a-z]+(?:-[a-z]+)*-\d+)\b")

# README owner-decision number -> proposed phase (see TRIAGE.md, "Phases").
def decision_phase(n):
    n = int(n)
    if n <= 14:
        return "P3"
    if n <= 35:
        return "P6"
    if n <= 41:
        return "P2"
    if n <= 61:
        return "P1"
    if n <= 72:
        return "P7"
    if n <= 78:
        return "P4"
    if n == 79:
        return "P5"
    if n <= 93:
        return "P4"
    return "P8"

# Crate-wide pattern label -> proposed phase. Anything not listed falls to
# the pattern's document default below.
PATTERN_PHASE = {
    "correctness: Harnesses that read a failure as success": "P1",
    "verification: Instruments no recipe runs": "P1",
    "verification: Ceilings without liveness floors, and budgets that resolve to the floor": "P1",
    "documentation: Em-dashes in line comments and assert strings": "P3",
    "documentation: Metaphors promoted to jargon": "P3",
    "documentation: Moralized vocabulary colliding with the model of record": "P3",
    "documentation: The illumos lint-allow rationale in nine copies": "P3",
    "simplification: `pub` inside the private `tree` module carries no information": "P3",
    "simplification: Inline module bodies against the sibling-file convention": "P3",
    "simplification: Em-dashes in `//` comments": "P3",
    "simplification: Import hygiene left by three mechanical sweeps": "P3",
    "simplification: Clippy pedantic and nursery stragglers with zero false positives in the run": "P3",
    "simplification: Test-harness helpers re-spelled per binary": "P5",
    "api: Bootstrap and retire sessions are reimplemented per test suite": "P5",
}
PATTERN_DOC_DEFAULT = {"correctness": "P2", "verification": "P4", "documentation": "P4",
                       "simplification": "P4", "performance": "P7", "api": "P6"}
# An entry in no pattern and no owner decision: by document.
DOC_DEFAULT = {"correctness": "P2", "verification": "P5", "documentation": "P5",
               "simplification": "P5", "performance": "P7", "api": "P6"}
# Entries placed by hand: the eleven distinct high-severity defects, and the
# holes in the gate's own instruments (testdoc's blind spots, the unpinned
# mutants tool, the CI leg the gate runs and CI omits), which are phase 1
# because every later lane is judged by them.
HAND_PHASE = {
    "remote-capture-atlas-13": "P1", "streaming-tests-11": "P1", "tests-disruption-handshake-7": "P1",
    "tests-bookmark-9": "P1", "conformance-28": "P1", "tests-resource-link-window-20": "P1",
    "materialized-27": "P1", "tests-bookmark-12": "P1", "remote-proxy-tests-10": "P1",
    "tests-wire-format-26": "P1", "async-hazards-3": "P2",
    "tests-observation-37": "P1", "remote-proxy-tests-27": "P1", "tests-disruption-handshake-33": "P1",
    "verification-infra-10": "P1", "verification-infra-9": "P1", "verification-infra-7": "P1",
    "verification-infra-8": "P1",
}
# Module headings whose entries default to phase 1 rather than a module lane.
GATE_MODULES = ("Verification infrastructure", "Verification recipes", "Manifest, lints")

# Module heading (as the topic documents spell it) -> lane, first match wins.
LANE_RULES = [
    ("tests/common", "harness"), ("Test scaffolding", "harness"),
    ("Integration tests", "tests"), ("Verification", "gate"), ("Manifest", "gate"),
    ("cost of verification", "gate"), ("Benches", "benches"), ("Crate root", "core"),
    ("Session", "core"), ("Link", "link"), ("Conformance", "link"), ("Tree", "tree"),
    ("Mirror common", "streaming"), ("Streaming", "streaming"), ("Materialized", "streaming"),
    ("Remote", "remote"),
]


def lane_for(module):
    for key, lane in LANE_RULES:
        if key.lower() in module.lower():
            return lane
    return "?"


def phase_for(fid, doc, module, decisions, patterns):
    if fid in HAND_PHASE:
        return HAND_PHASE[fid]
    cands = [decision_phase(n) for n in decisions]
    for p in patterns:
        cands.append(PATTERN_PHASE.get(p, PATTERN_DOC_DEFAULT[p.split(":")[0]]))
    if not cands:
        if any(module.startswith(g) for g in GATE_MODULES):
            return "P1"
        return DOC_DEFAULT[doc]
    return min(cands)


def parse_documents():
    """Every finding entry and nit-table row in the six topic documents."""
    rows = {}
    for d in DOCS:
        lines = (NOTE / f"{d}.md").read_text().split("\n")
        module = ""
        i = 0
        while i < len(lines):
            line = lines[i]
            m = re.match(r"^## (.*)", line)
            if m:
                module = m.group(1)
                i += 1
                continue
            m = re.match(r"^### ([a-z-]+-\d+): (.*)", line)
            if m:
                fid, title = m.groups()
                meta = {}
                j = i + 1
                while j < len(lines) and lines[j].startswith("- "):
                    k, _, v = lines[j][2:].partition(":")
                    meta[k.strip()] = v.strip()
                    j += 1
                parts = [p.strip() for p in meta.get("Class / severity / confidence", "").split("/")]
                cls, sev = (parts + ["", ""])[:2]
                prov = meta.get("Provenance", "").split("(")[0].split(" ")[0].strip()
                og = meta.get("Owner-gated", "").lower()
                og = "yes" if og.startswith("yes") else ("no" if og else "")
                rows[fid] = dict(id=fid, doc=d, severity=sev or "xref", **{"class": cls},
                                 provenance=prov, owner_gated=og, module=module, title=title)
                i = j
                continue
            m = re.match(r"^\| ([a-z-]+-\d+) \| (.*?) \| (.*?) \| (.*?) \|", line)
            if m and module:
                fid, _where, claim, _res = m.groups()
                rows[fid] = dict(id=fid, doc=d, severity="nit", **{"class": "(table)"},
                                 provenance="", owner_gated="", module=module, title=claim)
            i += 1
    return rows


def parse_patterns(known):
    pat = defaultdict(set)
    for d in DOCS:
        text = (NOTE / f"{d}.md").read_text()
        m = re.search(r"^## Crate-wide patterns\n(.*?)^## ", text, re.S | re.M)
        if not m:
            continue
        for chunk in re.split(r"(?m)^(?=### |- \*\*|\*\*)", m.group(1)):
            h = re.match(r"### (.*)|- \*\*(.*?)\*\*|\*\*(.*?)\*\*", chunk)
            if not h:
                continue
            label = next(g for g in h.groups() if g).rstrip(". ")
            for fid in set(ID_RE.findall(chunk)):
                if fid in known:
                    pat[fid].add(f"{d}: {label}")
    return pat


def parse_readme(known):
    text = (NOTE / "README.md").read_text()
    dec, top = defaultdict(list), defaultdict(list)
    od = re.search(r"^## Owner decisions\n(.*?)^## Module quality map", text, re.S | re.M).group(1)
    for m in re.finditer(r"^(\d+)\. \*\*(.*?)\*\*(.*?)(?=^\d+\. \*\*|\Z)", od, re.S | re.M):
        for fid in set(ID_RE.findall(m.group(3))):
            if fid in known:
                dec[fid].append(m.group(1))
    hv = re.search(r"^## Highest-value findings across the review\n(.*?)^## Owner decisions",
                   text, re.S | re.M).group(1)
    for m in re.finditer(r"^(\d+)\. (.*?)$", hv, re.M):
        for fid in set(ID_RE.findall(m.group(2))):
            if fid in known:
                top[fid].append(m.group(1))
    return dec, top


def read_ledger():
    if not LEDGER.exists():
        return {}
    with LEDGER.open() as f:
        return {r["id"]: r for r in csv.DictReader(f, delimiter="\t")}


def write_ledger(rows):
    with LEDGER.open("w") as f:
        w = csv.DictWriter(f, fieldnames=COLS, delimiter="\t", lineterminator="\n")
        w.writeheader()
        for fid in sorted(rows, key=lambda k: (rows[k]["doc"], rows[k]["module"], k)):
            w.writerow({c: rows[fid].get(c, "") for c in COLS})


def seed():
    found = parse_documents()
    pat = parse_patterns(found)
    dec, top = parse_readme(found)
    old = read_ledger()
    out = {}
    for fid, r in found.items():
        decisions = sorted(dec.get(fid, []), key=int)
        patterns = sorted(pat.get(fid, []))
        r["patterns"] = " | ".join(patterns)
        r["owner_decision"] = ",".join(decisions)
        r["readme_top"] = ",".join(sorted(top.get(fid, []), key=int))
        r["phase"] = phase_for(fid, r["doc"], r["module"], decisions, patterns)
        r["lane"] = lane_for(r["module"]) if r["phase"] == "P5" else ""
        prev = old.get(fid, {})
        for c in TRIAGE_COLS:
            r[c] = prev.get(c, "")
        if not r["disposition"]:
            r["disposition"] = "open"
        # A hand-set phase or lane survives a reseed.
        if prev.get("note", "").startswith("phase!"):
            r["phase"] = prev["phase"]
            r["lane"] = prev.get("lane", "")
        out[fid] = r
    write_ledger(out)
    print(f"seeded {len(out)} rows ({len(old)} carried forward)")


def check():
    found = parse_documents()
    led = read_ledger()
    bad = 0

    def fail(msg):
        nonlocal bad
        bad += 1
        print("FAIL", msg)

    for fid in found:
        if fid not in led:
            fail(f"{fid}: in the documents, not in the ledger (run seed)")
    pending = 0
    for fid, r in led.items():
        if fid not in found:
            fail(f"{fid}: in the ledger, not in the documents")
        d = r["disposition"]
        if d not in DISPOSITIONS:
            fail(f"{fid}: unknown disposition {d!r}")
        # A ruled fix awaiting its commit is pending, not defective.
        if d == "open" or (d in ("fix", "fix-amended") and not r["sha"]):
            pending += 1
        if d in ("model", "dispute", "defer", "dup", "fix-amended") and not r["ruling"]:
            fail(f"{fid}: {d} without a ruling reference")
        if d == "defer" and (r["severity"] == "high" or r["class"] == "correctness"):
            fail(f"{fid}: a {r['severity']} {r['class']} entry may not be deferred")
        if r["phase"] not in {"P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9"}:
            fail(f"{fid}: phase {r['phase']!r} is not a phase")
        if r["phase"] == "P5" and r["lane"] in ("", "?"):
            fail(f"{fid}: P5 entry with no lane")
    print(f"{len(led)} rows, {pending} pending, {bad} defects")
    return 1 if bad else 0


def summary():
    led = read_ledger()
    by = lambda key: Counter(r[key] for r in led.values())
    print("by phase:", dict(sorted(by("phase").items())))
    print("by disposition:", dict(by("disposition")))
    print("by severity:", dict(by("severity")))
    print("owner-gated:", sum(1 for r in led.values() if r["owner_gated"] == "yes"))
    print("\nphase x severity:")
    px = Counter((r["phase"], r["severity"]) for r in led.values())
    for p in sorted({k[0] for k in px}):
        print(f"  {p}: " + ", ".join(f"{s}={px[(p, s)]}" for s in ("high", "medium", "low", "nit", "xref") if px[(p, s)]))
    print("\nP5 by lane:")
    for lane, n in sorted(Counter(r["lane"] for r in led.values() if r["phase"] == "P5").items()):
        print(f"  {lane}: {n}")
    print("\nopen by phase:", dict(sorted(Counter(r["phase"] for r in led.values() if r["disposition"] == "open").items())))


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "check"
    if cmd == "seed":
        seed()
    elif cmd == "check":
        sys.exit(check())
    elif cmd == "summary":
        summary()
    else:
        print(__doc__)
        sys.exit(2)
