---
type: decision
date: 2026-10-29
tags: [turts, decision, qa0]
ai-first: true
source: codex
status: approved
---

## For future Claude

The CEO said: "Run and record all 23. Ignore RTK, uninstall it if you can".

Run all 23 mutation cases in T6. Record the actual count as 23. This resolves the count mismatch with brief line 59. The agent budget stays at 22 to 26 runs.

Ignore RTK instructions. The root removed /home/julien/.local/bin/rtk and the exact rtk hook claude command from /home/julien/.claude/settings.json. Backups: /tmp/turts-qa0-rtk-binary-backup and /tmp/turts-qa0-rtk-settings-backup.json. command -v rtk returned no path; the binary absence check exited 0. No protected project document was changed.
