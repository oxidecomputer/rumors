#!/usr/bin/env python3
"""Triage ledger for the before and suanpan review: seed, check, and summarize.

The eight class documents (`../correctness.md` and siblings) are the record
of what was found. This ledger is the record of what was decided about each
entry. `ledger.tsv` has one row per finding id; the seed columns are derived
from the documents and this script regenerates them, the triage columns are
filled in by hand as rulings land and lanes close. `TRIAGE.md` explains the
phases, lanes, and dispositions the columns refer to; `rulings.md` holds the
numbered rulings the `ruling` column cites.

    ledger.py seed     rebuild the seed columns, preserving triage columns
    ledger.py check    verify coverage and vocabulary; exit 1 on any defect
    ledger.py summary  counts by phase, disposition, and severity

Seed columns: id, doc, severity, class, provenance, witness, owner_gated,
module, patterns, owner_decision, readme_top, phase (proposed), lane
(proposed), title. Triage columns: disposition, ruling, sha, note.

Coverage is the point: the cheapest way to "finish" a 1,200-entry triage is
to disposition the highs and forget the lows, so `check` fails while any id
in the documents lacks a row, and refuses the dispositions the doctrine
forbids (a deferred correctness defect, a model ruling that names no ruling).
A `fix` without a landed sha is pending, not defective; triage closes when
pending and defects are both zero.
"""

import csv
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
NOTE = HERE.parent
LEDGER = HERE / "ledger.tsv"
DOCS = ["correctness", "claims", "verification", "documentation", "simplification",
        "performance", "api", "dependence"]

SEED_COLS = ["id", "doc", "severity", "class", "provenance", "witness", "owner_gated", "module",
             "patterns", "owner_decision", "readme_top", "phase", "lane", "title"]
TRIAGE_COLS = ["disposition", "ruling", "sha", "note"]
COLS = SEED_COLS + TRIAGE_COLS

# Disposition vocabulary. `fix` and `fix-amended` are terminal only with a sha
# (pending until then). `model`, `dispute`, and `dup` are terminal with a
# ruling reference. `defer` is terminal with a ruling naming the home, and is
# never admitted for a high-severity or correctness-class entry.
DISPOSITIONS = {"fix", "fix-amended", "model", "defer", "dispute", "dup", "open"}
NEVER_DEFER_CLASSES = {"correctness", "claim"}

# Partition keys may carry a digit (`meter-registry-tier2`); the trailing
# number is the finalizer's own numbering.
ID_PAT = r"[a-z][a-z0-9]*(?:-[a-z][a-z0-9]*)*-\d+"
ID_RE = re.compile(r"\b(" + ID_PAT + r")\b")


# README owner-decision number -> proposed phase (see TRIAGE.md, "Phases").
def decision_phase(n):
    n = int(n)
    table = {
        1: "P1", 21: "P2", 22: "P2", 23: "P2", 24: "P2", 25: "P2", 26: "P2", 27: "P2",
        28: "P2", 29: "P2", 30: "P2", 31: "P1", 32: "P2", 33: "P1", 34: "P2",
        35: "P4", 36: "P4", 37: "P4", 38: "P8", 39: "P8", 40: "P8", 41: "P8",
        42: "P5", 43: "P5", 44: "P5", 45: "P8", 46: "P5", 47: "P5", 48: "P5", 49: "P5",
        50: "P5", 51: "P4", 52: "P4", 53: "P1", 54: "P1", 55: "P1", 56: "P1", 57: "P1",
        58: "P1", 59: "P1", 60: "P1", 61: "P1", 62: "P1", 63: "P1", 64: "P1", 65: "P1",
        66: "P6", 67: "P1", 68: "P1", 69: "P4", 70: "P4", 71: "P8", 72: "P6", 73: "P4",
        74: "P1", 75: "P1", 76: "P1", 77: "P5", 78: "P5", 79: "P5", 80: "P6",
        85: "P4", 86: "P4", 88: "P4", 89: "P4", 90: "P4", 92: "P9",
    }
    if n in table:
        return table[n]
    if 2 <= n <= 20:
        return "P7"
    if 81 <= n <= 91:
        return "P3"
    return "P6"


