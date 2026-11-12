#!/usr/bin/env python3
"""MP1 plan check (producer; revision 3 after the re-check of plan revision 2, 2026-11-12). Python 3 standard library only.

It reads the task table of the MP1 plan and the MP1 design. It prints the graph
numbers (schedule, critical path, waves, lanes, headed runs, tiers, conflicts) and
it checks:
  - every acceptance row part of design section 12 has exactly one owner task;
  - the stage of each owner agrees with the design stage lists, the plan splits
    (block mp1-splits) and the plan moves (block mp1-moves, each with a D erratum);
  - every scope item of design section 4 has a task;
  - every open point of design section 15 has a closing task;
  - every task input exists, and the graph has no cycle;
  - each task has one builder, and the reviewer is not the builder;
  - every stage-2 task descends from C1-PERSON through Inputs (not Merge after);
  - each (task, slice task) pair of block mp1-tree is a path through Inputs;
  - each row part has at least one plant entry (block mp1-plants); the owner
    descends from each plant task (Inputs or Merge after); a pixel plant ("px")
    needs a Headed count above 0 in its task;
  - (revision 3, qa N2, rigour L4) a row part whose design "Runs" cell names
    "pixels" has at least one plant entry marked lane+px; each plant cell holds a
    design plant label of the row: a label "(x)" of the design "Red build" cell,
    or, for a row whose Red build cell has no label, three words in sequence of
    that cell; the task text of each plant task names the row with the word
    "plant" near it, and the first label of the cell after the row name;
  - (revision 3) the pairs of REQUIRED_TREE are paths through Inputs and are
    listed in block mp1-tree; each cross stack (an MP1 task with a stacked slice
    input) has its row in block mp1-tree;
  - (revision 3, ruling R-T) every MP1 task with a Headed count above 0 descends
    from Q-LOCK through Inputs.

Schedule model (gate 2, rigour M1 and TD H2):
  - a plain input i: the task starts at end[i] (i merged);
  - a stacked input "i^": the task starts when the build of i ends
    (start[i] + runs[i]) and it merges at least 1 run after i merges;
  - a merge-after task a: the task ends at max(start + runs, end[a] + 1).
  Two tasks are concurrent when no path of Inputs joins them; a merge-after edge
  orders merges, not builds.

Usage:
  python3 -I mp1_plan_check.py <plan.md> <design.md>          exit 0 green, 1 red
  python3 -I mp1_plan_check.py <plan.md> <design.md> --self-test
The self-test applies twenty mutations to the parsed plan and expects red each time.
"""
import copy
import re
import sys
from collections import defaultdict

ROW_ID = re.compile(r"^[A-Z]{2,5}[0-9]$")
OUTSIDE = ("gate2", "MP2", "M7", "M4", "M8", "Root", "closed", "CEO")
LABEL = re.compile(r"\(([a-z]|[0-9]{1,2})\)")
RANGE = re.compile(r"\(([a-z]|[0-9]{1,2})\) to \(([a-z]|[0-9]{1,2})\)")
# Build-tree pairs that the script holds itself, so that a deleted mp1-tree row
# cannot remove the rule (rigour L4 n1, n3). Each pair: (task, task that must be
# on a path of Inputs to it), with its source.
REQUIRED_TREE = (
    ("L0-API1", "S2"), ("L0-API1", "D0p"),      # E1, D:2304
    ("L0-SIM", "S2"), ("L0-SIM", "D0p"),        # E1, D:2304
    ("L0-WIRE", "S2"), ("L0-WIRE", "D0p"),      # E1, D:2304
    ("L0b-ORD", "A1c"),                         # E2, D:2305
    ("L-PAY", "A1d"), ("L-INC", "A1d"),         # the ledger oracle, D:2421-2422
    ("L-TGT", "A3"),                            # D:2224-2225, TD H2
    ("L-OBS", "A3"),                            # D:2523, rigour H2
    ("C1-FINAL", "A3"),                         # D:2365, X0e
    ("C1-PLAYER1", "I-PLAY1"),                  # it judges the I-PLAY1 evidence set (R-D)
    ("C1-PLAYER", "REG2-1"),                    # it judges the REG2-1 evidence set (R-D)
    ("PT3-PLAYER", "REG2-2"),                   # it judges the REG2-2 evidence set (R-D)
)


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
        rows.append([c.strip() for c in line.strip("|").split("|")])
    return rows[1:]  # drop header


