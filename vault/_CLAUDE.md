# TuRTS studio vault

Vault marker for `obsidian-vault-mcp`. Pass `vault: /home/julien/Projects/TuRTS/vault` on every
obsidian tool call. This is a dedicated vault for the TuRTS studio only. It lives in the
repository and is committed. It is the studio's work log and the CEO's window on the project.

## Layout

- `Home.md`: dashboard. Status, current milestone, links.
- `Decisions/`: one note per decision. Date, who decided, status (studio call, CEO decision, open).
- `Tasks.md`: the board. Backlog, in progress, done. Owner and link per task.
- `Work Log/YYYY-MM-DD.md`: what happened that day, by whom, with evidence links.
- `Reports/YYYY-MM-DD.md`: the daily report for the CEO.
- `Knowledge/`: shared facts the studio verified (tooling hazards, library facts, process).
- `Team.md`: the roster, the model tiers, and the review process.
- `Inbox/`: notes written through the MCP connector land here; curate them into the folders above.

## Note format

```markdown
---
type: note | decision | log | report | task
date: YYYY-MM-DD
tags:
  - turts
ai-first: true
source: mcp
---

## For future Claude
One-line standing summary.

Body.
```

## Rules

- Every decision gets a note before the work that depends on it starts.
- Every work-log entry names the agent (seat) that did the work and the commit or file.
- The daily report is written at the end of each working session, even a short one.
- Do not store secrets, account names, or project IDs here. The vault is committed to a public repository.