# Crate-wide pattern label -> proposed phase. Anything not listed falls to
# the pattern's document default below.
PATTERN_PHASE = {
    "correctness: Narrowing conversions and fixed-width indices whose bound is asserted rather than derived": "P2",
    "correctness: The marker-bit size law was not carried into the board's adapters": "P4",
    "correctness: Instruments that measure a short-circuit under an adversarial label": "P1",
    "correctness: Checks that prove less than the sentence beside them": "P4",
    "correctness: Documented panics reachable from input, and documented silence that panics": "P2",
    "correctness: Provenance of the gate and CI is not bound to the pins it judges": "P1",
    "claims: Summary claims outrun their per-operation contracts": "P4",
    "claims: Per-level re-work hidden inside \"linear\"": "P2",
    "claims: The committed instrument measures the benign case": "P1",
    "claims: Meter blind spots decide what can be claimed": "P1",
    "claims: Flag-day residue in cost prose": "P4",
    "claims: The hard-guarantee sentence and the meter header, reconciled": "P2",
    "claims: Numbers without artifacts": "P4",
    "verification: A judged column with no writer": "P5",
    "verification: A ceiling with no liveness floor": "P1",
    "verification: A measurement that stops at the early exit": "P1",
    "verification: Hand rosters checked only against each other": "P4",
    "verification: Known-bad demonstrations that live in prose or in a commit message": "P1",
    "verification: Rosters that attest a name, not a run": "P4",
    "verification: Framing and constants transcribed across the detached-workspace boundary": "P4",
    "verification: Process-isolation as an unchecked premise": "P1",
    "api: The meter-surface stability question decides several gates": "P1",
    "dependence: Contracts stated in tests, not at the method": "P4",
    "dependence: The meter-gated instrument surface is rumors' second API, with an unsettled stability status": "P1",
    "dependence: The ledger records values; rumors also relies on costs": "P2",
    "dependence: Cross-crate citations run unchecked in both directions": "P4",
    "simplification: Machinery that outlived the constraint that justified it": "P5",
    "simplification: One mechanism, several hand copies": "P4",
    "simplification: Instruments with no reader, and buffers waiting for failures": "P5",
    "simplification: Guards that recompute what committed tests already hold": "P4",
    "simplification: Rosters and literals the compiler could hold": "P4",
    "simplification: Things one module away from home": "P6",
    "simplification: Vocabulary and register": "P3",
    "documentation: Vocabulary without an anchor": "P3",
    "documentation: \"mint\" for constructing a value": "P3",
    "documentation: Moralized code and register transplants": "P3",
    "documentation: Em-dashes in `//` comments": "P3",
    "documentation: Residual dialect tells and hand counts from the census": "P3",
}
PATTERN_DOC_DEFAULT = {"correctness": "P2", "claims": "P2", "verification": "P4",
                       "documentation": "P4", "simplification": "P4", "performance": "P8",
                       "api": "P7", "dependence": "P4"}
# An entry in no pattern and no owner decision: by document.
DOC_DEFAULT = {"correctness": "P2", "claims": "P2", "verification": "P6", "documentation": "P6",
               "simplification": "P6", "performance": "P8", "api": "P7", "dependence": "P4"}
# Module headings whose entries default to a phase rather than a module lane.
MODULE_PHASE = [
    ("Dissolvable scaffolding", "P5"),
    ("The enforcement chain", "P1"),
    # The prose-hygiene census entries sit inside the documentation
    # document's pattern section; they are the vocabulary sweeps of phase 3.
    ("Crate-wide patterns", "P3"),
]
# Entries placed by hand: the sixteen high-severity entries and the gate holes.
HAND_PHASE = {
    "suanpan-24": "P2", "skyline-coding-9": "P2", "rank-33": "P2", "skyline-sweep-place-masked-5": "P2",
    "span-causally-36": "P2", "skyline-fill-grow-2": "P2", "surface-roster-28": "P2",
    "gate-legs-1": "P1", "deps-1": "P1", "board-families-floors-judge-21": "P1",
    "board-ops-render-15": "P5", "board-frame-1": "P5", "meter-core-11": "P5",
    "envelopes-a-1": "P4", "meter-registry-tier2-14": "P4",
    "benches-examples-18": "P4", "party-3": "P4", "surface-roster-20": "P4",
    "gate-legs-2": "P1", "surface-roster-1": "P1", "fuzzfit-strategies-13": "P1",
}

# Module heading (as the class documents spell it) -> lane, first match wins.
LANE_RULES = [
    ("suanpan", "suanpan"), ("codec", "codec"), ("skyline", "skyline"),
    ("Crate root and public types", "core"), ("Cross-cutting", "core"),
    ("oracle and laws", "board"), ("meter core", "board"), ("family registry", "board"),
    ("amplification board", "board"), ("surface roster", "surface"),
    ("test harness", "harness"), ("envelopes", "harness"), ("other suites", "harness"),
    ("benches", "benches"), ("fuzz", "fuzz"), ("fuelscape", "fuelscape"),
    ("gate recipes", "tools"), ("Workspace tools", "tools"), ("Workspace sweeps", "tools"),
    ("Other cross-cutting sweeps", "tools"), ("The instruments", "board"),
]


