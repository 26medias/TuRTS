#!/usr/bin/env python3
"""MP1 plan check (producer, 2026-11-12). Python 3 standard library only.

It reads the task table of the MP1 plan and the MP1 design. It prints the graph
numbers (ASAP schedule, critical path, waves, lanes, headed runs, conflicts) and
it checks coverage:
  - every acceptance row part of design section 12 has exactly one owner task;
  - the stage of each owner agrees with the design stage lists;
  - every scope item of design section 4 has a task;
  - every open point of design section 15 has a closing task;
  - every task input exists, and the graph has no cycle.

Usage:
  python3 -I mp1_plan_check.py <plan.md> <design.md>          exit 0 green, 1 red
  python3 -I mp1_plan_check.py <plan.md> <design.md> --self-test
The self-test applies six mutations to the parsed plan and expects red each time.
"""
import re
import sys
from collections import defaultdict

ROW_ID = re.compile(r"^[A-Z]{2,5}[0-9]$")


def block(text, name):
    m = re.search(r"<!-- %s:start -->(.*?)<!-- %s:end -->" % (name, name), text, re.S)
    if not m:
        raise SystemExit("missing block %s" % name)
    return m.group(1)


def table(md):
    rows = []
    for line in md.splitlines():
        line = line.strip()
        if not line.startswith("|") or set(line) <= set("|- :"):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        rows.append(cells)
    return rows[1:]  # drop header


def items(cell):
    cell = cell.replace("`", "")
    if cell in ("", "-", "none"):
        return []
    return [x.strip() for x in cell.split(",") if x.strip()]


def parse_plan(text):
    tasks = {}
    order = []
    for c in table(block(text, "mp1-tasks")):
        tid, stage, builder, reviewer, tier, inputs, after, runs, rng, headed, rows, scope, hot = c[:13]
        lo, hi = (int(x) for x in rng.split("-"))
        tasks[tid] = dict(stage=stage, builder=builder, reviewer=reviewer, tier=tier,
                          inputs=items(inputs), after=items(after), runs=int(runs), lo=lo, hi=hi,
                          headed=int(headed), rows=items(rows), scope=items(scope), hot=items(hot))
        order.append(tid)
    opens = {}
    for c in table(block(text, "mp1-open")):
        opens[int(c[0])] = items(c[1])
    splits = {}
    for c in table(block(text, "mp1-splits")):
        splits[c[0]] = items(c[1])
    return tasks, order, opens, splits


def parse_design(text):
    s12 = text.split("## 12. Acceptance rows", 1)[1].split("## 13.", 1)[0]
    rows = []
    for line in s12.splitlines():
        if line.startswith("| ") and not line.startswith("| Row") and not line.startswith("| ---"):
            rid = line[2:].split("|", 1)[0].strip()
            if ROW_ID.match(rid):
                rows.append(rid)
    lists = s12.split("Stage 1 rows:", 1)[1].split("Section 10 (the stage", 1)[0]
    s1_text, s2_text = lists.split("Stage 2 rows:", 1)
    tok = lambda t: {x for x in re.findall(r"\b[A-Z]{2,5}[0-9]\b", t) if x in rows}
    s1, s2 = tok(s1_text), tok(s2_text)
    s4 = text.split("## 4. Scope", 1)[1].split("Not in MP1", 1)[0]
    scope = re.findall(r"^\| (X[0-9]+[a-z]?) \|", s4, re.M)
    s15 = text.split("## 15. Open points", 1)[1].split("The disposition", 1)[0]
    points = [int(x) for x in re.findall(r"^([0-9]+)\. ", s15, re.M)]
    return rows, s1, s2, scope, points


def schedule(tasks):
    start, end, pred = {}, {}, {}
    state = {}

    def visit(t, stack):
        if state.get(t) == 2:
            return
        if state.get(t) == 1:
            raise ValueError("cycle: " + " -> ".join(stack + [t]))
        state[t] = 1
        s, p = 0, None
        for i in tasks[t]["inputs"]:
            visit(i, stack + [t])
            if end[i] > s:
                s, p = end[i], i
        e, pe = s + tasks[t]["runs"], p
        for a in tasks[t]["after"]:
            visit(a, stack + [t])
            if end[a] > e:
                e, pe = end[a], a
        start[t], end[t], pred[t] = s, e, pe
        state[t] = 2

    for t in tasks:
        visit(t, [])
    return start, end, pred


