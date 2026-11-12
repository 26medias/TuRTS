import sys; sys.argv = [sys.argv[0], sys.argv[1]]
sys.path.insert(0, sys.argv[1])
import follow, pacing, copy
p = pacing.SETS['new']
VAR = {'rev6 (F fallback shown at once)': {}, 'rev6, F only after H5 (as hold5.py)': {'f_rule': 'after_h5'}}
for name, v in VAR.items():
    for fl in ('R3', 'RRT'):
        for slow in (False, True):
            o = follow.one(p, v, slow, follow.F_LISTS[fl])
            print('%-38s F=%-3s %-5s longest %5.1f spans %s tank %s four %s sounds %s' % (name, fl, 'slow' if slow else 'once', o['longest'], o['spans'], o['tank'], o['four'], o['sounds']))
