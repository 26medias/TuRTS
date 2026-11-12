import sys; sys.path.insert(0, sys.argv[1])
import follow, pacing, copy
BASE = pacing.SETS['new']
def pset(**kw):
    p = copy.deepcopy(BASE)
    for k, v in kw.items():
        if k.startswith('cost_'): p['cost'][k[5:]] = v
        elif k.startswith('time_'): p['time'][k[5:]] = v
        else: p[k] = v
    return p
OFFS = list(range(0, 31, 2))
def evaluate(p, variant):
    res = {}
    for fl in ('R3', 'RRT'):
        row = {}
        for off in OFFS:
            o = follow.one(p, variant, off, follow.F_LISTS[fl])
            row[off] = o
        res[fl] = row
    return res
def summarize(res):
    out = {}
    for fl, row in res.items():
        out[fl] = dict(once=row[0]['longest'], slow10=row[10]['longest'], worst=max(r['longest'] for r in row.values()),
                       worst_off=max(row, key=lambda k: row[k]['longest']),
                       tank=row[0]['tank'], four=row[0]['four'], tank10=row[10]['tank'], four10=row[10]['four'],
                       spans0=row[0]['spans'], sounds0=row[0]['sounds'], shown0=row[0]['shown'])
    return out
