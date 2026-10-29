#!/usr/bin/env python3
"""Check roles and pixels-only constraints for browser test files."""
import argparse
import csv
import os
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
SAMPLES = ROOT / "tests/qa/driver"
ROLES = {"driver", "pure", "oracle", "instrument", "unit", "legacy-bridge"}
LEDGER_LIMIT = 5
DRIVER_PART_A = {"A1", "A2", "A3", "A4", "A5", "A6", "A7"}
CHANNEL_MODULES = {
    "child_process", "vm", "worker_threads", "inspector", "module", "net", "http",
    "https", "http2", "dgram", "cluster", "repl",
}
BUILTINS = {
    "assert", "async_hooks", "buffer", "child_process", "cluster", "console", "constants",
    "crypto", "dgram", "diagnostics_channel", "dns", "domain", "events", "fs", "http",
    "http2", "https", "inspector", "module", "net", "os", "path", "perf_hooks",
    "process", "punycode", "querystring", "readline", "repl", "stream", "string_decoder",
    "sys", "timers", "tls", "trace_events", "tty", "url", "util", "v8", "vm",
    "wasi", "worker_threads", "zlib",
}
PROCESS_MEMBERS = {"binding", "_linkedBinding", "dlopen", "mainModule", "getBuiltinModule"}


def line_of(source, offset):
    return source.count("\n", 0, offset) + 1


def without_comments(source):
    """Remove JS comments while preserving offsets and newlines."""
    chars = list(source)
    i = 0
    quote = None
    escaped = False
    while i < len(chars):
        c = chars[i]
        if quote:
            if escaped:
                escaped = False
            elif c == "\\":
                escaped = True
            elif c == quote:
                quote = None
            i += 1
            continue
        if c in "'\"`":
            quote = c
            i += 1
            continue
        if c == "/" and i + 1 < len(chars) and chars[i + 1] == "/":
            start = i
            while i < len(chars) and chars[i] != "\n":
                chars[i] = " "
                i += 1
            if i < len(chars):
                chars[i] = "\n"
            continue
        if c == "/" and i + 1 < len(chars) and chars[i + 1] == "*":
            chars[i] = chars[i + 1] = " "
            i += 2
            while i < len(chars):
                if chars[i] == "*" and i + 1 < len(chars) and chars[i + 1] == "/":
                    chars[i] = chars[i + 1] = " "
                    i += 2
                    break
                if chars[i] != "\n":
                    chars[i] = " "
                i += 1
            continue
        i += 1
    return "".join(chars)


def without_strings(source):
    """Mask string text but retain executable code in template expressions."""
    chars = list(source)
    frames = [{"kind": "code", "brace": None}]
    i = 0
    while i < len(chars):
        c = chars[i]
        frame = frames[-1]
        if frame["kind"] == "string":
            if c != "\n":
                chars[i] = " "
            if frame["escaped"]:
                frame["escaped"] = False
            elif c == "\\":
                frame["escaped"] = True
            elif c == frame["quote"]:
                frames.pop()
            i += 1
            continue
        if frame["kind"] == "template":
            if c != "\n":
                chars[i] = " "
            if frame["escaped"]:
                frame["escaped"] = False
            elif c == "\\":
                frame["escaped"] = True
            elif c == "`":
                frames.pop()
            elif c == "$" and i + 1 < len(chars) and chars[i + 1] == "{":
                chars[i + 1] = " "
                frames.append({"kind": "code", "brace": 1})
                i += 2
                continue
            i += 1
            continue
        if c in "'\"":
            chars[i] = " "
            frames.append({"kind": "string", "quote": c, "escaped": False})
        elif c == "`":
            chars[i] = " "
            frames.append({"kind": "template", "escaped": False})
        elif frame["brace"] is not None and c == "{":
            frame["brace"] += 1
        elif frame["brace"] is not None and c == "}":
            frame["brace"] -= 1
            if frame["brace"] == 0:
                chars[i] = " "
                frames.pop()
        i += 1
    return "".join(chars)