def items(cell):
    cell = cell.replace("`", "")
    if cell in ("", "-", "none"):
        return []
    return [x.strip() for x in cell.split(",") if x.strip()]


def parse_plan(text):
    tasks, order = {}, []
    for c in table(block(text, "mp1-tasks")):
        tid, stage, builder, reviewer, tier, inputs, after, runs, rng, headed, rows, scope, hot = c[:13]
        lo, hi = (int(x) for x in rng.split("-"))
        ins = items(inputs)
        tasks[tid] = dict(stage=stage, builder=builder, reviewer=reviewer, tier=tier,
                          inputs=[i.rstrip("^") for i in ins if not i.endswith("^")],
                          stacked=[i.rstrip("^") for i in ins if i.endswith("^")],
                          after=items(after), runs=int(runs), lo=lo, hi=hi,
                          headed=int(headed), rows=items(rows), scope=items(scope), hot=items(hot))
        order.append(tid)
    opens = {}
    for c in table(block(text, "mp1-open")):
        opens[int(c[0])] = items(c[1])
    splits = {c[0]: items(c[1]) for c in table(block(text, "mp1-splits"))}
    moves = {c[0]: (c[1], c[2]) for c in table(block(text, "mp1-moves"))}
    tree = [(c[0], items(c[1])) for c in table(block(text, "mp1-tree"))]
    plants = [dict(part=c[0], task=c[1], plants=c[2], red=c[3]) for c in table(block(text, "mp1-plants"))]
    return dict(tasks=tasks, order=order, opens=opens, splits=splits, moves=moves, tree=tree, plants=plants,
                texts=task_texts(text, tasks))


def task_texts(text, tasks):
    """The task text: the "### Task <id>" section (a heading can name several
    tasks), or the stage-2 table row that starts with "| `<id>` |"."""
    texts = defaultdict(str)
    parts = re.split(r"^(#{2,3} .*)$", text, flags=re.M)
    for i in range(1, len(parts) - 1, 2):
        head, body = parts[i], parts[i + 1]
        if not head.startswith("### Task "):
            continue
        for tid in re.findall(r"Task ([A-Za-z0-9-]+)", head):
            if tid in tasks:
                texts[tid] += head + body
    for line in text.splitlines():
        m = re.match(r"^\| `([A-Za-z0-9-]+)` \|", line)
        if m and m.group(1) in tasks:
            texts[m.group(1)] += line + "\n"
    return dict(texts)


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
    info = {}
    for line in s12.splitlines():
        m = re.match(r"^\| ([A-Z]{2,5}[0-9]) \|", line)
        if m:
            c = [x.strip() for x in line.strip().strip("|").split("|")]
            info[m.group(1)] = dict(red=c[2], runs=c[-1], labels=set(LABEL.findall(c[2])))
    return rows, s1, s2, scope, points, info


def deps(d):
    return d["inputs"] + d["stacked"] + d["after"]


def schedule(tasks):
    start, end, pred = {}, {}, {}
    state = {}

    def visit(t, stack):
        if state.get(t) == 2:
            return
        if state.get(t) == 1:
            raise ValueError("cycle: " + " -> ".join(stack + [t]))
        state[t] = 1
        d = tasks[t]
        s, p = 0, None
        for i in d["inputs"]:
            visit(i, stack + [t])
            if end[i] > s:
                s, p = end[i], i
        for i in d["stacked"]:
            visit(i, stack + [t])
            b = start[i] + tasks[i]["runs"]
            if b > s:
                s, p = b, i
        e, pe = s + d["runs"], p
        for a in d["after"] + d["stacked"]:
            visit(a, stack + [t])
            if end[a] + 1 > e:
                e, pe = end[a] + 1, a
        start[t], end[t], pred[t] = s, e, pe
        state[t] = 2

    for t in tasks:
        visit(t, [])
    return start, end, pred


def ancestors(tasks, through_after):
    memo = {}

    def up(t):
        if t not in memo:
            memo[t] = set()
            ins = tasks[t]["inputs"] + tasks[t]["stacked"] + (tasks[t]["after"] if through_after else [])
            r = set()
            for i in ins:
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
            ins = deps(tasks[t])
            lv[t] = 0 if not ins else 1 + max(L(i) for i in ins)
        return lv[t]

    for t in tasks:
        L(t)
    return lv


