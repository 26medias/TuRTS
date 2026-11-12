import sys; sys.path.insert(0, sys.argv[1]); sys.argv = [sys.argv[0], sys.argv[1]]
from opts import *
import mc
def h2(o):
    t = [round(a, 1) for a, w in o['S']['done'] if w == 'haul']; return t[0] if t else None
OPT = [
 ('d  accept (rev 6 as is)', pset(), {}),
 ('a  H4 after PS complete', pset(), {'h4_needs_ps': True}),
 ('b1 F after 2nd hauler complete', pset(), {'f_rule': 'haul2'}),
 ('b2 F after 2nd hauler 1st delivery [NEW cond]', pset(), {'f_rule': 'deliver2'}),
 ('b3 F after Turret complete', pset(), {'f_rule': 'tur_done'}),
 ('c1 start 1300', pset(start=1300), {}),
 ('c2 start 1400', pset(start=1400), {}),
 ('c3 income 4/s', pset(trickle=4.0), {}),
 ('c4 hauler cost 400', pset(cost_haul=400), {}),
 ('c5 start 1300 + F after Turret', pset(start=1300), {'f_rule': 'tur_done'}),
]
print('%-46s | %-34s | %-34s | tank | 4u(R3) | haul2 | MC J=12: worst / >4s' % ('option', 'R3: once / +10 / worst0..30', 'RRT: once / +10 / worst0..30'))
for n, p, v in OPT:
    res = evaluate(p, v); s = summarize(res)
    a, b = s['R3'], s['RRT']
    w, o4, p90, med = mc.mc(p, v, 12.0)
    print('%-46s | %5.1f / %5.1f / %5.1f              | %5.1f / %5.1f / %5.1f              | %5s | %5s | %5s | %5.1f s / %4.1f %%' % (n, a['once'], a['slow10'], a['worst'], b['once'], b['slow10'], b['worst'], b['tank'], a['four'], h2(res['R3'][0]), w, o4))