def read_role(path):
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError):
        return None, 1, "cannot read file"
    entries = [(index, line[len("// turts-test-role: "):])
               for index, line in enumerate(lines, 1)
               if line.startswith("// turts-test-role: ")]
    if not entries:
        return None, 1, "missing role line"
    if len(entries) > 1:
        return None, entries[1][0], "multiple role lines"
    line, role = entries[0]
    if line != 1:
        return None, line, "role line must be first"
    if role not in ROLES:
        return None, line, "unknown role"
    return role, line, ""


def quoted_require_args(code):
    """Return require-like call sites and their literal module names, if any."""
    calls = []
    pattern = re.compile(r"(?<![\w$.])(?:module\s*\.\s*)?require\b")
    for match in pattern.finditer(code):
        tail = code[match.end():]
        open_paren = re.match(r"\s*\(\s*", tail)
        if not open_paren:
            calls.append((match.start(), None, False))
            continue
        arg_start = match.end() + open_paren.end()
        literal = re.match(r"(['\"])([^'\"\n]*)\1\s*\)", code[arg_start:])
        if literal:
            calls.append((match.start(), literal.group(2), True))
        else:
            calls.append((match.start(), None, True))
    return calls


def part_a(source):
    code = without_comments(source)
    found = {}

    def add(rule, offset, text):
        found.setdefault(rule, (line_of(source, offset), text))

    for match in re.finditer(r"(?<![\w$])turts_[\w$]*", without_strings(code)):
        add("A1", match.start(), "bridge identifier")
    for match in re.finditer(r"\[\s*(?:(['\"])(turts_[\w$]+)\1|`(turts_[\w$]+)`)\s*\]", code):
        add("A1", match.start(), "bridge identifier")
    for pattern, scanned in (
        (r"(?<![\w$])eval(?![\w$])", code),
        (r"(?<![\w$])Function(?![\w$])", code),
        (r"\.\s*constructor\b", code),
        (r"\[\s*(['\"])constructor\1\s*\]", code),
        (r"(?:window|global|globalThis)\s*(?:\.\s*(?:eval|Function)|\[\s*(?:(['\"])(?:eval|Function)\1|`(?:eval|Function)`)\s*\])", code),
    ):
        match = re.search(pattern, scanned)
        if match:
            add("A2", match.start(), "dynamic code")
    for match in re.finditer(r"(?<![\w$.])(?:global|globalThis)(?![\w$])", code):
        add("A3", match.start(), "global access")

    roots = r"(process|global|globalThis|window|module)"
    for match in re.finditer(roots + r"\s*(?:\?\.\s*)?\[([^\]\n]*)\]", code):
        root = match.group(1)
        key = match.group(2).strip()
        literal = re.fullmatch(r"(['\"])([^'\"+`$]*)\1", key)
        if not literal:
            add("A4", match.start(), "computed member with non-literal key")
        elif root == "process" and literal.group(2) in PROCESS_MEMBERS:
            add("A4", match.start(), "process code-loading member")
        elif root == "module" and literal.group(2) != "exports":
            add("A4", match.start(), "module member other than exports")
    for match in re.finditer(r"\[[^\]\n]*(?:['\"`][^\]\n]*\+|\+[^\]\n]*['\"])[^\]\n]*\]", code):
        add("A4", match.start(), "string-built computed key")
    for match in re.finditer(r"\[\s*`[^`]*\$\{", code):
        add("A4", match.start(), "template-built computed key")
    for match in re.finditer(r"(?<![\w$.])process\s*(?:\?\.\s*)?\.\s*([\w$]+)", code):
        if match.group(1) in PROCESS_MEMBERS:
            add("A4", match.start(), "process code-loading member")
    for match in re.finditer(r"(?<![\w$.])module\s*(?:\?\.\s*)?\.\s*([\w$]+)", code):
        if match.group(1) != "exports":
            add("A4", match.start(), "module member other than exports")

    if re.search(r"\bimport\s*\(", code):
        match = re.search(r"\bimport\s*\(", code)
        add("A5", match.start(), "dynamic import")
    calls = {offset: (module_name, called) for offset, module_name, called in quoted_require_args(code)}
    for match in re.finditer(r"(?<![\w$])require\b", code):
        offset = match.start()
        call = calls.get(offset)
        if code[max(0, offset - 8):offset].endswith("module."):
            continue
        if call is None:
            add("A5", offset, "require used as a value")
        elif not call[1] or call[0] is None:
            add("A5", offset, "require argument is not one string literal")
    for match in re.finditer(r"(?<![\w$.])require\s*\.\s*(?:cache|main)\b", code):
        add("A5", match.start(), "require cache or main")

    for match in re.finditer(r"(?<![\w$.])(?:fetch|WebSocket)(?![\w$])", code):
        add("A6", match.start(), "network global")
    for offset, module_name, called in quoted_require_args(code):
        if not called or module_name is None or module_name.startswith("./") or module_name.startswith("../"):
            continue
        base = module_name[5:] if module_name.startswith("node:") else module_name
        base = base.split("/")[0]
        if base in CHANNEL_MODULES:
            add("A6", offset, "second-channel built-in " + module_name)
        elif not module_name.startswith("node:") and base not in BUILTINS:
            add("A6", offset, "npm package " + module_name)

    for offset, module_name, called in quoted_require_args(code):
        if called and module_name is not None:
            base = module_name[5:] if module_name.startswith("node:") else module_name
            if base.split("/")[0] == "fs":
                add("A7", offset, "file-system access " + module_name)
    return found