def expected_parts(plan, design, red):
    rows, s1, s2, scope, points, info = design
    expected = {}
    for r in rows:
        if r in plan["splits"]:
            for p in plan["splits"][r]:
                expected[p] = "1" if p.endswith("/s1") else "2"
        elif r in s1 and r in s2:
            expected[r + "/s1"], expected[r + "/s2"] = "1", "2"
        elif r in s1:
            expected[r] = "1"
        elif r in s2:
            expected[r] = "2"
        else:
            red.append("row %s is in no stage list of the design" % r)
    for p, (dstage, pstage) in plan["moves"].items():
        if p not in expected:
            red.append("move of unknown row part %s" % p)
        elif expected[p] not in (dstage, pstage):
            red.append("move of %s: design stage %s, but the move says %s" % (p, expected[p], dstage))
        else:
            expected[p] = pstage
    return expected


def check(plan, design, out):
    tasks, order = plan["tasks"], plan["order"]
    rows, s1, s2, scope, points, info = design
    red = []
    for t, d in tasks.items():
        for i in deps(d):
            if i not in tasks:
                red.append("task %s: unknown input %s" % (t, i))
        if not d["builder"] or "," in d["builder"]:
            red.append("task %s: needs exactly one builder" % t)
        if d["reviewer"].split(" ")[0] == d["builder"]:
            red.append("task %s: the reviewer is the builder (%s)" % (t, d["builder"]))
    if red:
        return red
    try:
        start, end, pred = schedule(tasks)
    except ValueError as e:
        return [str(e)]
    up_in = ancestors(tasks, False)
    up_all = ancestors(tasks, True)
    expected = expected_parts(plan, design, red)
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
        if n not in plan["opens"]:
            red.append("open point %d has no closing task" % n)
        else:
            for t in plan["opens"][n]:
                if t not in tasks and t not in OUTSIDE:
                    red.append("open point %d names unknown task %s" % (n, t))
    # stage 2 starts after C1 (D:60, D:2366), through Inputs only
    if "C1-PERSON" in tasks:
        for t, d in tasks.items():
            if d["stage"] == "2" and "C1-PERSON" not in up_in[t]:
                red.append("stage-2 task %s does not descend from C1-PERSON through Inputs" % t)
    # build-tree needs (slice code that a task tests)
    for t, needs in plan["tree"]:
        if t not in tasks:
            red.append("tree rule names unknown task %s" % t)
            continue
        for n in needs:
            if n not in up_in[t]:
                red.append("task %s must have %s in its build tree through Inputs" % (t, n))
    # plants
    by_part = defaultdict(list)
    for e in plan["plants"]:
        if e["task"] not in tasks:
            red.append("plant entry names unknown task %s" % e["task"])
            continue
        if e["part"] not in expected:
            red.append("plant entry for unknown row part %s" % e["part"])
            continue
        if not e["plants"] or e["plants"] == "-":
            red.append("plant entry %s / %s has no plant" % (e["part"], e["task"]))
        if "px" in e["red"] and tasks[e["task"]]["headed"] < 1:
            red.append("task %s has a pixel plant for %s and no headed run" % (e["task"], e["part"]))
        by_part[e["part"]].append(e["task"])
    # revision 3: pixel status from the design Runs cell; plant labels; task text
    texts = plan.get("texts", {})
    for e in plan["plants"]:
        if e["task"] not in tasks or e["part"] not in expected:
            continue
        row = e["part"].split("/")[0]
        d = info.get(row)
        if d is None:
            red.append("plant entry %s: row %s has no design row" % (e["part"], row))
            continue
        if not plant_label_ok(e["plants"], d):
            red.append("plant entry %s / %s holds no design plant label of row %s" % (e["part"], e["task"], row))
        if not step_names_plant(texts.get(e["task"], ""), row, e["plants"], d):
            red.append("task %s: its step does not name the plants of %s" % (e["task"], e["part"]))
    px_parts = defaultdict(bool)
    for e in plan["plants"]:
        if e["red"].strip() == "lane+px":
            px_parts[e["part"]] = True
    for p in sorted(expected):
        d = info.get(p.split("/")[0])
        if d and "pixels" in d["runs"] and not px_parts[p]:
            red.append("row part %s runs on pixels (design Runs: %s) but no plant entry is lane+px" % (p, d["runs"]))
    for p in sorted(expected):
        if not by_part.get(p):
            red.append("row part %s has no plant entry" % p)
            continue
        o = owners.get(p, [])
        if len(o) == 1:
            for t in by_part[p]:
                if t != o[0] and t not in up_all[o[0]]:
                    red.append("row part %s: owner %s does not descend from plant task %s" % (p, o[0], t))
    # revision 3: required build-tree pairs, cross stacks, the GPU lock task
    listed = {(t, n) for t, needs in plan["tree"] for n in needs}
    for t, n in REQUIRED_TREE:
        if t not in tasks or n not in tasks:
            continue
        if n not in up_in[t]:
            red.append("task %s must have %s in its build tree through Inputs (REQUIRED_TREE)" % (t, n))
        if (t, n) not in listed:
            red.append("block mp1-tree has no row for the required pair %s / %s" % (t, n))
    for t, d in tasks.items():
        if d["stage"] == "slice":
            continue
        for x in d["stacked"]:
            if tasks[x]["stage"] == "slice" and (t, x) not in listed:
                red.append("cross stack %s^ in %s has no row in block mp1-tree" % (x, t))
    if "Q-LOCK" in tasks:
        for t, d in tasks.items():
            if d["stage"] != "slice" and d["headed"] > 0 and "Q-LOCK" not in up_in[t]:
                red.append("headed task %s does not descend from Q-LOCK through Inputs" % t)
    if out:
        report(plan, start, end, pred, expected, scope, points, up_in)
    return red


