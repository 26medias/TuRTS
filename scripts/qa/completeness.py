#!/usr/bin/env python3
"""Check milestone specs against the committed yardstick and spec registry."""
import argparse
import csv
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
YARDSTICK = ROOT / "docs/qa/yardstick-a.tsv"
MILESTONES = ROOT / "docs/qa/milestones.tsv"
CASES = ROOT / "tests/qa/completeness/cases.tsv"
SPEC_ROOT = ROOT / "docs/superpowers/specs"
REGISTER = ROOT / "RTS-Engine-Project-Specifications.md"
MIN_CASES = 21
RULES = {"R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9"}
EXEMPT = {
    "2026-10-26-m1-foundation-design.md",
    "2026-10-26-m2-economy-design.md",
    "2026-10-27-m3-industrial-design.md",
    "2026-10-28-m4-terrain-design.md",
}
NO_CLIENT = {"2026-10-29-qa0-instrument-design.md"}
NOT_MILESTONE = {
    "2026-10-25-studio-agent-team-design.md",
    "2026-10-26-m2-authoring-example.md",
}
ID = re.compile(r"^[A-Z]{2}[0-9]{2}$")
RED = re.compile(r"^RED (R[1-9]) (.+?) (\S+): ")


def read_tsv(path):
    with open(path, newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream, delimiter="\t"))


def yardstick_items():
    out = {}
    for row in read_tsv(YARDSTICK):
        out[row["id"]] = (row["tier"], [d.strip() for d in row["deps"].split(",") if ID.match(d.strip())])
    return out


def milestone_order():
    return {row["id"]: int(row["order"]) for row in read_tsv(MILESTONES)}


def markdown_sections(text):
    sections = {}
    current = None
    for line in text.splitlines():
        match = re.match(r"^##\s+(.+?)\s*$", line)
        if match:
            current = match.group(1).strip()
            sections.setdefault(current, [])
        elif current is not None:
            sections[current].append(line)
    return sections


def table_cells(line):
    value = line.strip()
    if not value.startswith("|"):
        return None
    return [cell.strip() for cell in value.strip("|").split("|")]


def diagnostic(rule, file_name, item, message):
    return "RED {} {} {}: {}".format(rule, file_name, item or "-", message)


def parse_coverage(path, milestone, items, milestones, register_ids):
    file_name = str(path)
    red = []
    text = path.read_text(encoding="utf-8")
    sections = markdown_sections(text)
    cov = next((lines for name, lines in sections.items() if name.lower() == "yardstick coverage"), None)
    acceptance = [line for name, lines in sections.items() if "acceptance" in name.lower() for line in lines]
    path_section = next((lines for name, lines in sections.items() if name.lower().startswith("player path")), None)
    core = {item for item, (tier, _) in items.items() if tier == "CORE"}
    rows = {}
    if cov is None:
        red.append(diagnostic("R1", file_name, "-", "no Yardstick coverage section"))
    else:
        for line in cov:
            cells = table_cells(line)
            if cells is None or len(cells) < 3 or not ID.match(cells[0]):
                continue
            item, disposition, test = cells[:3]
            if item not in items:
                red.append(diagnostic("R1", file_name, item, "row id is not in yardstick TSV"))
                continue
            if item in rows:
                red.append(diagnostic("R1", file_name, item, "duplicate coverage row"))
            else:
                rows[item] = (disposition, test)
            if not re.fullmatch(r"in|mod|later:[A-Za-z0-9]+|excluded:D[0-9]{2}|done:.+", disposition):
                red.append(diagnostic("R1", file_name, item, "bad disposition {!r}".format(disposition)))
            if disposition == "in":
                if test in ("", "-", "none"):
                    red.append(diagnostic("R2", file_name, item, "in row has no test id"))
                elif not any((table_cells(line) or [None])[0] == test for line in acceptance):
                    red.append(diagnostic("R2", file_name, item, "test id {} is not first cell of an acceptance row".format(test)))
            if disposition.startswith("later:"):
                later = disposition.split(":", 1)[1]
                if later not in milestones or milestone not in milestones or milestones.get(later, 0) <= milestones.get(milestone, 0):
                    red.append(diagnostic("R5", file_name, item, "later milestone {} is missing or not after {}".format(later, milestone)))
            if disposition.startswith("excluded:"):
                decision = disposition.split(":", 1)[1]
                if decision[1:] not in register_ids:
                    red.append(diagnostic("R6", file_name, item, "decision {} is not in section 2".format(decision)))
            if disposition.startswith("done:"):
                target = ROOT / disposition.split(":", 1)[1]
                if not target.exists():
                    red.append(diagnostic("R7", file_name, item, "done path does not exist: {}".format(disposition.split(":", 1)[1])))
        for item in sorted(core):
            if item not in rows:
                red.append(diagnostic("R1", file_name, item, "CORE item has no coverage row"))
    if path_section is None or not any(re.match(r"^\s*[0-9]+\.\s+", line) for line in path_section):
        red.append(diagnostic("R3", file_name, "-", "no Player path section with numbered step"))
    return red, rows


