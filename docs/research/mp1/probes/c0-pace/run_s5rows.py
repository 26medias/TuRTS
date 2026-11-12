# Claim: which rows of the section 5 results table (D330-D337) change when the start credits change.
import sys, copy; sys.path.insert(0, sys.argv[1]); import pacing
orig = pacing.SETS['new']
S = {'S1': ([(10,'ps'),(40,'rifle'),(41,'rifle'),(42,'rifle'),(45,'tur'),(70,'tank'),(90,'rifle'),(91,'rifle'),(150,'tank'),(200,'launch'),(201,'launch')], 180, {}),
     'S4': ([(10,'ps'),(12,'haul'),(40,'rifle'),(41,'rifle'),(60,'tur'),(100,'tank'),(160,'tank'),(220,'tank')], 180, {}),
     'S5': ([(3,'ps'),(4,'tank'),(5,'tank'),(6,'tank')], 180, {}),
     'S7': ([(8,'ps'),(14,'rifle'),(30,'haul'),(50,'tur'),(60,'rifle'),(61,'rifle'),(75,'tank'),(110,'rifle'),(111,'rifle'),(140,'tank'),(170,'launch'),(171,'launch')], 180, {}),
     'S8': ([(5,'ps'),(5,'tank'),(6,'tank'),(7,'tank'),(8,'tur'),(9,'tur'),(10,'tank'),(11,'tank')], 300, {}),
     'L1b': ([(10,'ps'),(12,'tank'),(40,'tur'),(60,'tank'),(95,'cancel_u'),(95.1,'haul')], 420, {'kill_haulers_at': 90.0}),
     'L1': ([(10,'ps'),(12,'tank'),(40,'tur'),(60,'tank'),(95,'haul')], 420, {'kill_haulers_at': 90.0})}
for st in (1200, 1300, 1400, 1500):
    p = copy.deepcopy(orig); p['start'] = st; out = []
    for k, (sc, T, kw) in S.items():
        lg, w, ev = pacing.run(p, k, sc, [0], T=T, log=False, **kw)
        cr = [e for e in ev]
        out.append('%s %.0f' % (k, lg))
    # credits at 180 s of S1 (design D330: 445)
    import io, contextlib
    b = io.StringIO()
    with contextlib.redirect_stdout(b): pacing.run(p, 'S1', S['S1'][0], [0], T=180, log=True)
    c = [l for l in b.getvalue().split('\n') if 'at 180s' in l]
    print('start %d | longest hold s: %s |%s' % (st, ', '.join(out), c[0].split(', harvested')[0]))
