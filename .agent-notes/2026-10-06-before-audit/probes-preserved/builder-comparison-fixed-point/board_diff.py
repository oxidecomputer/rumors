import re, sys
def cells(path):
    out, section = {}, None
    for line in open(path, errors="replace"):
        if line.startswith("=== "):
            section = line.strip()
            continue
        m = re.match(r"^(GREEN|RED)\s+(\S+)\s+(\S+)\s+(.*)$", line.rstrip("\n"))
        if m and section:
            key = (section, m.group(2), m.group(3))
            assert key not in out, key
            out[key] = (m.group(1), re.sub(r"\s+", " ", m.group(4)))
    return out
a, b = cells(sys.argv[1]), cells(sys.argv[2])
print(f"cells parent {len(a)} tip {len(b)} same keys {set(a)==set(b)}")
moved = [k for k in a if k in b and a[k] != b[k]]
print(f"moved cells: {len(moved)}")
for k in moved:
    print(" ", k); print("    parent:", a[k]); print("    tip:   ", b[k])
