# Handoff to Codex, 2026-11-12 (evening)

Written by the orchestrator session (Claude). The CEO asked for this handoff because of token use.
Section 9, "Final state at handoff", is the last thing written. Read it first: it corrects the
lane states below where they moved.

## 0. The CEO changed the working mode today. Read this first.

The CEO asked why this project is planned in months when people build whole games with these
models in hours. The honest answer: the scope is an engine (authoritative multiplayer, browser
client, mod API), and the orchestrator put a heavy process on every task. One slice task took
seven agent passes in a day. After five milestones the CEO tried the build two times and could
not play it.

CEO words (2026-11-12): "The anvil for everything for example must go, we anvil only the big
specs otherwise we waste time and tokens. You know I don't want to spend 6 months on it, I want
to build it as fast as possible without sacrificing quality."

The new rules are in `/home/julien/Projects/TuRTS/CLAUDE.md`, section "Working mode: playable
first, speed first" (working tree of `main`, not committed: only the CEO commits to main), and in
`~/.claude/CLAUDE.md`. In short:

- **Playable first.** The priority is a skirmish a person can play in the browser: build a base,
  harvest, make units, fight, win or lose (MP1 checkpoint C1). Each working day ends with a build
  that the scripted player probes (`docs/qa/cold-play.md` on `mp1/integration`) can play further
  than the day before.
- **Two tracks.** Foundation (server authority, wire protocol, determinism, the mod API contract):
  full process. Everything else: build straight through, run it, ONE review at the end of the
  feature, merge.
- **No review for each task, no follow-up pass on a PASS, no re-check loops.** A repair gets one
  check of the repaired items. Minor findings go on one list.
- **Machinery on a diet.** The plant registry, receipt ledgers, per-task rulings and per-lane
  worktrees are for the Foundation track only.
- The full gate set and acceptance run one time for each integration push, not for each lane.

**What this means for the lanes below.** They were built under the old mode and are nearly done.
Finish each with ONE more step, not a new loop: merge what has a PASS; give a FAIL one repair and
one check. Then stop the slice-style work and put the team on the playable track:
1. Land the forward merge, L0-STORE and sound-off on `mp1/integration` (section 5).
2. Land A1d and A2b1 on the slice branch (section 4). Do A2c only after the design change in
   section 3. Ask yourself for each remaining slice task (A2b2, A4, A3, REG, TDN): does the first
   playable build need it? If not, park it and say so to the CEO in one line.
3. Go straight for C1: cut the MP1 plan (76 tasks) to the shortest path to a playable loop. Build
   it with the light path. Show the CEO a build only when the readiness probes pass.

## 1. Rules that bind you

- **Never merge or push `main`.** The CEO said so; it stays in force.
- Write all text in ASD-STE100 Simplified Technical English (`~/.claude/CLAUDE.md`).
- Verify, never assume. Cite `path:line`. Label what you did not check `NOT TRACED`. An agent
  summary is a lead, not proof.
- When there is a review, the reviewer runs things. One review for each feature (section 0).
- Never `git add -A`. Stage explicit paths. Never commit in a worktree while a gate run uses it
  (the runner binds a run to HEAD).
- **Sound (CEO, absolute):** no test browser may play sound on the desktop. Every Chrome start
  passes `--alsa-output-device=null` as an ARGUMENT and uses the silent environment. Headed rows
  run only through `target/mc-evidence/gate-runner/mc_headed_v6.sh`. Never touch the CEO's own
  Chrome, its profile, or system sound.
- **No play request to the CEO** until a build passes the readiness probes. The MC person play
  test is deferred by the CEO.
- Ask the CEO about any rule a player can see early, in plain game terms with an example. Give
  the file path and the full option text, not ids.
- Disk is short (about 100 to 150 GB free of 1.8 TB). Remove `target/mc-*` build caches of
  finished lanes. Keep `target/mc-evidence`.