def collect_file(path, closure, missing):
    path = path.resolve()
    if path in closure:
        return
    closure.add(path)
    try:
        source = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        missing.append((path, 1))
        return
    code = without_comments(source)
    for offset, module_name, called in quoted_require_args(code):
        if called and module_name and (module_name.startswith("./") or module_name.startswith("../")):
            target = (path.parent / module_name).resolve()
            if not target.suffix:
                target = target.with_suffix(".js")
            collect_file(target, closure, missing)


def check_driver(path):
    found = set()
    closure = set()
    missing = []
    collect_file(path, closure, missing)
    driver = path.resolve()
    roles = {}
    for item in closure:
        role, line, error = read_role(item)
        roles[item] = role
        if error:
            found.add(("DR1", None, item, line, error))
    for item in closure:
        if item == driver:
            continue
        if roles.get(item) not in {"driver", "pure"}:
            found.add(("DR3", None, driver, 1, "local require has role other than driver or pure: " + str(item)))
    for item in closure:
        try:
            source = item.read_text(encoding="utf-8")
        except (OSError, UnicodeError):
            continue
        for rule, (line, text) in part_a(source).items():
            found.add(("DR2", rule, item, line, text))
    return found


def check_pure(path):
    found = set()
    role, line, error = read_role(path)
    if error:
        found.add(("DR1", None, path, line, error))
    for rule, (line, text) in part_a(path.read_text(encoding="utf-8")).items():
        if rule != "A7":
            found.add(("DR4", rule, path, line, text))
    return found


def check_oracle(path):
    found = set()
    role, line, error = read_role(path)
    if error:
        found.add(("DR1", None, path, line, error))
    source = path.read_text(encoding="utf-8")
    code = without_comments(source)
    banned = re.search(r"(?<![\w$])(?:mouse|keyboard|touchscreen|dispatchEvent|turts_move|evaluate|page)(?![\w$])", code)
    if banned:
        found.add(("DR5", None, path, line_of(source, banned.start()), "oracle contains a browser-driving name"))
    for rule in ("A2", "A5"):
        if rule in part_a(source):
            line, _ = part_a(source)[rule]
            found.add(("DR5", None, path, line, "oracle contains " + rule))
    if re.search(r"require\s*\(\s*(['\"])playwright-core\1", code):
        match = re.search(r"require\s*\(\s*(['\"])playwright-core\1", code)
        found.add(("DR5", None, path, line_of(source, match.start()), "oracle requires playwright-core"))
    return found


def read_ledger(path):
    try:
        with path.open(encoding="utf-8", newline="") as stream:
            rows = list(csv.DictReader(stream, delimiter="\t"))
    except (OSError, UnicodeError, csv.Error):
        return [], False
    good_header = bool(rows) or path.read_text(encoding="utf-8").splitlines()[:1] == ["file\tuntil"]
    return rows, good_header


