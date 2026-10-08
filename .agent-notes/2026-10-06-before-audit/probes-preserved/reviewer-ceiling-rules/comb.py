"""Print comb-scatter heap/denominator per sample from a shard log (v3 or v5)."""
import sys
SC = {'3ff0000000000000': 'base', '4010000000000000': 'top'}
for path in sys.argv[1:]:
    head = scale = ver = None
    for line in open(path):
        if line.startswith('HEAD '): head = line.split()[1][:10]
        if line.startswith('amp-board-shard'):
            p = line.split(); ver = p[1]; scale = SC.get(p[-1], p[-1])
        if not line.startswith('cell\t'): continue
        f = line.rstrip('\n').split('\t')
        if f[2] != 'comb-scatter' or 'span' in f[1]: continue
        width = (len(f) - 3) // 2
        for k in range(2):
            s = f[3 + k*width: 3 + (k+1)*width]
            denom = int(s[0])
            if ver == 'v5':
                units = int(s[2].split(':')[0]) if s[2] != '-' else denom; heap = int(s[5])
            elif ver == 'v4':  # denom exp 4 models (heap seg scan touch) 4 readings ...
                units = int(s[2].split(':')[0]) if s[2] != '-' else denom; heap = int(s[6])
            elif ver == 'v3':  # denom exp arity declared_heap heap segments scan touch ...
                units = denom; heap = int(s[4])
            else:
                raise SystemExit('unknown protocol ' + ver)
            print(f"{head} {ver} {f[1]:30s} {scale:4s} s{k+1} heap={heap:8d} units={units:7d} r={heap/units:.6f}")