Gate scripts (durable copy): `/home/julien/Projects/TuRTS/target/mc-evidence/gate-scripts/`. Scratch folder of this session:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/`
(`int_gates.sh <lane> <cache> <prefix>` = 17 slice gates; `mp1_gates.sh <prefix>` = 24 MP1 gates;
`mc_gates.sh <prefix>` = 16 MC gates). Copy them if `/tmp` may be cleared.

Runner: `python3 /home/julien/Projects/TuRTS/target/mc-evidence/gate-runner/exec_in_lane_v4.py
--lane <worktree> --target <cache name> --head <commit> --label <unique literal> --audio
[--allow-dirty] -- <cmd>` (scripts need `MC_JOBS` 1 to 4).

## 2. Branches and worktrees

| Branch | Worktree | Pushed head | What |
| --- | --- | --- | --- |
| `codex/mc-integration` | `/home/julien/Projects/TuRTS-mc-integration` | 9d2ab86b | MC (player control), draft PR #2 |
| `pre-mp1/integration` | `/home/julien/Projects/TuRTS-pre-integration` | afb9c6de (local merge 6e191618 of A1d, NOT pushed) | pre-MP1 slice, 12 of 21 tasks merged |
| `mp1/integration` | `/home/julien/Projects/TuRTS-mp1-integration` | 4553acc4 | MP1 lane 0, 15 of 18 tasks merged |
| `mp1-design` | `/home/julien/Projects/TuRTS-mp1` | see `git log` | design, plans, all review and ruling reports |

Documents (branch `mp1-design`):
- Slice plan: `docs/superpowers/plans/2026-11-12-pre-mp1-slice.md` (read the "Binding amendments" block).
- MP1 plan: `docs/superpowers/plans/2026-11-12-mp1-playable-skirmish.md`.
- Running list of rulings and errata for design revision 6.3: `docs/research/mp1/errata-pending-6.3.md`.
- Every review, re-check, ruling and builder report of today: `docs/research/mp1/*.md`.

Frozen values (must not change): `tests/m3/m2_operation_stream.json` sha256
`bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b` (one CEO-approved re-freeze
only, at slice task Q2). Slice whole-file trace hashes: base d4c73ebe…, haul 805309b2…, acme
19dcf9a3…, cedar 0db10e01…. `probe_neutral` on the slice base and on MP1:
`frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`; after A2b1 by design:
`e6e4417f9cbe5fa5`, `49fc63ffd1d8c57d`.

## 3. The CEO decision on touch point 1 (Decided 2026-11-12)

The CEO chose **arrive first**: the unit that REACHES a contested point first keeps it; a unit
that arrives later stops as close as it can without overlap. This replaces the "ordered first"
rule of the slice design (gate case S12). The unit id must not decide a result that a player can
see; it is a tie-break only for two arrivals in the same tick. The CEO called the old id effect
technical debt.

Full binding text: slice plan, Binding amendments, "CEO DECISION on touch point 1". Vault:
`vault/Decisions/2026-11-12 Contested move points - arrive first.md`.

What follows for you:
- **A2b1: the hold is lifted.** Merge it when its merge with A1d and its gates are green.
- **A2c: do not build yet.** First the `simulation-core-engineer` and the `technical-director`
  rework design section 9 to the new rule and state the cost; an independent reviewer gates it;
  show the CEO the changed rule text before the build.
- **Audit owed:** every place where unit-id order decides a player-visible result, with a ruling
  for each.
- The limit "above 2,048 single moves in one tick, the rest start one tick later" was explained
  to the CEO, who did not object.

## 4. Slice lanes (`pre-mp1/*`)

- **A1d** (`pre-mp1/a1d-oracle`, 28e551d9): review FAIL, ruling (ii) by the technical director,
  repair, re-check PASS, small follow-up PASS. Merged LOCALLY into `pre-mp1/integration`
  (6e191618, not pushed). Merge gates: the 17 gates are green. Acceptance exit 101: only the HD1
  native reference step is red (`crates/turts_client/tests/hd1.rs:237`, `accepted set for
  sequence 12`, expected `[43]`, got `[]`), at machine load near 15. The same step passed in
  A2b1's acceptance at load 5.9, and the A1d product code is equal to the builder's head where
  acceptance passed. Load timing is likely, NOT proven. To do: run
  `scripts/hd1-native-reference.sh` (or the whole acceptance) again at low load, 3 times. Green 3
  of 3: push. Any red: trace it as a defect before the push. The 8 `ci-browser` cases ran in the
  background (`<scratch>/int-a1d-browser.out`; expect `"failed": []`).
- **A2b1** (`pre-mp1/a2b1-fallback`, 820bfc57): two repair passes, re-check PASS, follow-up PASS.
  To do: after A1d is pushed, merge integration into it (`tests/m2/refutations.json`: keep both
  row sets, unique ids), run `test_refutations.py`, `movement_writers`, `run_constant_writers`,
  then the 17 gates. The CEO hold is lifted (section 3).
- Then, in plan order: A2b2, A2c (cost gate in the plan's amendments block), Q2 (the CEO approves
  the one re-freeze), A4, A3, REG, TDN.
- Owed on the slice: the compiler pin (`rust-toolchain.toml` `channel = "1.99.1"`, as on MC) and
  the forward port of the replica stall repair (`crates/turts_audit/src/combat.rs`
  `check_removed`).
- Open item for the technical director: a runtime guard for run constants that a writer scan
  cannot see (`errata-pending-6.3.md`, last lines).

## 5. MP1 lanes (`mp1/*`)

- **Forward merge of MC** (`mp1/forward-mc`, 37c41e70): built, builder gates green. Independent
  review WITH the headed rows was in work at handoff (report expected at
  `docs/research/mp1/review-forward-mc.md` on the `mp1-design` worktree, or in
  `<scratch>/forward-mc-review/`). After a PASS: merge into `mp1/integration`, run
  `mp1_gates.sh`, push. It must go in FIRST (it leads in `replica.rs`).
- **L0-STORE** (`mp1/l0-store`, d4c94308): review PASS, follow-up done
  (`docs/research/mp1/followup-l0-store-report.md`). To do: after the forward merge is in
  integration, merge integration into it (its three changed lines go inside `apply_chained`), run
  the gates, a short re-check of the follow-up by `bevy-rendering-engineer`, merge.
- **Sound off by default** (`mp1/sound-off-default`, ee9bd0dd at pass 3): re-check FAIL (one
  High). Pass 4 in work at handoff: F10 is reserved for the settings menu; the engine fallback
  switch shows when no mod switch is visible; the probe becomes silent. Report expected at
  `docs/research/mp1/sound-off-pass4-report.md`. Then: merge integration (after the forward
  merge), headed rows SND1, SND2, ORD6, ORD7, ALR1, HUD2, AUD1, a re-check by `mod-builder`, merge.
- **Q-COLD small follow-up** (accept the armed state; "no on-screen control at all" is a FAIL;
  doc fixes): owed, not started.
- Then: L0-EXIT freeze review, L0b-ORD, stage 1 to checkpoint C1 (first playable candidate).
  Apply design revision 6.3 and the plan revision from `errata-pending-6.3.md` before stage 1.

## 6. MC lanes (`codex/*`)

- Table run at 9d2ab86b: 33 of 34 pixel rows pass, 0 resyncs, acceptance green
  (`target/mc-evidence/reviews/mc-table-9d2ab86b.md`). Receipts on `codex/mc-table-receipts`
  (769ded22).
- **SEL4 instrument repair** (`codex/mc-sel4-gap`, 8cb49871 at pass 1): review FAIL (the frame
  hold put both presses in one frame, so the row could not fail). Pass 2 in work at handoff
  (no hold, gap 250 ms, different-frames check, PT1 converted, a native 400/401 ms test). Report:
  `target/mc-evidence/reviews/mc-sel4-gap.md`, section "Pass 2". Then a re-check by
  `web-platform-engineer`, merge into `codex/mc-integration`, run `mc_gates.sh`, push.
- Not merged, held: the play repair lane `codex/mc-fix-pt1` (9b3a7bd).
- Still owed for MC: read the hosted CI runs on 9d2ab86b; Task 24 (regression, records); the
  final report for the main merge (the CEO merges main, not you). Open harness items for
  `qa-lead` are listed in `errata-pending-6.3.md`.
- Move to rustc 1.100 is a task of its own.

## 7. Agent seats (who reviews what)

Project agents are in `.claude/agents/` (17 seats). Used today: builders by domain;
reviewers `qa-lead` (instruments), `performance-architect` (cost), `technical-director`
(rulings), `mod-builder` (public API consumer), `bevy-rendering-engineer` (client store),
`web-platform-engineer` (browser driver), `network-engineer` (forward merge).

## 8. Housekeeping

- Progress pane status file: `/home/julien/Projects/TuRTS/target/turts-progress.json`.
- Vault (work log, decisions, daily report): `vault/` in the MC integration worktree. Today's
  files: `vault/Work Log/2026-11-12.md`, `vault/Reports/2026-11-12 Daily report.md`.
- Review worktrees that can go when their lane merges: `TuRTS-pre-a1d-review`,
  `TuRTS-pre-a2b1-review`, `TuRTS-mp1-l0-store-review`, `TuRTS-mp1-forward-mc-review`,
  `TuRTS-mc-sel4-review`.

## 9. Final state at handoff (2026-11-12, 21:30 local)

The Claude session stopped here. Two background runs were still active and finish on their own.

**Still running at handoff (read their results first):**
- **The forward-merge review with headed rows** (agent, `qa-lead`). Its gates were done and the M2
  headed cases were in work (`target/mc-evidence/fwd-rev-*`). Report: `docs/research/mp1/review-forward-mc.md`
  in `/home/julien/Projects/TuRTS-mp1`, or `<scratch>/forward-mc-review/`. If no report exists,
  the agent died with the session: read the `fwd-rev-*` receipts, run only the rows that have no
  receipt, and decide the merge yourself (light path: one review, already mostly done).
- **The 8 A1d browser cases** (`<scratch>/int-a1d-browser.out`; several cases already
  `RED-RESTORED`; the last line gives `"failed": [...]`). The chain then deletes
  `TuRTS-pre-integration/target/par-browser`.

**Lane states (these replace sections 4 to 6 where they differ):**

| Lane | Branch and head | State | Your next step |
| --- | --- | --- | --- |
| Slice A1d | local merge 6e191618 in `TuRTS-pre-integration` (not pushed) | 17 gates green; acceptance red only in the HD1 native reference step, at load 15 | Run `bash scripts/hd1-native-reference.sh` (or acceptance) at low load. Green: push `pre-mp1/integration`. Red again: trace it. |
| Slice A2b1 | `pre-mp1/a2b1-fallback` 820bfc57 | re-check PASS, follow-up PASS, CEO hold lifted | Merge integration into it after A1d is pushed (`tests/m2/refutations.json`: keep both row sets), run the gates one time, merge, push. |
| Slice A2c | not started | waits for the design change to "arrive first" (section 3) | Design rework by `simulation-core-engineer` and `technical-director`, one gate, show the CEO the rule text. |
| MP1 forward merge | `mp1/forward-mc` 37c41e70 | builder gates green; review running | On PASS: merge into `mp1/integration`, `mp1_gates.sh`, push. |
| MP1 L0-STORE | `mp1/l0-store` d4c94308 | review PASS, follow-up done | After the forward merge: merge integration into it, gates, merge. No new review (light path). |
| MP1 sound off | `mp1/sound-off-default` 3b9fe6fa | pass 4 native part PASS (`docs/research/mp1/sound-off-pass4-report.md`) | After the forward merge: merge integration (conflicts in `ci.yml`, `tests/browser/feedback-driver.js`, `feedback-oracle.js`), run the headed rows SND1, SND2, ORD6, ORD7, ALR1, HUD2, AUD1 one time, merge. |
| MC SEL4 | `codex/mc-sel4-gap` 1119970d | pass 2 STOPPED on a measurement: with no frame hold the gap is 374 to 519 ms for `gap_ms` 250 | Decision of the orchestrator: take the builder's option 3 (`gap_ms` 100 for SEL4 and skip the post-eval between taps in `tapKey`), prove SEL4 two times quiet and two times under load, run the five local receipts again, merge into `codex/mc-integration` with `codex/mc-table-receipts`. Report: `target/mc-evidence/reviews/mc-sel4-gap.md`, "Pass 2". |

**What is good to know from today:**
- The table run of MC at 9d2ab86b: 33 of 34 pixel rows pass, 0 resyncs, acceptance green.
- Findings that are recorded and not yet done are in `docs/research/mp1/errata-pending-6.3.md`
  (branch `mp1-design`). Under the new working mode, close them in bulk; do not open a lane for each.
- The file `/home/julien/Projects/TuRTS/CLAUDE.md` has uncommitted changes on `main` (the new
  working mode). Do not commit to main; the CEO does that.
- The sound guard watches of this session showed 0 lines from 14:25 to the end.
