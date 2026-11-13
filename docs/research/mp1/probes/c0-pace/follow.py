# Hint follower on the stage-1 hint table (design Player path D2988-D3001; selection rule 7.8 D1993-D1996).
import sys, copy
sys.path.insert(0, sys.argv[1]); import pacing
sys.path.insert(0, sys.path[0].rsplit('/', 0)[0])
from sim import run2, COMBAT, STRUCT

BASE_R = {'H1': 8, 'H2': 6, 'H4': 1, 'H5': 2, 'H6': 2, 'F': 6}   # reproduces script B/C of hold5.py (8,14,15,34,40)
F_ITEMS = {'rifle', 'launch', 'tank'}
F_LISTS = {'TT': ['tank', 'tank'], 'L3': ['launch'] * 3, 'R5': ['rifle'] * 5, 'R3': ['rifle'] * 3, 'R3x': ['rifle'] * 3, 'RRT': ['rifle', 'rifle', 'tank'], 'T': ['tank'], 'RRR2': ['rifle'] * 2}

def make_driver(variant, slow, flist, batches=1, react=BASE_R):
    off = 10 if slow is True else (0 if slow is False else slow)
    r = {k: v + off for k, v in react.items()}
    st = dict(h1=False, h2=False, h4=False, h5=False, h6=False, shown=None, since=0.0, pending=[], batches=0, f_ep=None,
              h6_t=None, shown_log=[])
    def combat_done(S): return sum(1 for _, w in S['done'] if w in COMBAT)
    def haul_done(S): return 1 + sum(1 for _, w in S['done'] if w == 'haul')
    def can_f(S):
        rule = variant.get('f_rule', 'fallback')
        if rule == 'fallback': return True
        if rule == 'after_h5': return st['h5']                      # what hold5.py measured
        if rule == 'haul2': return haul_done(S) >= 2               # OwnUnits(ore_hauler, Complete, 2)
        if rule == 'tur_done': return S['tur'] >= 1               # OwnUnits(guard_turret, Complete, 1)
        if rule == 'none': return False
        if rule == 'deliver2':                                      # NEW HintCondition (not in 6.10 row H14)
            th = [t for t, w in S['done'] if w == 'haul']
            return bool(th) and S['t'] >= th[0] + variant['first']
        if rule.startswith('cr'): return S['cr'] >= float(rule[2:])  # NEW HintCondition (not in 6.10 row H14)
        raise ValueError(rule)
    def current(S):
        if not st['h1']: return 'H1'
        if not st['h2']: return 'H2'
        h4_start = st['h2'] and (S['ps'] >= 1 if variant.get('h4_needs_ps') else True)
        if not st['h4'] and h4_start: return 'H4'
        if st['h4'] and not st['h5'] and S['ps'] >= 1: return 'H5'
        if st['h5'] and not st['h6'] and combat_done(S) >= 4: return 'H6'
        if st['h6']: return None
        return 'F' if can_f(S) else None
    def driver(t, S):
        out = []
        cur = current(S)
        if cur != st['shown']:
            st['shown'] = cur; st['since'] = t; st['shown_log'].append((round(t, 1), cur)); st['pending'] = []
            if cur == 'F':
                if st['batches'] < batches:
                    st['pending'] = [(t + r['F'] + j, w) for j, w in enumerate(flist)]
            elif cur in r:
                st['pending'] = [(t + r[cur], cur)]
            if cur == 'H6' and st['h6_t'] is None: st['h6_t'] = t
        keep = []
        for at, what in st['pending']:
            if at <= t + 1e-9:
                if what == 'H1': out.append('ps'); st['h1'] = True
                elif what == 'H2': out.append('rifle'); st['h2'] = True
                elif what == 'H4': out.append('haul'); st['h4'] = True
                elif what == 'H5': out.append('tur'); st['h5'] = True
                elif what == 'H6': st['h6'] = True
                else:
                    out.append(what)
                    if what in F_ITEMS and not st.get('counted_' + str(st['batches'])):
                        st['counted_' + str(st['batches'])] = True; st['batches'] += 1
            else: keep.append((at, what))
        st['pending'] = keep
        return out
    driver.st = st
    return driver

def one(p, variant, slow, flist, T=240, batches=1, react=BASE_R):
    tr = pacing.trip(p, (14, 22)); variant = dict(variant, first=tr[1])
    d = make_driver(variant, slow, flist, batches, react)
    lg, spans, S, sounds = run2(p, d, T=T, trip=tr)
    tank = [round(t, 1) for t, w in S['done'] if w == 'tank']
    comb = [round(t, 1) for t, w in S['done'] if w in COMBAT]
    h6 = d.st['h6_t']
    return dict(longest=lg, spans=[(round(a), round(b)) for a, b in spans if b - a >= 2], tank=tank[0] if tank else None,
                four=comb[3] if len(comb) >= 4 else None, h6=h6, sounds=sounds, shown=d.st['shown_log'], S=S)