def cell_labels(cell):
    labs = set(LABEL.findall(cell))
    for a, b in RANGE.findall(cell):
        if a.isdigit() and b.isdigit():
            labs |= {str(k) for k in range(int(a), int(b) + 1)}
        elif a.isalpha() and b.isalpha():
            labs |= {chr(k) for k in range(ord(a), ord(b) + 1)}
    return labs


def words(t):
    return re.findall(r"[a-z0-9_]+", t.lower().replace("`", ""))


def plant_label_ok(cell, d):
    if d["labels"]:
        return bool(cell_labels(cell) & d["labels"])
    w, dw = words(cell), " ".join(words(d["red"]))
    return any(" ".join(w[i:i + 3]) in dw for i in range(len(w) - 2))


def step_names_plant(text, row, cell, d):
    key = None
    if d["labels"]:
        m = LABEL.search(cell)
        key = m.group(0) if m else None
    low = text.lower()
    for m in re.finditer(r"\b%s\b" % re.escape(row), text):
        a, b = max(0, m.start() - 300), m.end() + 300
        if "plant" not in low[a:b]:
            continue
        if key is None or key in text[m.end():b]:
            return True
    return False


def report(plan, start, end, pred, expected, scope, points, up_in):
    tasks, order = plan["tasks"], plan["order"]
    mp1 = [t for t in order if tasks[t]["stage"] != "slice"]
    last = max(mp1, key=lambda t: end[t])
    path, t = [], last
    while t:
        path.append(t)
        t = pred[t]
    path.reverse()
    print("critical path (%d runs from now, D0 wait not counted): %s" % (end[last], " -> ".join(path)))
    for name in ("L0-EXIT", "C1-PLAYER1", "REG2-1", "C1-PERSON", "L0-EXIT2", "I2-PIX", "REG2-2", "PT3-PEOPLE", "REC"):
        if name in tasks:
            print("  %s ends at offset %d" % (name, end[name]))
    lv = levels(tasks)

    def running(x):
        return [u for u in order if start[u] <= x < start[u] + tasks[u]["runs"]]

    print("\nwaves (topological level; MP1 tasks; [slice]); lanes = max concurrent builds in the wave span")
    for k in range(max(lv.values()) + 1):
        ts = [t for t in order if lv[t] == k]
        if not ts:
            continue
        a = min(start[t] for t in ts)
        b = max(start[t] + tasks[t]["runs"] for t in ts)
        best = (0, 0, 0)
        for x in range(a, max(b, a + 1)):
            run = running(x)
            m = sum(1 for u in run if tasks[u]["stage"] != "slice")
            if len(run) > best[0]:
                best = (len(run), m, len(run) - m)
        names = ", ".join(t if tasks[t]["stage"] != "slice" else "[%s]" % t for t in ts)
        print("W%-2d offset %3d-%-3d max lanes %d (MP1 %d, slice %d): %s" % (k, a, b, best[0], best[1], best[2], names))
    print("\nper-offset load (MP1 builds/slice builds/headed tasks)")
    horizon = max(end.values())
    line = []
    for x in range(horizon):
        run = running(x)
        m = sum(1 for u in run if tasks[u]["stage"] != "slice")
        g = sum(1 for u in run if tasks[u]["headed"] > 0)
        line.append("%d:%d/%d/%d" % (x, m, len(run) - m, g))
    print(" ".join(line))
    horizon = end[last]
    late_end = {}
    for t in sorted(order, key=lambda u: -lv[u]):
        bound = [horizon]
        for s in order:
            d = tasks[s]
            if t in d["inputs"]:
                bound.append(late_end[s] - d["runs"])
            if t in d["stacked"]:
                bound.append(late_end[s] - 1)
                bound.append(late_end[s] - d["runs"] + (end[t] - start[t] - tasks[t]["runs"]))
            if t in d["after"]:
                bound.append(late_end[s] - 1)
        late_end[t] = min(bound)
    print("\nschedule (task earliest start-end; slack in runs before the path moves)")
    print(" ".join("%s %d-%d/%d" % (t, start[t], end[t], late_end[t] - end[t]) for t in order))
    print("\nbudget by stage (point runs, range, headed runs, task count)")
    for st in ("0", "1", "2", "slice"):
        ts = [t for t in order if tasks[t]["stage"] == st]
        print("  stage %-5s runs %3d range %3d-%-3d headed %2d tasks %2d" % (
            st, sum(tasks[t]["runs"] for t in ts), sum(tasks[t]["lo"] for t in ts),
            sum(tasks[t]["hi"] for t in ts), sum(tasks[t]["headed"] for t in ts), len(ts)))
    tiers = defaultdict(list)
    for t in mp1:
        tiers[tasks[t]["tier"]].append(t)
    print("\nmodel tiers (builder/reviewer): " + "; ".join("%s %d" % (k, len(v)) for k, v in sorted(tiers.items())))
    print("\nconcurrent pairs that share a hot file (no path of Inputs between them)")
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
                    if a == b or a in up_in[b] or b in up_in[a]:
                        continue
                    if tasks[a]["stage"] == "slice" and tasks[b]["stage"] == "slice":
                        continue
                    pairs.add((min(f, g), tuple(sorted((a, b)))))
    hidden = 0
    for f, (a, b) in sorted(pairs):
        tag = ""
        if a in tasks[b]["after"] or b in tasks[a]["after"]:
            tag, hidden = "  (merge-after only)", hidden + 1
        print("  %-28s %s | %s%s" % (f, a, b, tag))
    print("  %d pairs; %d of them are ordered by a merge-after edge only" % (len(pairs), hidden))
    print("\nrow parts: %d; scope items: %d; open points: %d; plant entries: %d" % (
        len(expected), len(scope), len(points), len(plan["plants"])))