def reach(tasks):
    memo = {}

    def up(t):
        if t not in memo:
            r = set()
            for i in tasks[t]["inputs"] + tasks[t]["after"]:
                r |= {i} | up(i)
            memo[t] = r
        return memo[t]

    for t in tasks:
        up(t)
    return memo


def levels(tasks):
    lv = {}

    def L(t):
        if t not in lv:
            ins = tasks[t]["inputs"] + tasks[t]["after"]
            lv[t] = 0 if not ins else 1 + max(L(i) for i in ins)
        return lv[t]

    for t in tasks:
        L(t)
    return lv


def check(tasks, order, opens, splits, design, out):
    rows, s1, s2, scope, points = design
    red = []
    for t, d in tasks.items():
        for i in d["inputs"] + d["after"]:
            if i not in tasks:
                red.append("task %s: unknown input %s" % (t, i))
        if not d["builder"] or "," in d["builder"]:
            red.append("task %s: needs exactly one builder" % t)
    if red:
        return red
    try:
        start, end, pred = schedule(tasks)
    except ValueError as e:
        return [str(e)]
    # rows
    expected = {}
    for r in rows:
        if r in splits:
            for p in splits[r]:
                expected[p] = "1" if p.endswith("/s1") else "2"
        elif r in s1 and r in s2:
            expected[r + "/s1"], expected[r + "/s2"] = "1", "2"
        elif r in s1:
            expected[r] = "1"
        elif r in s2:
            expected[r] = "2"
        else:
            red.append("row %s is in no stage list of the design" % r)
    owners = defaultdict(list)
    for t, d in tasks.items():
        for p in d["rows"]:
            owners[p].append(t)
    for p, st in sorted(expected.items()):
        o = owners.get(p, [])
        if len(o) != 1:
            red.append("row part %s has %d owners %s" % (p, len(o), o))
        elif tasks[o[0]]["stage"] not in ({"0", "1"} if st == "1" else {"2"}):
            red.append("row part %s is stage %s but its owner %s is stage %s" % (p, st, o[0], tasks[o[0]]["stage"]))
    for p in owners:
        if p not in expected:
            red.append("unknown row part %s (owners %s)" % (p, owners[p]))
    covered = {s for d in tasks.values() for s in d["scope"]}
    for x in scope:
        if x not in covered:
            red.append("scope item %s has no task" % x)
    for n in points:
        if n not in opens:
            red.append("open point %d has no closing task" % n)
        else:
            for t in opens[n]:
                if t not in tasks and t not in ("gate2", "MP2", "M7", "M4", "Root", "closed", "CEO"):
                    red.append("open point %d names unknown task %s" % (n, t))
    if out:
        report(tasks, order, start, end, pred, rows, expected, scope, points)
    return red


