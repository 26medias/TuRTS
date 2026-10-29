#!/usr/bin/env python3
"""Probe: a completeness gate that can go red.

Rules (each milestone spec):
  R1  The spec has a "## Yardstick coverage" table. Every CORE item of the yardstick has
      exactly one row: | id | disposition | test | where disposition is
      "in" | "later:M<n>" | "mod" | "excluded:S:<line>".
  R2  An "in" row names a test id. That test id must occur in the spec's acceptance section
      ("## ... Acceptance ..." heading).
  R3  The spec has a "## Player path" section with at least one numbered step.
  R4  (roadmap) An item that is "in" at milestone Mx needs every CORE dependency "in" at a
      milestone <= Mx (wrong-milestone check), over all specs given.
Exit 0 = green, 1 = red. Every red line names the rule and the id.
"""
import re
import sys

ID = re.compile(r"^[A-Z]{2}[0-9]{2}$")


def load_yardstick(path):
    items = {}
    in_part_a = False
    for line in open(path, encoding="utf-8"):
        if line.startswith("# Part A"):
            in_part_a = True
        elif line.startswith("# Part B"):
            in_part_a = False
        if not in_part_a or not line.startswith("| "):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) == 7 and ID.match(cells[0]) and cells[5].split()[0] in ("CORE", "COMMON", "OPTIONAL"):
            deps = [d.strip() for d in cells[6].split(",") if ID.match(d.strip())]
            items[cells[0]] = (cells[5].split()[0], deps)
    return items


def sections(text):
    out = {}
    cur = None
    for line in text.split("\n"):
        m = re.match(r"^#{2,3} (.*)", line)
        if m:
            cur = m.group(1)
            out[cur] = []
        elif cur:
            out[cur].append(line)
    return out


def check_spec(path, items, milestone):
    red = []
    rows = {}
    text = open(path, encoding="utf-8").read()
    secs = sections(text)
    cov = next((v for k, v in secs.items() if "yardstick coverage" in k.lower()), None)
    acc = "\n".join(l for k, v in secs.items() if "acceptance" in k.lower() for l in v)
    path_sec = next((v for k, v in secs.items() if k.lower().strip().startswith("player path")), None)
    core = [i for i, (t, _) in items.items() if t == "CORE"]
    if cov is None:
        red.append(f"R1 {path}: no 'Yardstick coverage' section ({len(core)} CORE items unlisted)")
    else:
        for line in cov:
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            if len(cells) >= 3 and ID.match(cells[0]):
                if cells[0] in rows:
                    red.append(f"R1 {path}: duplicate row {cells[0]}")
                rows[cells[0]] = (cells[1], cells[2])
        for i in core:
            if i not in rows:
                red.append(f"R1 {path}: CORE item {i} has no row")
        for i, (disp, test) in rows.items():
            if not re.match(r"^(in|mod|later:M[0-9]+|excluded:S:[0-9]+)$", disp):
                red.append(f"R1 {path}: {i} bad disposition '{disp}'")
            if disp == "in":
                if test in ("", "none", "-"):
                    red.append(f"R2 {path}: {i} is in scope with no test")
                elif not re.search(r"\b" + re.escape(test) + r"\b", acc):
                    red.append(f"R2 {path}: {i} names test {test}, absent from the acceptance section")
    if path_sec is None or not any(re.match(r"^\s*[0-9]+\. ", l) for l in path_sec):
        red.append(f"R3 {path}: no 'Player path' section with numbered steps")
    return red, {i: (milestone, d) for i, (d, _) in rows.items()}


def roadmap(all_rows, items):
    red = []
    first_in = {}
    for i, entries in all_rows.items():
        ms = [m for m, d in entries if d == "in"]
        if ms:
            first_in[i] = min(ms)
    for i, m in first_in.items():
        for dep in items[i][1]:
            if items.get(dep, ("", []))[0] != "CORE":
                continue
            dm = first_in.get(dep)
            if dm is None or dm > m:
                red.append(f"R4 wrong-milestone: {i} in at M{m}, dependency {dep} in at {('M'+str(dm)) if dm else 'never'}")
    return red


def main():
    yard = sys.argv[1]
    items = load_yardstick(yard)
    specs = sys.argv[2:]
    red = []
    all_rows = {}
    for arg in specs:
        m, path = arg.split("=", 1)
        r, rows = check_spec(path, items, int(m))
        red += r
        for i, (mm, d) in rows.items():
            all_rows.setdefault(i, []).append((mm, d))
    red += roadmap(all_rows, items)
    core = sum(1 for t, _ in items.values() if t == "CORE")
    print(f"yardstick items={len(items)} CORE={core}")
    shown = [l for l in red if "has no row" not in l]
    missing = [l for l in red if "has no row" in l]
    for l in shown:
        print("RED", l)
    if missing:
        print(f"RED R1 {len(missing)} 'has no row' lines; first: {missing[0]}")
    print("VERDICT", "red" if red else "green")
    sys.exit(1 if red else 0)


if __name__ == "__main__":
    main()
