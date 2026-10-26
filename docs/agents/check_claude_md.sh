#!/usr/bin/env bash
# Checks that every bold bullet label of design section 6 appears in CLAUDE.md.
# Usage: check_claude_md.sh [DESIGN] [CLAUDE_MD]
set -u
DESIGN="${1:-docs/superpowers/specs/2026-10-25-studio-agent-team-design.md}"
CLAUDE="${2:-CLAUDE.md}"
RC=0
labels="$(sed -n '/^## 6\. /,/^## 7\. /p' "$DESIGN" | grep -oE '^- \*\*[^*]+\*\*' | sed 's/^- \*\*//; s/\*\*$//')"
[ -z "$labels" ] && { echo "FAIL no labels found in $DESIGN section 6"; exit 1; }
while IFS= read -r label; do
    if ! grep -qF -- "**$label**" "$CLAUDE"; then
        echo "FAIL $label"
        RC=1
    fi
done <<< "$labels"
exit $RC
