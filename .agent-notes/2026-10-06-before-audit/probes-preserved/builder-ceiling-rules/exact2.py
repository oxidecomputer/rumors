import math, struct
scale = None
out = []
for line in open('exact-columns-2.log'):
    if line.startswith('amp-board-shard'):
        scale = {'3ff0000000000000': 'base', '4010000000000000': 'top'}[line.split()[-1]]
    if not line.startswith('cell\t'):
        continue
    f = line.rstrip('\n').split('\t')
    op, fam = f[1], f[2]
    # per sample: denom, exp_denom, 3 models, 3 readings, 3 floors = 11 fields
    for k, s in enumerate((f[3:14], f[14:25])):
        model = s[2]
        units = int(model.split(':')[1]) if model != '-' else int(s[0])
        ceil = struct.unpack('>d', bytes.fromhex(model.split(':')[2]))[0] if model != '-' and model.split(':')[2] != '-' else None
        heap = int(s[5])
        out.append((heap / units, op, fam, scale, k + 1, units, heap, ceil))
for op in ('version_borsh_deserialize', 'span_borsh_deserialize', 'own_version_to_version', 'own_span_to_span', 'clock_own_version_to_version'):
    qual = [o for o in out if o[1] == op and o[5] >= 2048]
    print(op, "samples >= 2048 units, top 5 by reading:")
    for o in sorted(qual, reverse=True)[:5]:
        print("   %.6f  %s x %s [%s s%d] heap %d / %d units  ceiling %s  -> 1.25x = %.6f -> %d" % (o[0], o[1], o[2], o[3], o[4], o[6], o[5], o[7], o[0]*1.25, math.ceil(o[0]*1.25)))
    print(op, "benign/sub-floor check:", [("%.4f" % o[0], o[2], o[3], o[4], o[5]) for o in out if o[1] == op and o[2] == 'benign' and o[3] == 'base'])
