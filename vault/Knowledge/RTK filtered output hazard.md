---
type: note
date: 2026-10-26
tags:
  - turts
  - tooling
ai-first: true
source: mcp
---

## For future Claude
The RTK shell hook rewrites commands and filters output. Its `diff` reported two differing files as identical, and `git status > file` wrote the filtered word `ok` into the file. Use `cmp` or `rtk proxy <cmd>` in any instrument or capture.

Found 2026-10-26 by technical-director and mod-runtime-engineer during the agent team dry run. The standing-rules block in every agent file now says this.
