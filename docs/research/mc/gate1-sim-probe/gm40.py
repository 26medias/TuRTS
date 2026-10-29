"""Model of MC design 6.3 to 6.5 (spreading, avoidance, arrival), one reading of each rule.
Parameters from the design: r=0.5, speed 3.0 (muster trooper), tick_hz 20, s = 2 r 1.05,
separation threshold r_i + r_j + 0.1, arrival 0.05, stall test 20 ticks / 1 % of one step."""
import math, random, sys

R, SPEED, HZ = 0.5, 3.0, 20
STEP = SPEED / HZ
S = 2 * R * 1.05
SEP_EXTRA = float(sys.argv[1]) if len(sys.argv) > 1 else 0.1
ARRIVE = 0.05
MAPW, MAPH = 64.0, 48.0

def hex_slots(p, count):
    out = []
    k = 0
    while len(out) < count * 4:
        k += 1
        out = []
        for i in range(-k, k + 1):
            for j in range(-k, k + 1):
                x = p[0] + S * (i + 0.5 * j)
                y = p[1] + S * (math.sqrt(3) / 2 * j)
                if R <= x <= MAPW - R and R <= y <= MAPH - R:
                    out.append((x, y))
    out.sort(key=lambda q: (round(math.dist(q, p), 9), math.atan2(q[1] - p[1], q[0] - p[0])))
    return out

def run(starts, p, label, resting=()):
    n = len(starts)
    slots = hex_slots(p, n + len(resting))
    # resting units keep their positions; design drops only static footprints, not resting units
    first = slots[:n]
    order = sorted(range(n), key=lambda i: (math.dist(starts[i], p), i))
    taken = set(); slot_of = {}
    for i in order:
        best = min((k for k in range(n) if k not in taken), key=lambda k: (math.dist(starts[i], first[k]), k))
        taken.add(best); slot_of[i] = first[best]
    pos = [list(s) for s in starts] + [list(q) for q in resting]
    rad = [R] * len(pos)
    state = ['moving'] * n + ['rest'] * len(resting)
    hist = {i: [] for i in range(n)}
    reassigned = set(); blocked_since = {}
    tick = 0
    while tick < 4000 and any(s == 'moving' for s in state):
        tick += 1
        for i in range(n):
            if state[i] != 'moving':
                continue
            tx, ty = slot_of[i]
            dx, dy = tx - pos[i][0], ty - pos[i][1]
            d = math.hypot(dx, dy)
            if d <= ARRIVE:
                state[i] = 'rest'; continue
            m = min(STEP, d)
            vx, vy = dx / d * m, dy / d * m
            for j in range(len(pos)):
                if j == i: continue
                ex, ey = pos[i][0] - pos[j][0], pos[i][1] - pos[j][1]
                e = math.hypot(ex, ey)
                lim = rad[i] + rad[j] + SEP_EXTRA
                if 1e-9 < e < lim:
                    vx += ex / e * (lim - e); vy += ey / e * (lim - e)
            v = math.hypot(vx, vy)
            if v > STEP:
                vx, vy = vx / v * STEP, vy / v * STEP
            pos[i][0] += vx; pos[i][1] += vy
            d2 = math.dist(pos[i], slot_of[i])
            if d2 <= ARRIVE:
                state[i] = 'rest'; continue
            hist[i].append(d2)
            if len(hist[i]) > 20 and hist[i][-21] - d2 < 0.01 * STEP:
                if i not in reassigned:
                    used = set(slot_of.values())
                    free = [q for q in slots if q not in used]
                    slot_of[i] = min(free, key=lambda q: math.dist(pos[i], q))
                    reassigned.add(i); hist[i] = []
                elif len(hist[i]) > 20:
                    state[i] = 'blocked'
    overlaps = 0; worst = 0.0
    for i in range(len(pos)):
        for j in range(i + 1, len(pos)):
            e = math.dist(pos[i], pos[j])
            if e < rad[i] + rad[j] - 1e-6:
                overlaps += 1; worst = max(worst, rad[i] + rad[j] - e)
    nn = len(pos)
    rnear = R * (math.sqrt(nn / 0.7) + 1)
    far = sum(1 for i in range(n) if math.dist(pos[i], p) > rnear)
    maxd = max(math.dist(pos[i], p) for i in range(n))
    print(f"{label}: sep_extra={SEP_EXTRA} ticks={tick} rest={state.count('rest')-len(resting)} "
          f"blocked={state.count('blocked')} moving={state.count('moving')} reassigned={len(reassigned)} "
          f"overlaps={overlaps} worst_overlap={worst:.3f} R_near={rnear:.3f} max_dist={maxd:.3f} beyond_R_near={far}")
    return pos[:n]

P = (40.0, 30.0)
block = [(10 + 1.2 * (k % 8), 10 + 1.2 * (k // 8)) for k in range(40)]
run(block, P, "A block->P")
rng = random.Random(7)
ring = [(P[0] + 15 * math.cos(a), P[1] + 15 * math.sin(a)) for a in [rng.uniform(0, 2 * math.pi) for _ in range(40)]]
ring = [(min(max(x, 1), 63), min(max(y, 1), 47)) for x, y in ring]
run(ring, P, "B ring->P")
first = run(block, P, "C1 first group")
block2 = [(10 + 1.2 * (k % 8), 30 + 1.2 * (k // 8)) for k in range(40)]
run(block2, P, "C2 second group to same P, first at rest", resting=[tuple(q) for q in first])
first_slot = hex_slots(P, 40)[:40]
print("slot radius of slot 40:", round(math.dist(first_slot[-1], P), 3), "R_near(40):", round(R * (math.sqrt(40 / 0.7) + 1), 3))
