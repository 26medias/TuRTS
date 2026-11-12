import sys; sys.path.insert(0, sys.argv[1]); sys.argv = [sys.argv[0], sys.argv[1]]
import opts
from opts import *
O = [
 ('0 rev6 as is', pset(), {}),
 ('0b rev6, F only after H5 (hold5 rule)', pset(), {'f_rule': 'after_h5'}),
 ('a H4 needs PS complete', pset(), {'h4_needs_ps': True}),
 ('a2 H4 needs PS + F after H5 only', pset(), {'h4_needs_ps': True, 'f_rule': 'after_h5'}),
 ('b1 F after 2nd hauler complete', pset(), {'f_rule': 'haul2'}),
 ('b2 F after 2nd hauler delivered [new cond]', pset(), {'f_rule': 'deliver2'}),
 ('b3 F after Turret complete', pset(), {'f_rule': 'tur_done'}),
 ('b4 F when credits >= 700 [new cond]', pset(), {'f_rule': 'cr700'}),
]
for name, p, v in O:
    s = summarize(evaluate(p, v))
    print('%-44s' % name)
    for fl in ('R3', 'RRT'):
        d = s[fl]
        print('    F=%-3s once %5.1f | +10s %5.1f | worst(0..20) %5.1f @%2d | tank %s (+10: %s) | 4 units %s (+10: %s) | spans0 %s' % (fl, d['once'], d['slow10'], d['worst'], d['worst_off'], d['tank'], d['tank10'], d['four'], d['four10'], d['spans0']))
