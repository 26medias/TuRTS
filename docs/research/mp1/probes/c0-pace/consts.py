# Claim tested: the constants of pacing.py SETS['new'] equal section 5 of design rev 6 (D = design file line).
import sys; sys.path.insert(0, sys.argv[1]); import pacing
p = pacing.SETS['new']
rows = [
 ('start credits', p['start'], 1200, 'D304'),
 ('free income /s', p['trickle'], 3.0, 'D309'),
 ('haul cost', p['cost']['haul'], 500, 'D305'), ('haul time', p['time']['haul'], 15, 'D305'),
 ('haul load cap', p['cap'], 300, 'D306'), ('gather /s', p['gps'], 15, 'D306'), ('unload /s', p['ups'], 60, 'D306'), ('haul speed', p['speed'], 2.4, 'D306'),
 ('rifle time', p['time']['rifle'], 7, 'D310'), ('launch time', p['time']['launch'], 10, 'D311'),
 ('eng cost', p['cost']['eng'], 250, 'D312'), ('tank time', p['time']['tank'], 20, 'D313'),
 ('tur cost', p['cost']['tur'], 600, 'D314'), ('ps time', p['time']['ps'], 12, 'D323'),
 # not in the section 5 table (unchanged GD values, input-game-designer.md:38-39, 54-55)
 ('rifle cost', p['cost']['rifle'], 100, 'GD38'), ('launch cost', p['cost']['launch'], 250, 'GD38'),
 ('tank cost', p['cost']['tank'], 700, 'GD38'), ('ps cost', p['cost']['ps'], 300, 'GD56'), ('tur time', p['time']['tur'], 20, 'GD57'),
 ('eng time', p['time']['eng'], 10, 'GD39'),
]
bad = 0
for n, a, b, ref in rows:
    ok = a == b; bad += (not ok)
    print('%-14s script %-6s design %-6s %-6s %s' % (n, a, b, ref, 'OK' if ok else 'MISMATCH'))
print('power rule in pacing.py: low = 40 + 30*tur > 100*ps, sp 0.5 (design D323, D507-D540 Completed counts, GD power 40/100/30)')
print('7.9 Not-enough-credits: cooldown 90, clear_after 15 (design 7.9 table) -> hold5.py uses quiet=90, clear_after=15')
print('mismatches:', bad)
