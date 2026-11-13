# MP1 pacing model, repair pass (ux-designer with the game-designer pen).
# Base: docs/research/mp1/probes/player/play.py (player seat, gate 1). Same step model:
# charge over time, floor(cost x progress) per step, a step that cannot pay does not advance,
# low power halves the speed of both queues. Added: hauler geometry from economy.py
# (gather.rs reach), a free income of the command centre, the alert rule of design 7.9,
# and a hauler loss.
# Usage: python3 -I pacing.py [old|new]
import math, sys

SETS = {
    # GD numbers (input-game-designer.md:37-64, 99-117); control, must reproduce play.out holds.
    'old': dict(start=1200, trickle=0.0,
                cost={'rifle':100,'launch':250,'eng':300,'haul':800,'tank':700,'ps':300,'tur':500},
                time={'rifle':6,'launch':9,'eng':10,'haul':20,'tank':18,'ps':12,'tur':20},
                cap=200, gps=10, ups=40, speed=2.0, overhead=1.10),
    # Repair proposal (design section 5, "Numbers after gate 1").
    'new': dict(start=1200, trickle=3.0,
                cost={'rifle':100,'launch':250,'eng':250,'haul':500,'tank':700,'ps':300,'tur':600},
                time={'rifle':7,'launch':10,'eng':10,'haul':15,'tank':20,'ps':12,'tur':20},
                cap=300, gps=15, ups=60, speed=2.4, overhead=1.10),
}
STRUCT = {'ps', 'tur'}
DT = 0.1
CC0 = (16, 36)
HR, NR, CCR, IR = 0.9, 1.0, 2.5, 0.5   # economy.py: hauler, node, CC radius, interaction range

def trip(p, node):
    d = math.dist(CC0, node)
    travel = max(0.0, d - (HR + NR + IR) - (HR + CCR + IR))
    t = (2 * travel / p['speed'] + p['cap'] / p['gps'] + p['cap'] / p['ups']) * p['overhead']
    first = t - p['cap'] / p['ups']   # starts beside the CC, empty (economy.py rule)
    return t, first

def run(p, name, script, hauler_orders, T=180, log=True, kill_haulers_at=None,
        quiet=60.0, clear_after=10.0):
    TRIP, FIRST = trip(p, (14, 22))   # nearest main node
    COST, TIME = p['cost'], p['time']
    cr = float(p['start']); t = 0.0; q = {'s': [], 'u': []}; act = {'s': None, 'u': None}
    haulers = [h + FIRST for h in hauler_orders]
    ps = 0; tur = 0; events = []; waits = []; cur_wait = None; harvested = 0; trick = 0.0
    si = 0; script = sorted(script, key=lambda e: e[0])
    # alert model, design 7.9: a state alert sounds when the state starts, if the last sound of the
    # rule is at least `quiet` s old; the state counts as ended only after `clear_after` s clear.
    sounds = []; state_on = False; clear_since = None; last_sound = -1e9; naive_sounds = 0
    naive_prev = False
    while t < T - 1e-9:
        while si < len(script) and script[si][0] <= t + 1e-9:
            _, what = script[si]; si += 1
            if what == 'cancel_u':   # cancel the active unit job: refund what it paid (ratio 1/1)
                if act['u'] is not None:
                    cr += act['u'][2]; events.append((t, 'cancel %s, refund %d, credits %d' % (act['u'][0], act['u'][2], cr)))
                    act['u'] = None
                q['u'] = []
                continue
            q['s' if what in STRUCT else 'u'].append(what)
        if kill_haulers_at is not None and abs(t - kill_haulers_at) < DT / 2:
            haulers = []; events.append((t, 'all haulers destroyed'))
        nh = []
        for nd in haulers:
            if t >= nd - 1e-9:
                cr += p['cap']; harvested += p['cap']; events.append((t, '+%d delivery, credits %d' % (p['cap'], cr))); nd += TRIP
            nh.append(nd)
        haulers = nh
        cr += p['trickle'] * DT; trick += p['trickle'] * DT
        low = 40 + 30 * tur > 100 * ps
        sp = 0.5 if low else 1.0
        anyhold = False
        for k in ('s', 'u'):
            if act[k] is None and q[k]:
                act[k] = [q[k].pop(0), 0.0, 0]
            if act[k] is None: continue
            w, pr, paid = act[k]
            np_ = min(1.0, pr + DT * sp / TIME[w]); want = math.floor(COST[w] * np_) - paid
            if want <= cr:
                cr -= want; paid += want; pr = np_; act[k] = [w, pr, paid]
            else:
                anyhold = True
            if pr >= 1.0 - 1e-9:
                events.append((t + DT, '%s done, credits %d' % (w, cr)))
                if w == 'ps': ps += 1
                if w == 'tur': tur += 1
                if w == 'haul': haulers.append(t + DT + FIRST)   # rally on ore: gathers at once
                act[k] = None
        if anyhold and cur_wait is None: cur_wait = t
        if not anyhold and cur_wait is not None: waits.append((cur_wait, t)); cur_wait = None
        # naive rule (rev 3 ALR2): one sound each time the state starts
        if anyhold and not naive_prev: naive_sounds += 1
        naive_prev = anyhold
        # design 7.9 rule
        if anyhold:
            clear_since = None
            if not state_on:
                state_on = True
                if t - last_sound >= quiet:
                    sounds.append(round(t)); last_sound = t
        elif state_on:
            if clear_since is None: clear_since = t
            if t - clear_since >= clear_after: state_on = False
        t += DT
    if cur_wait is not None: waits.append((cur_wait, T))
    # a starved job pays in some ticks and holds in others (free income); merge holds with gaps < 2 s
    merged = []
    for a, b in waits:
        if merged and a - merged[-1][1] < 2.0: merged[-1] = (merged[-1][0], b)
        else: merged.append((a, b))
    waits = merged
    longest = max([b - a for a, b in waits] or [0])
    if log:
        print('=== %s (trip %.1f s, first delivery %.1f s, %.0f credits/min per hauler)' % (name, TRIP, FIRST, p['cap'] / TRIP * 60))
        for e in events: print('  %6.1fs %s' % e)
        print('  at %ds: credits %d, harvested %d, free income %d' % (T, cr, harvested, trick))
        print('  holds >= 2 s (start-end s):', [(round(a), round(b)) for a, b in waits if b - a >= 2])
        print('  longest hold (merged) %.1f s; hold spans %d; "Not enough credits" sounds: rev3 rule %d, 7.9 rule %d at %s'
              % (longest, len(waits), naive_sounds, len(sounds), sounds))
    return longest, waits, events

