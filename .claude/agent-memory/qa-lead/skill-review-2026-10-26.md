---
name: skill-review-2026-10-26
description: Verified facts from the third-party skill review for testing, CI, benchmarks, hosting, persistence (2026-10-26)
metadata:
  type: project
---

Verified on 2026-10-26: chrisgliddon/bevy-skills@bevy-testing pins Bevy 0.19; crates.io latest was 0.20.0; its API names (TimeUpdateStrategy::FixedTimesteps, MessageCursor, check_ready) exist on docs.rs 0.19.0. The project had no Cargo.toml, so the project Bevy version was NOT TRACED.
CodSpeed setup skill needs a SaaS login (`codspeed auth login`); simulation mode ignores I/O and threads. It does not satisfy spec 13.2 machine-configuration recording.
martinholovsky "SQLite Database Expert" has a Python TDD section and a HIGH/MEDIUM risk contradiction. Rejected.
Unauthenticated GitHub API tree lookups hit the rate limit; `gh api repos/<r>/git/trees/HEAD?recursive=1` works.

**Why:** the orchestrator asked which skills to install; the next review should not re-fetch these.
**How to apply:** re-verify the Bevy version against Cargo.toml before you trust the bevy-testing skill.