def rule4(red, specs, items, milestones):
    """Check in rows against CORE dependencies at this or an earlier milestone."""
    order = milestones
    evidence = {}
    entries = []
    for path, milestone, rows in specs:
        if milestone not in order:
            continue
        for item, (disposition, _) in rows.items():
            entries.append((path, milestone, item, disposition))
            if disposition in ("in",) or disposition.startswith("done:"):
                evidence.setdefault(item, []).append((order[milestone], disposition, path))
    for path, milestone, item, disposition in entries:
        if disposition != "in" or item not in items:
            continue
        for dep in items[item][1]:
            if items.get(dep, (None, []))[0] != "CORE":
                continue
            available = any(ms_order <= order[milestone] and (state == "in" or state.startswith("done:"))
                            for ms_order, state, _ in evidence.get(dep, []))
            if not available:
                red.append(diagnostic("R4", str(path), item, "CORE dependency {} is not in or done at or before {}".format(dep, milestone)))


def check_spec(path, milestone, items, milestones, register_ids, all_specs=None):
    red, rows = parse_coverage(path, milestone, items, milestones, register_ids)
    specs = list(all_specs or [])
    if all_specs is None:
        # R4 needs coverage evidence from every registered milestone spec.
        for row in read_registry(ROOT / "docs/qa/specs.tsv"):
            candidate = SPEC_ROOT / row["spec"]
            if candidate == path or row["client"] not in ("yes", "no") or not candidate.is_file():
                continue
            previous_rows = parse_coverage(candidate, row["milestone"], items, milestones, register_ids)[1]
            specs.append((candidate, row["milestone"], previous_rows))
    specs.append((path, milestone, rows))
    rule4(red, specs, items, milestones)
    return red


def read_registry(path):
    return read_tsv(path)


def registry_checks(specs_dir, registry_path):
    red = []
    registry = read_registry(registry_path)
    rows_by_name = {row["spec"]: row for row in registry}
    actual = {p.name for p in specs_dir.glob("*.md")}
    for name, row in rows_by_name.items():
        if not (specs_dir / name).is_file():
            red.append(diagnostic("R8", name, "-", "registry spec file does not exist"))
        client = row.get("client", "")
        if client == "no" and name not in NO_CLIENT:
            red.append(diagnostic("R9", name, "-", "client=no path is not in NO_CLIENT"))
        elif client == "n/a" and name not in NOT_MILESTONE:
            red.append(diagnostic("R9", name, "-", "client=n/a path is not in NOT_MILESTONE"))
        elif client not in ("yes", "no", "n/a"):
            red.append(diagnostic("R9", name, "-", "unknown client value {!r}".format(client)))
    for name in sorted(actual - set(rows_by_name)):
        red.append(diagnostic("R8", name, "-", "spec file has no registry row"))
    return red, registry


