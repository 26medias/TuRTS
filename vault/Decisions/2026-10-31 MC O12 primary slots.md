---
type: decision
date: 2026-10-31
tags: [turts, mc, ruling]
source: CEO and orchestrator
---


## O12: resume Task 6 (2026-10-31)

CEO ruling O12, main721e56ee020cbb60c23d2b99acd1fdfd7367d509, design revision13: all N primary slots are required. Fewer than N legal slots fails the envelope atomically with blocked for each unit; no unit moves. Spares are the remaining legal slots, zero or more, up to the existing maximum. Section6.4 is unchanged: with no spare, one second without progress becomes Blocked at once. The earlier pool-capacity stop is released.

The CEO clarified brief section4: when an existing mechanism cannot meet a design rule and the same player-visible shape is kept, Codex decides the mechanism conflict as C<n>, records evidence and the rejected alternative, and continues. Do not escalate the next such conflict merely because the design wording needs an applicability ruling. Frozen/K/Decided changes, changed player-visible shape and the other explicit stop conditions remain.

Integration rebased onto721e56e. The only conflict was the daily work log add/add; both main O12 and integration history were retained. All open lanes T6/T7/T11/T12 rebased onto the resulting integration8eff3c4. All14 dirty or untracked source files are byte-identical before/after; explicit-path stashes retained as backup. Receipt: main target/mc-evidence/o12-rebase/receipt.json. No new cache was created. RUN46 registered before Task6 Luna/high continuation;46/170 total, Task6run4/estimate6. Task6 alone has the compiler lease. Review/source preparation in other lanes remains idle. No main merge or push.
