

## C383: start Task 21 source work when its Needs are merged; builder model by task risk (2026-11-11)

Conflict: the wave table puts Task 21 in wave 9, after wave 8 (14, 16, 22). The CEO asked on 2026-11-11 for faster development with no loss of quality. Task 21 needs 14, 16, 17, 18 and 20 only (plan line 674); all five are merged at d967f2a. Task 22 is not a Need of Task 21, and the two tasks share one file, scripts/mc-browser.sh (Task 22: the order-log capture; Task 21: params.bindings).

Choice: Task 21 starts now on its own branch and worktree as source preparation, in parallel with the Task 22 runtime (the pattern of C260, C269 and C340). One compiler/browser/GPU lease stays: RUN112 (Task 22) holds it; RUN113 (Task 21) runs no cargo, trunk, browser or server until Root moves the lease. Task 22 merges first; Task 21 rebases on it and then takes its runtime. The wave 8 push and CI read stay after the Task 22 merge. No gate, review, fault cycle or acceptance row is dropped. Model: this build's remaining builder work is instrument and browser-harness work with a high repair cost, so the builders of Tasks 21 and 22 run on claude-opus-5-5, effort high; this narrows the builder line of C382 (Sonnet stays the choice for fully specified mechanical code).

Evidence: plan lines 46, 656, 674, 693; board section 5; brief section 4 line 52 (order of tasks, number of lanes); cache_handoff_v2.py and exec_in_lane_v2.py (one active compiler asserted).

Alternative not taken: keep Task 21 idle until Task 22 merges. It costs wall time and adds no check.

Global113/170 after the two registrations.