def check_directory(directory):
    found = set()
    js_files = []
    for current, directories, filenames in os.walk(directory):
        directories[:] = sorted(name for name in directories if name != "node_modules")
        js_files.extend(Path(current) / name for name in filenames if name.endswith(".js"))
    js_files.sort()
    for path in js_files:
        found.update(check_file(path))
    ledger_path = directory / "legacy-bridge.tsv"
    rows, header_ok = read_ledger(ledger_path)
    if not header_ok:
        found.add(("DR6", None, ledger_path, 1, "missing or invalid legacy bridge ledger"))
        rows = []
    ledger_names = {row.get("file", "") for row in rows}
    for path in js_files:
        role, _, _ = read_role(path)
        items = part_a(path.read_text(encoding="utf-8"))
        name = path.name
        if role == "legacy-bridge" and name not in ledger_names:
            found.add(("DR6", None, path, 1, "legacy-bridge file is not in ledger"))
        if name in ledger_names and role == "legacy-bridge" and not items:
            found.add(("DR6", None, path, 1, "ledgered legacy-bridge has no Part A item"))
    if len(rows) > LEDGER_LIMIT:
        found.add(("DR7", None, ledger_path, 1, "legacy bridge ledger exceeds limit " + str(LEDGER_LIMIT)))
    return found


def check_file(path):
    role, line, error = read_role(path)
    if error:
        return {("DR1", None, path, line, error)}
    if role == "driver":
        return check_driver(path)
    if role == "pure":
        return check_pure(path)
    if role == "oracle":
        return check_oracle(path)
    return set()


def check_path(path, mode=None):
    if mode == "dir" or (mode is None and path.is_dir()):
        return check_directory(path)
    return check_file(path)


def rule_tokens(findings):
    result = set()
    for driver_rule, part_a_rule, _, _, _ in findings:
        result.add(driver_rule)
        if part_a_rule:
            result.add(part_a_rule)
    return result


def parse_rules(value):
    if value == "-":
        return set()
    return set(value.split(","))


def format_findings(findings):
    ordered = sorted(findings, key=lambda item: (item[0], item[1] or "", str(item[2]), item[3], item[4]))
    for rule, part_a_rule, path, line, text in ordered:
        suffix = " " + part_a_rule if part_a_rule else " -"
        print(f"RED {rule}{suffix} {path}:{line}: {text}")


def self_test():
    cases_path = SAMPLES / "cases.tsv"
    try:
        with cases_path.open(encoding="utf-8", newline="") as stream:
            rows = list(csv.DictReader(stream, delimiter="\t"))
    except (OSError, UnicodeError, csv.Error) as error:
        print("driver rule self-test: cannot read cases: " + str(error))
        return 1
    expected_rules = set()
    failed = False
    for row in rows:
        path = (SAMPLES / row["path"]).resolve()
        findings = check_path(path, row["mode"])
        actual = rule_tokens(findings)
        expected = parse_rules(row["expected"])
        expected_rules.update(expected)
        if actual != expected:
            failed = True
            print(f"CASE {row['path']}: want {sorted(expected)} got {sorted(actual)}")
            format_findings(findings)
    required_rules = {f"DR{i}" for i in range(1, 8)} | {f"A{i}" for i in range(1, 8)}
    if len(rows) < 40:
        failed = True
        print(f"CASE COUNT: want at least 40 got {len(rows)}")
    for rule in sorted(required_rules - expected_rules):
        failed = True
        print("CASE COVERAGE: missing expected rule " + rule)
    print(f"driver rule self-test: {len(rows)} cases; {'FAIL' if failed else 'PASS'}")
    return 1 if failed else 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("paths", nargs="*")
    args = parser.parse_args(argv)
    if args.self_test:
        if args.paths:
            parser.error("--self-test does not take paths")
        return self_test()
    if not args.paths:
        parser.error("provide one or more files or directories")
    failed = False
    for name in args.paths:
        path = Path(name)
        findings = check_path(path)
        if findings:
            failed = True
            format_findings(findings)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
