import sys, copy; sys.path.insert(0, sys.argv[1]); import pacing
orig = pacing.SETS['new']
L1 = [(10,'ps'),(12,'tank'),(40,'tur'),(60,'tank'),(95,'haul')]; L1b = [(10,'ps'),(12,'tank'),(40,'tur'),(60,'tank'),(95,'cancel_u'),(95.1,'haul')]
for st in (1200, 1300, 1400):
    p = copy.deepcopy(orig); p['start'] = st
    r = []
    for n, sc in (('L1b', L1b), ('L1', L1)):
        lg, w, ev = pacing.run(p, n, sc, [0], T=420, log=False, kill_haulers_at=90.0)
        hd = [round(t) for t, e in ev if e.startswith('haul done')]
        r.append('%s new hauler done at %s s' % (n, hd[:1]))
    print('start %d: %s' % (st, ' | '.join(r)))
