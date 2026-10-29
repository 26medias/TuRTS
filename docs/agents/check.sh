#!/usr/bin/env bash
# Roster check for .claude/agents. See docs/superpowers/specs/2026-10-25-studio-agent-team-design.md section 7.
# Usage: check.sh [AGENTS_DIR] [SOURCE]
#   AGENTS_DIR  default .claude/agents
#   SOURCE      default $SOURCE or docs/agents/standing-rules.md
# Exit 0 when every check passes. Exit 1 and print one "FAIL <file> <check>: <detail>" line per failure.
set -u
DIR="${1:-.claude/agents}"
SOURCE="${2:-${SOURCE:-docs/agents/standing-rules.md}}"
SKIP_VALIDATE="${SKIP_VALIDATE:-0}"

python3 -I - "$DIR" "$SOURCE" <<'PY'
import os
import re
import sys

d, source = sys.argv[1], sys.argv[2]
NAMES = [
    "technical-director", "performance-architect", "simulation-core-engineer", "world-engineer",
    "network-engineer", "web-platform-engineer", "bevy-rendering-engineer", "mod-runtime-engineer",
    "backend-engineer", "ai-engineer", "game-designer", "qa-lead", "producer",
    "mod-builder", "player", "spec-rigour-reviewer", "ux-designer",
]
REVIEWER_ONLY = {"performance-architect", "player", "spec-rigour-reviewer"}
REV = {"Read", "Grep", "Glob", "Bash", "WebFetch", "WebSearch"}
BLD = REV | {"Write", "Edit"}
MEM = {"technical-director", "qa-lead", "mod-builder"}
SKILLS = {"spec-rigour-reviewer": "writing-simplified-technical-english"}
HEAD = ["Findings", "What I ran", "Not checked", "What I changed", "Open questions for the orchestrator"]
START, END = "<!-- standing-rules:start -->", "<!-- standing-rules:end -->"
fails = []


def fail(f, check, msg):
    fails.append(f"FAIL {f} {check}: {msg}")


def span(text):
    a, b = text.find(START), text.find(END)
    if a < 0 or b < 0 or b < a:
        return None
    return text[a + len(START):b]


def frontmatter(text):
    lines = text.split("\n")
    if not lines or lines[0] != "---":
        return None, text
    try:
        end = lines[1:].index("---") + 1
    except ValueError:
        return None, text
    fields = {}
    for ln in lines[1:end]:
        m = re.match(r"^([A-Za-z]+):\s?(.*)$", ln)
        if m:
            fields[m.group(1)] = m.group(2)
        else:
            fields.setdefault("_continuation", []).append(ln)
    return fields, "\n".join(lines[end + 1:])


def decode_description(raw):
    """Return the parsed value of a one-line double-quoted YAML string, or None."""
    raw = raw.rstrip()
    if len(raw) < 2 or raw[0] != '"' or raw[-1] != '"':
        return None
    inner = raw[1:-1]
    out, i = [], 0
    while i < len(inner):
        c = inner[i]
        if c == "\\" and i + 1 < len(inner):
            n = inner[i + 1]
            out.append({"n": "\n", "t": "\t", '"': '"', "\\": "\\"}.get(n, "\\" + n))
            i += 2
        elif c == '"':
            return None
        else:
            out.append(c)
            i += 1
    return "".join(out)


def toolset(raw):
    return {x.strip() for x in raw.split(",") if x.strip()}


try:
    src = span(open(source, encoding="utf-8").read())
except OSError:
    src = None
if src is None:
    fail("-", "block", f"source {source} missing or has no markers")

seen = {}
for fn in sorted(os.listdir(d)):
    path = os.path.join(d, fn)
    if not fn.endswith(".md") or not os.path.isfile(path):
        continue
    text = open(path, encoding="utf-8").read()
    f, body = frontmatter(text)
    if f is None or "name" not in f:
        fail(fn, "names", "no name: in frontmatter")
        continue
    n = f["name"].strip().strip('"')
    if n in seen:
        fail(fn, "names", f"duplicate name {n} (also in {seen[n]})")
    seen[n] = fn
    if fn != n + ".md":
        fail(fn, "names", f"file name differs from name {n}")

    raw_desc = f.get("description", "")
    val = decode_description(raw_desc)
    if val is None or "_continuation" in f:
        fail(fn, "desc", "description must be one physical double-quoted line")
        val = ""
    else:
        size = len(val.encode("utf-8"))
        if size > 900:
            fail(fn, "desc", f"{size} parsed bytes, cap 900")
    if val.count("<example>") != 1:
        fail(fn, "desc", f"{val.count('<example>')} <example> blocks, need 1")
    if "Modes:" not in val:
        fail(fn, "desc", "no Modes: line")

    tools = toolset(f.get("tools", ""))
    want = REV if n in REVIEWER_ONLY else BLD
    if n in NAMES and tools != want:
        fail(fn, "tools", f"got {sorted(tools)} want {sorted(want)}")
    if "Agent" in tools or "Skill" in tools:
        fail(fn, "tools", "has Agent or Skill")

    if f.get("model", "").strip() != "inherit":
        fail(fn, "model", "model: inherit missing")
    if (f.get("memory", "").strip() == "project") != (n in MEM):
        fail(fn, "memory", f"memory: {f.get('memory', '').strip() or 'none'}")
    skills = f.get("skills", "").strip()
    if n in SKILLS:
        if skills != SKILLS[n]:
            fail(fn, "skills", f"skills: {skills or 'none'}, want {SKILLS[n]}")
    elif skills:
        fail(fn, "skills", f"skills: {skills} not allowed")

    sp = span(body)
    if src is not None:
        if sp is None:
            fail(fn, "block", "marker lines missing")
        elif sp != src:
            fail(fn, "block", "block differs from source")
    for h in HEAD + ["VERDICT="]:
        if h not in body:
            fail(fn, "contract", f"missing {h}")

missing = set(NAMES) - set(seen)
extra = set(seen) - set(NAMES)
if missing:
    fail("-", "names", f"missing {sorted(missing)}")
if extra:
    fail("-", "names", f"unknown {sorted(extra)}")

for line in fails:
    print(line)
sys.exit(1 if fails else 0)
PY
rc=$?

if [ "$SKIP_VALIDATE" != "1" ]; then
    out="$(claude plugin validate "$DIR" 2>&1)"
    vrc=$?
    if [ "$vrc" -ne 0 ]; then
        echo "FAIL - validate: exit $vrc"
        echo "$out" | head -20
        rc=1
    fi
fi
exit "$rc"
