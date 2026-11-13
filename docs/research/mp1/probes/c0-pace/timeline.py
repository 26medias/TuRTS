# Item 1: the timeline of hold5.py variants B and C on the model (static scripts through run2; equal to pacing.run, see sim.py self-test).
import sys; sys.path.insert(0, sys.argv[1]); import pacing
from sim import run2, static_driver
p = pacing.SETS['new']; tr = pacing.trip(p, (14, 22)); TRIP = tr[0]
V = {'B (F: 3 Riflemen at 40 s)': [(8,'ps'),(14,'rifle'),(15,'haul'),(34,'tur'),(40,'rifle'),(41,'rifle'),(42,'rifle')],
     'C (F: 2 Riflemen and a Tank)': [(8,'ps'),(14,'rifle'),(15,'haul'),(34,'tur'),(40,'rifle'),(41,'rifle'),(42,'tank')]}
for name, sc in V.items():
    lg, spans, S, sounds = run2(p, static_driver(sc), T=110, trip=tr)
    print('=== %s: longest hold %.1f s, spans %s, "Not enough credits" sounds at %s' % (name, lg, [(round(a,1), round(b,1)) for a, b in spans if b-a>=2], sounds))
    rows = []
    for t, w in S['queued']: rows.append((t, 'queue ' + w))
    for t, w in S['done']: rows.append((t, 'DONE  ' + w))
    for t, e in S['ev']: rows.append((t, '+300 delivery'))
    for a, b in spans:
        if b - a >= 2: rows.append((a, 'HOLD starts')); rows.append((b, 'HOLD ends (%.1f s)' % (b - a)))
    rows.sort()
    print('  %6s  %-20s %8s %12s  %s' % ('t(s)', 'action', 'credits', 'income/s', 'hold'))
    for t, a in rows:
        i = min(int(round(t / 0.1)), len(S['trace']) - 1); tt, cr, hold, nh, low = S['trace'][i]
        inc = p['trickle'] + nh * p['cap'] / TRIP
        print('  %6.1f  %-20s %8.0f %12.1f  %s%s' % (t, a, cr, inc, 'HOLD' if hold else '-', ' low-power' if low else ''))