def report(tasks, order, start, end, pred, rows, expected, scope, points):
    mp1 = [t for t in order if tasks[t]["stage"] != "slice"]
    last = max(mp1, key=lambda t: end[t])
    path, t = [], last
    while t:
        path.append(t)
        t = pred[t]
    path.reverse()
    print("critical path (%d runs from now, D0 wait not counted): %s" % (end[last], " -> ".join(path)))
    for name in ("C1-PERSON", "REG2-2", "PT3-PEOPLE"):
        if name in tasks:
            print("  %s ends at offset %d" % (name, end[name]))
    lv = levels(tasks)
    print("\nwaves (topological level; MP1 tasks; [slice]); lanes = max concurrent builds in the wave span")
    for k in range(max(lv.values()) + 1):
        ts = [t for t in order if lv[t] == k]
        if not ts:
            continue
        a = min(start[t] for t in ts)
        b = max(start[t] + tasks[t]["runs"] for t in ts)
        best = (0, 0, 0)
        for x in range(a, max(b, a + 1)):
            run = [u for u in order if start[u] <= x < start[u] + tasks[u]["runs"]]
            m = sum(1 for u in run if tasks[u]["stage"] != "slice")
            s = len(run) - m
            g = sum(1 for u in run if tasks[u]["headed"] > 0)
            best = max(best, (m + s, m, s, g)[:3]) if (m + s) > best[0] else best
        names = ", ".join(t if tasks[t]["stage"] != "slice" else "[%s]" % t for t in ts)
        print("W%-2d offset %3d-%-3d max lanes %d (MP1 %d, slice %d): %s" % (k, a, b, best[0], best[1], best[2], names))
    print("\nper-offset load (MP1 builds, slice builds, headed tasks)")
    horizon = max(end.values())
    line = []
    for x in range(horizon):
        run = [u for u in order if start[u] <= x < start[u] + tasks[u]["runs"]]
        m = sum(1 for u in run if tasks[u]["stage"] != "slice")
        g = sum(1 for u in run if tasks[u]["headed"] > 0)
        line.append("%d:%d/%d/%d" % (x, m, len(run) - m, g))
    print(" ".join(line))
    # latest finish without moving the last MP1 task (merge-after inputs bound the end)
    horizon = end[last]
    late_end = {}
    for t in sorted(order, key=lambda u: -lv[u]):
        bound = [horizon]
        for s in order:
            if t in tasks[s]["inputs"]:
                bound.append(late_end[s] - tasks[s]["runs"])
            if t in tasks[s]["after"]:
                bound.append(late_end[s])
        late_end[t] = min(bound)
    print("\nschedule (task earliest start-end; slack in runs before the path moves)")
    print(" ".join("%s %d-%d/%d" % (t, start[t], end[t], late_end[t] - end[t]) for t in order))
    print("\nbudget by stage (point runs, range, headed runs, task count)")
    for st in ("0", "1", "2", "slice"):
        ts = [t for t in order if tasks[t]["stage"] == st]
        print("  stage %-5s runs %3d range %3d-%-3d headed %2d tasks %2d" % (
            st, sum(tasks[t]["runs"] for t in ts), sum(tasks[t]["lo"] for t in ts),
            sum(tasks[t]["hi"] for t in ts), sum(tasks[t]["headed"] for t in ts), len(ts)))
    print("\nconcurrent pairs that share a hot file (no path between them)")
    up = reach(tasks)
    files = defaultdict(list)
    for t in order:
        for f in tasks[t]["hot"]:
            files[f].append(t)

    def same(f, g):
        return f == g or (f.endswith("/") and g.startswith(f)) or (g.endswith("/") and f.startswith(g))

    keys = sorted(files)
    pairs = set()
    for i, f in enumerate(keys):
        for g in keys[i:]:
            if not same(f, g):
                continue
            for a in files[f]:
                for b in files[g]:
                    if a == b or a in up[b] or b in up[a]:
                        continue
                    if tasks[a]["stage"] == "slice" and tasks[b]["stage"] == "slice":
                        continue
                    pairs.add((min(f, g), tuple(sorted((a, b)))))
    for f, (a, b) in sorted(pairs):
        print("  %-28s %s | %s" % (f, a, b))
    print("\nrow parts: %d; scope items: %d; open points: %d" % (len(expected), len(scope), len(points)))


def main():
    plan_text = open(sys.argv[1], encoding="utf-8").read()
    design = parse_design(open(sys.argv[2], encoding="utf-8").read())
    tasks, order, opens, splits = parse_plan(plan_text)
    if "--self-test" in sys.argv:
        import copy
        muts = []
        owner = next(t for t in order if tasks[t]["rows"])
        m = copy.deepcopy(tasks); m[owner]["rows"] = m[owner]["rows"][1:]; muts.append(("row unowned", m, opens))
        m = copy.deepcopy(tasks); other = next(t for t in order if t != owner); m[other]["rows"].append(tasks[owner]["rows"][0]); muts.append(("row owned twice", m, opens))
        m = copy.deepcopy(tasks); first = order[0]; m[first]["inputs"].append(order[-1]); muts.append(("cycle", m, opens))
        m = copy.deepcopy(tasks)
        for t in m:
            m[t]["scope"] = [s for s in m[t]["scope"] if s != "X0e"]
        muts.append(("scope X0e dropped", m, opens))
        o = dict(opens); o.pop(21, None); muts.append(("open point 21 dropped", copy.deepcopy(tasks), o))
        m = copy.deepcopy(tasks)
        s2task = next(t for t in order if m[t]["stage"] == "2" and m[t]["rows"])
        m[s2task]["stage"] = "1"; muts.append(("stage-2 owner marked stage 1", m, opens))
        bad = 0
        for name, mt, mo in muts:
            r = check(mt, order, mo, splits, design, False)
            print("%-32s %s" % (name, "RED (%s)" % r[0] if r else "GREEN: the check cannot see it"))
            bad += 0 if r else 1
        base = check(tasks, order, opens, splits, design, False)
        print("unmutated plan:", "GREEN" if not base else "RED %s" % base)
        sys.exit(1 if bad or base else 0)
    red = check(tasks, order, opens, splits, design, True)
    print("\nRESULT", "GREEN" if not red else "RED")
    for r in red:
        print("  " + r)
    sys.exit(1 if red else 0)


if __name__ == "__main__":
    main()
