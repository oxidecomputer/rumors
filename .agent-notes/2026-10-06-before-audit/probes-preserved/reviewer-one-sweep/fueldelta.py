import re,sys
def parse(p):
    t=open(p).read()
    out={}
    for m in re.finditer(r'Band \{(\s*kernel.*?)\n\s*\}', t, re.S):
        b=m.group(1)
        k=re.search(r'kernel: "([^"]+)"',b).group(1)
        rej=re.search(r'rejected: (\w+)',b).group(1)
        sl=float(re.search(r'slope: ([-\d.]+)',b).group(1))
        ic=float(re.search(r'intercept: ([-\d.]+)',b).group(1))
        lo=int(re.search(r'min_denom: (\d+)',b).group(1)); hi=int(re.search(r'max_denom: (\d+)',b).group(1))
        out[(k,rej,len(out))]=(sl,ic,lo,hi)
    return out
import math
a=parse(sys.argv[1]); c=parse(sys.argv[2])
for key in a:
    if key not in c or a[key][:2]==c[key][:2]: continue
    sa,ia,lo,hi=a[key]; sc,ic,_,_=c[key]
    def pct(d): return (10**((ic+sc*math.log10(d))-(ia+sa*math.log10(d)))-1)*100
    print(f"{key[0]:28s} rej={key[1]:5s} denom {lo:>7}..{hi:>8}: {pct(lo):+.2f}% .. {pct(hi):+.2f}%  (at128 {pct(128):+.2f}%)")