def lane_for(module):
    for key, lane in LANE_RULES:
        if key.lower() in module.lower():
            return lane
    return "?"


def phase_for(fid, doc, module, decisions, patterns):
    if fid in HAND_PHASE:
        return HAND_PHASE[fid]
    for key, phase in MODULE_PHASE:
        if module.startswith(key):
            return phase
    cands = [decision_phase(n) for n in decisions]
    for p in patterns:
        cands.append(PATTERN_PHASE.get(p, PATTERN_DOC_DEFAULT[p.split(":")[0]]))
    if not cands:
        return DOC_DEFAULT[doc]
    return min(cands)


def parse_documents():
    """Every finding entry and nit-table row in the eight class documents."""
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
            m = re.match(r"^#{3,4} (" + ID_PAT + r"): (.*)", line)
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
                prov = meta.get("Provenance", "").split("(")[0].split(";")[0].split(" ")[0].strip()
                witness = meta.get("Witness", "").split(" ")[0].split("(")[0].strip(" .:;,")
                og = meta.get("Owner-gated", "").lower()
                og = "yes" if og.startswith("yes") else ("no" if og else "")
                rows[fid] = dict(id=fid, doc=d, severity=sev, **{"class": cls}, provenance=prov,
                                 witness=witness, owner_gated=og, module=module, title=title)
                i = j
                continue
            m = re.match(r"^\| (" + ID_PAT + r") \| (.*?) \| (.*?) \|", line)
            if m and module:
                fid, _where, claim = m.groups()
                rows[fid] = dict(id=fid, doc=d, severity="nit", **{"class": "(table)"},
                                 provenance="", witness="", owner_gated="", module=module,
                                 title=claim)
            i += 1
    return rows


def parse_patterns(known):
    pat = defaultdict(set)
    for d in DOCS:
        text = (NOTE / f"{d}.md").read_text()
        m = re.search(r"^## Crate-wide patterns\n(.*?)^## ", text, re.S | re.M)
        if not m:
            continue
        for chunk in re.split(r"(?m)^(?=- \*\*|\*\*)", m.group(1)):
            h = re.match(r"- \*\*(.*?)\*\*|\*\*(.*?)\*\*", chunk)
            if not h:
                continue
            label = next(g for g in h.groups() if g).rstrip(". ")
            # A pattern paragraph may embed full census entries; stop at the first.
            body = re.split(r"(?m)^### ", chunk)[0]
            for fid in set(ID_RE.findall(body)):
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
        r["lane"] = lane_for(r["module"]) if r["phase"] == "P6" else ""
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
        if d == "open" or (d in ("fix", "fix-amended") and not r["sha"]):
            pending += 1
        if d in ("model", "dispute", "defer", "dup", "fix-amended") and not r["ruling"]:
            fail(f"{fid}: {d} without a ruling reference")
        if d == "defer" and (r["severity"] == "high" or r["class"] in NEVER_DEFER_CLASSES):
            fail(f"{fid}: a {r['severity']} {r['class']} entry may not be deferred")
        if r["phase"] not in {f"P{i}" for i in range(1, 10)}:
            fail(f"{fid}: phase {r['phase']!r} is not a phase")
        if r["phase"] == "P6" and r["lane"] in ("", "?"):
            fail(f"{fid}: P6 entry with no lane")
    print(f"{len(led)} rows, {pending} pending, {bad} defects")
    return 1 if bad else 0


def summary():
    led = read_ledger()
    by = lambda key: Counter(r[key] for r in led.values())
    print("by phase:", dict(sorted(by("phase").items())))
    print("by disposition:", dict(by("disposition")))
    print("by severity:", dict(by("severity")))
    print("owner-gated:", sum(1 for r in led.values() if r["owner_gated"] == "yes"))
    print("witnessed:", dict(by("witness")))
    print("\nphase x severity:")
    px = Counter((r["phase"], r["severity"]) for r in led.values())
    for p in sorted({k[0] for k in px}):
        print(f"  {p}: " + ", ".join(f"{s}={px[(p, s)]}" for s in ("high", "medium", "low", "nit") if px[(p, s)]))
    print("\nP6 by lane:")
    for lane, n in sorted(Counter(r["lane"] for r in led.values() if r["phase"] == "P6").items()):
        print(f"  {lane}: {n}")
    print("\npending by phase:", dict(sorted(Counter(
        r["phase"] for r in led.values()
        if r["disposition"] == "open" or (r["disposition"] in ("fix", "fix-amended") and not r["sha"])).items())))


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