def enforce(specs_dir, registry_path):
    """Return the diagnostics and checked count used by CLI and self-test."""
    items = yardstick_items()
    milestones = milestone_order()
    register_ids = set(re.findall(r"^\| D([0-9]{2}) \|", REGISTER.read_text(encoding="utf-8"), re.M))
    red, registry = registry_checks(specs_dir, registry_path)
    specs = []
    checked = 0
    for row in registry:
        name = row["spec"]
        client = row["client"]
        if client == "n/a" or client not in ("yes", "no"):
            continue
        path = specs_dir / name
        if not path.is_file():
            continue
        milestone = row["milestone"]
        found, rows = parse_coverage(path, milestone, items, milestones, register_ids)
        # Exempt specs do not receive diagnostics, but their rows can satisfy R4.
        if client == "yes" and name in EXEMPT and row["gate"] != "gated":
            specs.append((path, milestone, rows))
            continue
        if client == "no":
            red.extend(line for line in found if RED.match(line) and RED.match(line).group(1) not in {"R1", "R2", "R3"})
        else:
            red.extend(found)
        specs.append((path, milestone, rows))
        checked += 1
    rule4(red, specs, items, milestones)
    return sorted(set(red)), checked


def print_lines(lines):
    for line in sorted(set(lines)):
        print(line)


def self_test():
    items = yardstick_items()
    milestones = milestone_order()
    register_ids = set(re.findall(r"^\| D([0-9]{2}) \|", REGISTER.read_text(encoding="utf-8"), re.M))
    registry = read_registry(ROOT / "docs/qa/specs.tsv")
    cases = read_tsv(CASES)
    failures = 0
    passed_cases = 0
    expected_all = set()
    for number, case in enumerate(cases, 1):
        expected = set() if case["expected_rules"] == "-" else set(case["expected_rules"].split(","))
        expected_all.update(expected)
        path_value = case["file"]
        if case["mode"] == "spec":
            path = ROOT / path_value
            got_lines = check_spec(path, case["as_milestone"], items, milestones, register_ids)
            if case["as_milestone"] in {"QA0", "MS"}:
                got_lines = [line for line in got_lines if not RED.match(line) or RED.match(line).group(1) not in {"R1", "R2", "R3"}]
        elif case["mode"] == "enforce-dir":
            base = ROOT / path_value
            got_lines, _checked = enforce(base / "specs", base / "specs.tsv")
        else:
            got_lines = [diagnostic("R1", path_value, "-", "unknown mode")]
        got = {match.group(1) for line in got_lines if (match := RED.match(line))}
        if got != expected:
            print("FAIL case {} {}: want {} got {}".format(number, path_value, sorted(expected), sorted(got)))
            for line in got_lines:
                print(line)
            failures += 1
        else:
            passed_cases += 1
    exempt_names = {row["spec"] for row in registry if row["spec"] in EXEMPT and row["gate"] != "gated"}
    for name in sorted(exempt_names):
        matched = any(c["mode"] == "spec" and Path(c["file"]).name == name and c["expected_rules"] != "-" for c in cases)
        if not matched:
            print("FAIL self-test: exempt spec {} has no red case".format(name))
            failures += 1
    for rule in sorted(RULES - expected_all):
        print("FAIL self-test: no case expects {}".format(rule))
        failures += 1
    if len(cases) < MIN_CASES:
        print("FAIL self-test: {}<{} cases".format(len(cases), MIN_CASES))
        failures += 1
    print("self-test: {}/{} cases passed; failures={}".format(passed_cases, len(cases), failures))
    return 1 if failures else 0


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--self-test", action="store_true")
    group.add_argument("--enforce", action="store_true")
    parser.add_argument("--specs-dir", default=str(SPEC_ROOT))
    parser.add_argument("--registry", default=str(ROOT / "docs/qa/specs.tsv"))
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    red, checked = enforce(Path(args.specs_dir), Path(args.registry))
    print_lines(red)
    print("checked specs: {}".format(checked))
    return 1 if red else 0


if __name__ == "__main__":
    sys.exit(main())