def mutations(plan):
    tasks, order = plan["tasks"], plan["order"]
    out = []

    def mut(name, fn):
        p = copy.deepcopy(plan)
        try:
            fn(p)
        except (KeyError, StopIteration, ValueError) as e:
            out.append((name, None, "mutation could not apply: %r" % (e,)))
            return
        out.append((name, p, None))

    owner = next(t for t in order if tasks[t]["rows"])

    def m1(p):
        p["tasks"][owner]["rows"] = p["tasks"][owner]["rows"][1:]
    mut("row unowned", m1)

    def m2(p):
        other = next(t for t in order if t != owner)
        p["tasks"][other]["rows"].append(tasks[owner]["rows"][0])
    mut("row owned twice", m2)

    def m3(p):
        p["tasks"][order[0]]["inputs"].append(order[-1])
    mut("cycle", m3)

    def m4(p):
        for t in p["tasks"]:
            p["tasks"][t]["scope"] = [s for s in p["tasks"][t]["scope"] if s != "X0e"]
    mut("scope X0e dropped", m4)

    def m5(p):
        p["opens"].pop(21)
    mut("open point 21 dropped", m5)

    def m6(p):
        s2task = next(t for t in order if p["tasks"][t]["stage"] == "2" and p["tasks"][t]["rows"])
        p["tasks"][s2task]["stage"] = "1"
    mut("stage-2 owner marked stage 1", m6)

    def m7(p):
        p["tasks"]["I-PIX1"]["inputs"].remove("C1-FINAL")
    mut("qa: I-PIX1 without C1-FINAL", m7)

    def m8(p):
        n = len(p["plants"])
        p["plants"] = [e for e in p["plants"] if e["task"] != "L-STO"]
        if len(p["plants"]) == n:
            raise ValueError("no L-STO plant entry")
    mut("qa: L-STO plant entries deleted", m8)

    def m9(p):
        p["tasks"]["I-HEAD"]["inputs"] = ["L0-EXIT"]
        p["tasks"]["I-HEAD"]["stacked"] = []
    mut("qa: I-HEAD with only L0-EXIT", m9)

    def m10(p):
        p["tasks"]["L-DMG"]["reviewer"] = p["tasks"]["L-DMG"]["builder"]
    mut("rigour: reviewer equals builder", m10)

    def m11(p):
        d = p["tasks"]["L-CON3"]
        d["inputs"] = ["L0-EXIT" if i in ("L0-EXIT2", "C1-PERSON") else i for i in d["inputs"]]
        if "C1-PERSON" in ancestors(p["tasks"], False)["L-CON3"]:
            raise ValueError("L-CON3 still descends from C1-PERSON")
    mut("rigour: stage 2 before C1", m11)

    def m12(p):
        d = p["tasks"]["L-OBS"]
        if "A3" in d["inputs"]:
            d["inputs"].remove("A3")
        else:
            d["stacked"].remove("A3")
        d["after"].append("A3")
    mut("rigour: A3 merge-after only in L-OBS", m12)

    def m13(p):
        p["tasks"]["UI-PLACE"]["headed"] = 0
    mut("qa: pixel plant with no headed run", m13)

    # revision 3: the six green mutations of the re-check (qa N2 m4 to m6, rigour L4 n1 to n3)
    def m14(p):
        t = p["texts"]["L-STO"]
        cut = "plants STO1 (a) to (e)"
        if cut not in t:
            raise ValueError("no plant text in L-STO")
        p["texts"]["L-STO"] = t.replace(cut, "")
    mut("qa m4: L-STO step without its plant text", m14)

    def plant_entry(p, part, task):
        return next(e for e in p["plants"] if e["part"] == part and e["task"] == task)

    def m15(p):
        plant_entry(p, "GHO1", "UI-PLACE")["red"] = "lane"
    mut("qa m5: GHO1 plant lane, not lane+px", m15)

    def m16(p):
        plant_entry(p, "HUD5", "UI-HUD")["plants"] = "none"
    mut("qa m6: HUD5 plant cell none", m16)

    def m17(p):
        p["tree"] = [(t, n) for t, n in p["tree"] if t != "L-TGT"]
        d = p["tasks"]["L-TGT"]
        d["stacked"].remove("A3")
        d["after"].append("A3")
    mut("rigour n1: L-TGT tree row gone, A3 merge-after", m17)

    def m18(p):
        plant_entry(p, "FX1/s1", "L-FX-PRES")["red"] = "lane"
    mut("rigour n2: FX1/s1 pixel plant lane (mixed row)", m18)

    def m19(p):
        p["tasks"]["C1-PLAYER1"]["inputs"].remove("I-PLAY1")
    mut("rigour n3: C1-PLAYER1 without I-PLAY1", m19)

    def m20(p):
        p["tasks"]["L0-DBG"]["inputs"].remove("Q-LOCK")
    mut("R-T: headed L0-DBG without Q-LOCK", m20)
    return out


def main():
    plan = parse_plan(open(sys.argv[1], encoding="utf-8").read())
    design = parse_design(open(sys.argv[2], encoding="utf-8").read())
    if "--self-test" in sys.argv:
        bad = 0
        for name, p, err in mutations(plan):
            if err:
                print("%-40s ERROR: %s" % (name, err))
                bad += 1
                continue
            r = check(p, design, False)
            print("%-40s %s" % (name, "RED (%s)" % r[0] if r else "GREEN: the check cannot see it"))
            bad += 0 if r else 1
        base = check(plan, design, False)
        print("unmutated plan:", "GREEN" if not base else "RED %s" % base)
        sys.exit(1 if bad or base else 0)
    red = check(plan, design, True)
    print("\nRESULT", "GREEN" if not red else "RED")
    for r in red:
        print("  " + r)
    sys.exit(1 if red else 0)


if __name__ == "__main__":
    main()