def main(which):
    p = SETS[which]
    s1 = [(10,'ps'),(40,'rifle'),(41,'rifle'),(42,'rifle'),(45,'tur'),(70,'tank'),(90,'rifle'),(91,'rifle'),(150,'tank'),(200,'launch'),(201,'launch')]
    run(p, which + ' S1 hint follower, hauler gathers from tick 0 (D-F)', s1, [0])
    s4 = [(10,'ps'),(12,'haul'),(40,'rifle'),(41,'rifle'),(60,'tur'),(100,'tank'),(160,'tank'),(220,'tank')]
    run(p, which + ' S4 power + second hauler', s4, [0])
    s5 = [(3,'ps'),(4,'tank'),(5,'tank'),(6,'tank')]
    run(p, which + ' S5 tank rush', s5, [0])
    # S7: a person who follows the hint list of the Player path, step by step
    s7 = [(8,'ps'),(14,'rifle'),(30,'haul'),(50,'tur'),(60,'rifle'),(61,'rifle'),(75,'tank'),(110,'rifle'),(111,'rifle'),(140,'tank'),(170,'launch'),(171,'launch')]
    run(p, which + ' S7 Player path hint order', s7, [0])
    # S8: greedy: everything queued at once (a hold is expected; it tests the alert rule)
    s8 = [(5,'ps'),(5,'tank'),(6,'tank'),(7,'tank'),(8,'tur'),(9,'tur'),(10,'tank'),(11,'tank')]
    run(p, which + ' S8 greedy, overspends', s8, [0], T=300)
    # L1: the only hauler dies at 90 s with the bank near 0; the player queues a hauler at 95 s
    l1 = [(10,'ps'),(12,'tank'),(40,'tur'),(60,'tank'),(95,'haul')]
    run(p, which + ' L1 hauler lost at 90 s, new hauler queued at 95 s', l1, [0], T=420, kill_haulers_at=90.0)
    # L1b: as L1, but the player follows the alert text: cancel the unit job, then train a hauler
    l1b = [(10,'ps'),(12,'tank'),(40,'tur'),(60,'tank'),(95,'cancel_u'),(95.1,'haul')]
    run(p, which + ' L1b hauler lost at 90 s; cancel the tank and train a hauler at 95 s', l1b, [0], T=420, kill_haulers_at=90.0)
    # L2: the floor. Bank 0, no hauler, empty queues; the player queues one hauler at 0 s.
    q = dict(p); q['start'] = 0
    run(q, which + ' L2 floor: 0 credits, no hauler, a hauler queued at 0 s', [(0, 'haul')], [], T=400)

if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else 'new')
