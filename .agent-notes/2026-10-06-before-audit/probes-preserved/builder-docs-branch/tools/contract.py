import sys, pathlib
OLD = "`O((|self| + |iter|) log k)` time, `k` the operand count"
NEW = "`O(n log k)` time, `n` the total input bytes and `k` the operand count"
ISLANDS = ["version_join_all", "span_union_all", "span_intersect_all"]
root = pathlib.Path(sys.argv[1])
for isl in ISLANDS:
    p = root / "fuelscape" / f"{isl}.json"
    t = p.read_text()
    assert t.count(OLD) == 1, p
    p.write_text(t.replace(OLD, NEW))
    print("json", p)
for src in ["src/version.rs", "src/clock.rs", "src/span/algebra.rs"]:
    p = root / src
    lines = p.read_text().split("\n")
    n = 0
    for i, line in enumerate(lines):
        if any(f"/fuelscapes/{isl}.html" in line for isl in ISLANDS):
            for j in range(i + 1, i + 4):
                if OLD in lines[j]:
                    lines[j] = lines[j].replace(OLD, NEW); n += 1; break
            else:
                raise SystemExit(f"{src}:{i+1}: no literal follows")
    p.write_text("\n".join(lines))
    print(src, n)
