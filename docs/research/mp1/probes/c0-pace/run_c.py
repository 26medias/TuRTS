import sys; sys.path.insert(0, sys.argv[1]); sys.argv = [sys.argv[0], sys.argv[1]]
from opts import *
def line(name, p, v={}):
    s = summarize(evaluate(p, v)); a, b = s['R3'], s['RRT']
    print('%-30s R3 once %4.1f worst %4.1f | RRT once %4.1f worst %4.1f | tank %s 4u(R3) %s' % (name, a['once'], a['worst'], b['once'], b['worst'], b['tank'], a['four']))
    return s
print('-- start credits'); 
for s0 in (1200, 1300, 1400, 1500, 1600, 1700, 1800): line('start %d' % s0, pset(start=s0))
print('-- free income per second')
for tr in (3, 4, 5, 6, 8): line('income %d/s' % tr, pset(trickle=float(tr)))
print('-- hauler cost')
for hc in (500, 450, 400, 350, 300): line('haul cost %d' % hc, pset(cost_haul=hc))
print('-- rifleman cost')
for rc in (100, 80, 70, 60, 50): line('rifle cost %d' % rc, pset(cost_rifle=rc))
print('-- turret cost (info only: not on the list of the task)')
for tc in (600, 500, 400): line('turret cost %d' % tc, pset(cost_tur=tc))
