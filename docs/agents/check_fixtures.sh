#!/usr/bin/env bash
# Fixture test for docs/agents/check.sh. Builds planted cases in the scratchpad and asserts the exit code.
# Usage: check_fixtures.sh    (env SCRATCH overrides the fixture root)
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
SRC="$HERE/standing-rules.md"
SCRATCH="${SCRATCH:-${TMPDIR:-/tmp}/turts-agent-fixtures}"
FX="$SCRATCH/fixtures"
rm -rf "$FX"
mkdir -p "$FX"

BLOCK="$(sed -n '/standing-rules:start/,/standing-rules:end/p' "$SRC")"
NAMES="technical-director performance-architect simulation-core-engineer world-engineer network-engineer web-platform-engineer bevy-rendering-engineer mod-runtime-engineer backend-engineer ai-engineer game-designer qa-lead producer mod-builder player spec-rigour-reviewer ux-designer"
REV="Read, Grep, Glob, Bash, WebFetch, WebSearch"
DESC='Seat sentence. Modes: review-spec.\n\n<example>\nContext: x\nuser: \"x\"\nassistant: \"x\"\n<commentary>\nx\n</commentary>\n</example>'

# mk DIR NAME [DESC] [TOOLS] [MODEL] [MEMORY] [BLOCK] [HEADINGS] [SKILLS] [FILENAME]
mk() {
    local dir=$1 n=$2 desc=${3:-$DESC} tools=${4:-auto} model=${5:-inherit} mem=${6:-auto} block=${7:-$BLOCK} heads=${8:-all} skills=${9:-auto} fname=${10:-$2}
    if [ "$tools" = auto ]; then
        case $n in
            performance-architect|player|spec-rigour-reviewer) tools="$REV" ;;
            *) tools="$REV, Write, Edit" ;;
        esac
    fi
    if [ "$mem" = auto ]; then
        case $n in
            technical-director|qa-lead|mod-builder) mem=project ;;
            *) mem="" ;;
        esac
    fi
    [ "$mem" = none ] && mem=""
    if [ "$skills" = auto ]; then
        case $n in
            spec-rigour-reviewer) skills=writing-simplified-technical-english ;;
            *) skills="" ;;
        esac
    fi
    {
        echo '---'
        echo "name: $n"
        if [ "$desc" = MULTILINE ]; then
            printf 'description: |\n    Seat sentence. Modes: review-spec.\n    <example>x</example>\n'
        else
            echo "description: \"$desc\""
        fi
        echo "tools: $tools"
        [ -n "$model" ] && echo "model: $model"
        [ -n "$mem" ] && echo "memory: $mem"
        echo "color: blue"
        [ -n "$skills" ] && echo "skills: $skills"
        echo '---'
        echo "## Identity"
        echo "## Read first"
        echo "## Standing rules"
        echo "$block"
        echo "## Expertise"
        echo "## Modes"
        echo "## Output contract"
        echo 'Line 1: VERDICT=...'
        if [ "$heads" = all ]; then
            printf -- '- Findings\n- What I ran\n- Not checked\n- What I changed\n- Open questions for the orchestrator\n'
        else
            printf -- '- Findings\n- What I ran\n- What I changed\n- Open questions for the orchestrator\n'
        fi
    } > "$dir/$fname.md"
}

good() {
    mkdir -p "$1"
    for n in $NAMES; do mk "$1" "$n"; done
}

RC=0
run() {
    local cs=$1 exp=$2
    local dir="$FX/$cs/.claude/agents"
    SKIP_VALIDATE="${SKIP_VALIDATE:-0}" bash "$HERE/check.sh" "$dir" "$SRC" > "$FX/$cs.out" 2>&1
    local got=$?
    if [ "$got" = "$exp" ]; then
        echo "PASS $cs"
    else
        echo "FAIL $cs expected $exp got $got"
        sed 's/^/    /' "$FX/$cs.out"
        RC=1
    fi
}

d="$FX/good/.claude/agents"; good "$d"
d="$FX/no_name/.claude/agents"; good "$d"; printf -- '---\ndescription: "no name"\n---\nbody\n' > "$d/stray.md"
d="$FX/dup_name/.claude/agents"; good "$d"; cp "$d/player.md" "$d/player2.md"
d="$FX/filename_mismatch/.claude/agents"; good "$d"; rm "$d/player.md"; mk "$d" player "$DESC" auto inherit auto "$BLOCK" all auto the-player
d="$FX/desc_901/.claude/agents"; good "$d"
    pad=$(( 901 - $(python3 -I -c 'import sys; print(len(sys.argv[1].replace("\\n", "\n").replace("\\\"", "\"").encode()))' "$DESC") ))
    mk "$d" player "$DESC$(printf '%*s' "$pad" '' | tr ' ' a)"
d="$FX/two_examples/.claude/agents"; good "$d"; mk "$d" player "$DESC<example>b</example>"
d="$FX/desc_multiline/.claude/agents"; good "$d"; mk "$d" player MULTILINE
d="$FX/block_drift/.claude/agents"; good "$d"; mk "$d" player "$DESC" auto inherit auto "$(echo "$BLOCK" | sed '3s/$/ /')"
d="$FX/missing_heading/.claude/agents"; good "$d"; mk "$d" player "$DESC" auto inherit auto "$BLOCK" four
d="$FX/agent_tool/.claude/agents"; good "$d"; mk "$d" player "$DESC" "$REV, Agent"
d="$FX/reviewer_write/.claude/agents"; good "$d"; mk "$d" player "$DESC" "$REV, Write"
d="$FX/memory_wrong/.claude/agents"; good "$d"; mk "$d" performance-architect "$DESC" auto inherit project
d="$FX/memory_missing/.claude/agents"; good "$d"; mk "$d" qa-lead "$DESC" auto inherit none
d="$FX/no_inherit/.claude/agents"; good "$d"; sed -i '/^model: inherit$/d' "$d/player.md"
d="$FX/skills_wrong/.claude/agents"; good "$d"; mk "$d" producer "$DESC" auto inherit auto "$BLOCK" all writing-simplified-technical-english

for c in good:0 no_name:1 dup_name:1 filename_mismatch:1 desc_901:1 two_examples:1 desc_multiline:1 block_drift:1 missing_heading:1 agent_tool:1 reviewer_write:1 memory_wrong:1 memory_missing:1 no_inherit:1 skills_wrong:1; do
    run "${c%%:*}" "${c##*:}"
done

# Assert the planted size of desc_901 as parsed bytes.
planted=$(python3 -I - "$FX/desc_901/.claude/agents/player.md" <<'PY'
import re, sys
t = open(sys.argv[1], encoding="utf-8").read()
raw = re.search(r'^description: "(.*)"$', t, re.M).group(1)
val = raw.replace('\\n', '\n').replace('\\"', '"')
print(len(val.encode()))
PY
)
if [ "$planted" = 901 ]; then
    echo "PASS desc_901 planted size is 901"
else
    echo "FAIL desc_901 planted size is $planted"
    RC=1
fi
exit $RC
