# Claim: a change of a start number moves the match-length rows of design section 5 (D341-D345) by how much.
import sys; sys.path.insert(0, sys.argv[1])
import pacing, match
cases = [('start 1200 (rev 6)', {}), ('start 1300', {'start': 1300}), ('start 1400', {'start': 1400}), ('start 1500', {'start': 1500}),
         ('income 4/s', {'trickle': 4.0}), ('haul cost 400', {'cost_haul': 400})]
pairs = [('careful', 'careful', 0.9, 0.0, 'careful v careful, B 0.9 (D341: 11.4)'),
         ('careful', 'turtle', 1.0, 0.0, 'careful v turtle (D342: 20.2)'),
         ('aggressor', 'careful', 1.0, 0.0, 'aggressor v careful (D343: 20.2)'),
         ('aggressor', 'turtle', 1.0, 0.0, 'aggressor v turtle (D344: 3.5)'),
         ('careful', 'careful', 1.0, 0.0, 'two equal careful (D345: no end, ore gone 30.4)')]
orig = pacing.SETS['new']
import copy
for name, ov in cases:
    p = copy.deepcopy(orig)
    for k, v in ov.items():
        if k.startswith('cost_'): p['cost'][k[5:]] = v
        else: p[k] = v
    pacing.SETS['new'] = p
    outs = []
    for pa, pb, sk, dl, lab in pairs:
        res, dep, A, B = match.match('new', pa, pb, sk, dl, 1.3)
        outs.append('%s -> %s%s' % (lab.split(' (')[0], res, '' if not dep else ', ore gone %.1f min' % (dep / 60)))
    print('%-20s | %s' % (name, ' | '.join(outs)))
