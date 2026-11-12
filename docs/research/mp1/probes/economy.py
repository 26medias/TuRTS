import math
# Geometry from gather.rs:438 (node reach = unit r + node r + interaction) and gather.rs:652 (receiver reach).
HR, NR, CCR, IR, SPEED, CAP, GPS, UPS = 0.9, 1.0, 2.5, 0.5, 2.0, 200, 10, 40
CC0 = (16, 36)
fields = {"main": [(14, 22), (17.5, 21), (21, 22.5)], "side": [(14, 58), (18, 60)],
          "centre": [(54, 14), (58, 14), (56, 11), (56, 17)]}
def trip(node):
    d = math.dist(CC0, node)
    travel = max(0.0, d - (HR + NR + IR) - (HR + CCR + IR))
    return travel, 2 * travel / SPEED + CAP / GPS + CAP / UPS
for name, nodes in fields.items():
    near = min(nodes, key=lambda n: math.dist(CC0, n))
    tr, t = trip(near)
    print(f"{name:6} nearest node dist={math.dist(CC0, near):5.1f} travel one way={tr:5.1f} trip={t:5.1f}s income={CAP/t*60:5.0f}/min (no overhead)")
# mirror check (point symmetry about (56,36))
mir = lambda p: (112 - p[0], 72 - p[1])
CC1 = mir(CC0)
for name, nodes in fields.items():
    print(name, "slot0 dist", [round(math.dist(CC0, n), 1) for n in nodes], "slot1 dist to same", [round(math.dist(CC1, n), 1) for n in nodes])
print("CC to CC", math.dist(CC0, CC1))
# First three minutes, plan A (economy) and plan B (defence). Step 0.1 s.
# Charge over time: cost spread over build time; low power halves speed and spend rate.
TRIP = trip(min(fields["main"], key=lambda n: math.dist(CC0, n)))[1] * 1.10  # +10% overhead estimate
FIRST = TRIP - (CAP/UPS)  # starts at the CC, empty: travel out, fill, travel back, unload
def run(plan, label):
    credits, t, dt = 1200.0, 0.0, 0.1
    supply, demand = 0, 40
    harvs = [0.0]  # start time of each harvester's cycle
    lanes = {"S": list(p for p in plan if p[0] == "S"), "U": list(p for p in plan if p[0] == "U")}
    cur = {"S": None, "U": None}
    log, income = [], 0.0
    while t < 180:
        # deliveries
        for i, h0 in enumerate(harvs):
            if h0 is None: continue
            k = (t - h0 - FIRST) / TRIP
            kp = (t - dt - h0 - FIRST) / TRIP
            if t - h0 >= FIRST and math.floor(k) != math.floor(kp) or (kp < 0 <= k):
                credits += CAP; income += CAP
        speed = 0.5 if supply < demand else 1.0
        for lane in ("S", "U"):
            if cur[lane] is None and lanes[lane]:
                _, name, cost, secs, sup, dem = lanes[lane].pop(0)
                cur[lane] = [name, cost, secs, 0.0, 0.0, sup, dem]
            j = cur[lane]
            if j is None: continue
            step = speed * dt / j[2]
            pay = j[1] * step
            if credits >= pay:
                credits -= pay; j[3] += step; j[4] += pay
            if j[3] >= 1 - 1e-9:
                supply += j[5]; demand += j[6]
                if j[0] == "harvester": harvs.append(t)
                log.append(f"{t:5.1f}s {j[0]} done (credits {credits:6.0f}, power {supply}/{demand})")
                cur[lane] = None
        t += dt
    print(f"--- {label}: trip {TRIP:.1f}s, first delivery {FIRST:.1f}s")
    for l in log: print("   ", l)
    print(f"    at 180 s: credits {credits:.0f}, harvested {income:.0f}")
# (lane, name, cost, base seconds, supply, demand)
P = ("S", "power station", 300, 12, 100, 0)
run([P, ("U", "harvester", 800, 20, 0, 0), ("U", "rifleman", 100, 6, 0, 0), ("U", "rifleman", 100, 6, 0, 0),
     ("S", "turret", 500, 20, 0, 30), ("U", "launcher", 250, 9, 0, 0)], "Plan A: power, second harvester, then defence")
run([P, ("U", "rifleman", 100, 6, 0, 0), ("U", "rifleman", 100, 6, 0, 0), ("U", "rifleman", 100, 6, 0, 0),
     ("S", "turret", 500, 20, 0, 30), ("U", "launcher", 250, 9, 0, 0), ("U", "tank", 700, 18, 0, 0)], "Plan B: power, turret, army")
run([("U", "rifleman", 100, 6, 0, 0)]*6 + [("U", "launcher", 250, 9, 0, 0)]*2 + [P], "Plan C: no power first (shows the penalty)")
