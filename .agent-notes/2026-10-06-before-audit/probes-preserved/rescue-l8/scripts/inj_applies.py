"""Count each wasm32 injection's search strings in a git revision's files."""
import subprocess, sys
sys.path.insert(0, sys.argv[1])
from injections import INJ_ALL, INJ
rev = sys.argv[2]
for group, table in (("narrowing", INJ_ALL), ("trap", INJ)):
    for id_, f, swaps, flt in table:
        path = "crates/" + f
        try:
            text = subprocess.run(["git", "-C", "/Users/oxide/src/rumors", "show", f"{rev}:{path}"],
                                  capture_output=True, text=True, check=True).stdout
        except subprocess.CalledProcessError:
            print(f"{group:9} {id_:24} FILE MISSING {path}")
            continue
        verdicts = []
        for old, new, n in swaps:
            c = text.count(old)
            verdicts.append("ok" if c == n else f"count {c} != {n}")
        print(f"{group:9} {id_:24} {', '.join(verdicts)}")
