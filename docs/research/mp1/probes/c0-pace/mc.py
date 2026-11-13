# Monte Carlo of the hint follower: each step reaction = base + U(0, J); four F lists (R3, RRT, T, L3); 150 samples per list.
import sys; sys.path.insert(0, sys.argv[1]); sys.argv = [sys.argv[0], sys.argv[1]]
import random
from opts import *
FL = ('R3', 'RRT', 'T', 'L3')
def mc(p, variant, J=12.0, N=150, seed=7):
    rnd = random.Random(seed); worst = 0; over4 = 0; tot = 0; hist = []
    for fl in FL:
        for _ in range(N):
            react = {k: v + rnd.uniform(0, J) for k, v in follow.BASE_R.items()}
            o = follow.one(p, variant, 0, follow.F_LISTS[fl], T=300, react=react)
            tot += 1; worst = max(worst, o['longest']); over4 += o['longest'] > 4.0; hist.append(o['longest'])
    hist.sort()
    return worst, 100.0 * over4 / tot, hist[int(0.9 * len(hist))], hist[len(hist) // 2]
CASES = [('base rev6', pset(), {}), ('start 1300', pset(start=1300), {}), ('start 1400', pset(start=1400), {}),
         ('income 4/s', pset(trickle=4.0), {}), ('haul 400', pset(cost_haul=400), {}), ('haul 450', pset(cost_haul=450), {}),
         ('rifle 80', pset(cost_rifle=80), {}), ('b1 F after haul2', pset(), {'f_rule': 'haul2'}), ('b3 F after turret', pset(), {'f_rule': 'tur_done'}),
         ('a H4 after PS', pset(), {'h4_needs_ps': True}), ('b2 F after 2nd delivery [new cond]', pset(), {'f_rule': 'deliver2'}),
         ('b3 + start 1300', pset(start=1300), {'f_rule': 'tur_done'}), ('b3 + start 1400', pset(start=1400), {'f_rule': 'tur_done'}),
         ('b1 + start 1300', pset(start=1300), {'f_rule': 'haul2'}), ('start 1300 + income 4', pset(start=1300, trickle=4.0), {}),
         ('start 1500', pset(start=1500), {})]
if __name__ == "__main__":
  for J in (12.0, 30.0):
      print('reaction jitter U(0,%.0f) added to each step' % J)
      for n, p, v in CASES:
          w, o4, p90, med = mc(p, v, J)
          print('  %-36s worst %5.1f s | share of runs above 4 s %5.1f %% | p90 %4.1f | median %4.1f' % (n, w, o4, p90, med))
