# The four variants of hold5.py (static scripts) at start 1200 (control) and 1400, plus the worst case of the Monte Carlo at 1400.
import sys, copy, random; sys.path.insert(0, sys.argv[1]); sys.argv = [sys.argv[0], sys.argv[1]]
import pacing, follow
from opts import pset
V = {'A H1-H5 only, haul at 15': [(8,'ps'),(14,'rifle'),(15,'haul'),(34,'tur')],
     'B + F riflemen x3 at 40': [(8,'ps'),(14,'rifle'),(15,'haul'),(34,'tur'),(40,'rifle'),(41,'rifle'),(42,'rifle')],
     'C + F rifle x2 and a tank at 42': [(8,'ps'),(14,'rifle'),(15,'haul'),(34,'tur'),(40,'rifle'),(41,'rifle'),(42,'tank')],
     'D rev-4 order, haul at 30, same F': [(8,'ps'),(14,'rifle'),(30,'haul'),(50,'tur'),(60,'rifle'),(61,'rifle'),(75,'tank')]}
for st in (1200, 1400):
    p = pset(start=st)
    for k, s in V.items():
        lg, w, ev = pacing.run(p, k, s, [0], T=180, log=False, quiet=90, clear_after=15)
        tank = [round(t, 1) for t, e in ev if e.startswith('tank done')]; rif = [round(t, 1) for t, e in ev if e.startswith('rifle done')]
        print('start %d %-36s longest %4.1f s, spans %s, tank done %s, rifle done %s' % (st, k, lg, [(round(a), round(b)) for a, b in w if b - a >= 2], tank, rif))
# worst case of the Monte Carlo at 1400
p = pset(start=1400); rnd = random.Random(7); worst = (0, None)
for fl in ('R3', 'RRT', 'T', 'L3'):
    for _ in range(150):
        react = {k: v + rnd.uniform(0, 12.0) for k, v in follow.BASE_R.items()}
        o = follow.one(p, {}, 0, follow.F_LISTS[fl], T=300, react=react)
        if o['longest'] > worst[0]: worst = (o['longest'], fl, {k: round(v, 1) for k, v in react.items()}, o['spans'])
print('worst at 1400:', worst)
