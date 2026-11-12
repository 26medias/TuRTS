# Claim: hint X3 (JobOnHold(10), D3000) shows in the 14.5 s hold of script C. Test: the longest UNMERGED run of held ticks.
import sys; sys.path.insert(0, sys.argv[1]); import pacing
from sim import run2, static_driver
p = pacing.SETS['new']; tr = pacing.trip(p, (14, 22))
sc = [(8,'ps'),(14,'rifle'),(15,'haul'),(34,'tur'),(40,'rifle'),(41,'rifle'),(42,'tank')]
lg, spans, S, sounds = run2(p, static_driver(sc), T=110, trip=tr)
runs = []; cur = None
for t, cr, hold, nh, low in S['trace']:
    if hold and cur is None: cur = t
    if not hold and cur is not None: runs.append((cur, t)); cur = None
inside = [(round(a, 1), round(b - a, 1)) for a, b in runs if 84 <= a <= 99]
print('merged span (84.1, 98.6): %d unmerged held runs inside; longest run %.1f s; first runs %s' % (len(inside), max([d for _, d in inside] or [0]), inside[:8]))
print('free income per tick %.2f credits; tank charge per tick at full power %.2f credits' % (p['trickle'] * 0.1, 700 / 20 * 0.1))
