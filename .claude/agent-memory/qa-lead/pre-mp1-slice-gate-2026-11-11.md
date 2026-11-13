---
name: pre-mp1-slice-gate-2026-11-11
description: Pre-MP1 slice design gate (qa-lead instrument lens) - validator-off fault runs, color-moved hides statement swaps, signed zero, browser audit oracle reads pos from the economy log
metadata:
  type: project
---

Verified 2026-11-11 on TuRTS-mp1 426165f (code = MC integration), scratch worktree with
`validate_records` returning Ok at once (economylog.rs:371).

- M3 fault set (17) cannot reach the validator: every command is `cargo test -p turts_pack`,
  and turts_pack has no turts_server/economylog edge. 17/17 RED-RESTORED with validator off.
- `cargo test -p turts_server` with validator off: 51 economylog inline tests fail (they run in
  every test binary that `#[path]`-includes economylog.rs), plus economy.rs:238, :486 and the
  live reader test. All six mc_orders "strictly replays" tests stay green;
  mc_orders.rs:469 has NO outcome assert besides the validator.
- E8 process test holds validator self-checks (economy.rs:3293-3345, `.unwrap_err()`), so the
  three E8 cases share a baseline that the validator-off build breaks.
- ci-native validator off (label slice-gate-native): 29/33 red-restored. E8 x3 baseline red at
  economy.rs:3294. E9-duplicate-spawn baseline red at economy.rs:2235 while E9-midtrace (same
  test) was green: the live reader economy.rs:1792-1870 keys on the validator text
  "command operation id N has no matching operation" (economylog.rs:1085) - a race.
- With self-checks off too (slice-gate-e8-v2): E8-foreign and E8-rejection red-restored;
  E8-mixed plant red at economy.rs:3197 "rejected seq 5 is write neutral" (independent of validator).
- `git diff --color-moved=dimmed-zebra`: a changed constant or comparison in a moved block shows
  plain +/- (good); a swap of two statements shows as moved (hidden). Item-level multiset check
  (scratch item_equal.py) catches both and passes a pure move.
- `MovementFrameOutput` derives PartialEq: 0.0 == -0.0, but movement.rs:481,964 use atan2 and
  total_cmp, so "bit-equal" must compare to_bits.
- grid `within` drops a point at exactly `reach` with a negative coordinate (qx 1.8, px -7.2000003,
  reach 9, cell 0.9); none found for coordinates in [0, 200] in 60 M tries.
- Plain group-0 move vs one-unit group-7 pooled move (orders tests fixture): position bits equal
  for 400 ticks, arrival at the same tick (263 index), pool collected at arrival.
- tests/browser/m2-audit.js:67-77 reads units[].pos, facing, gather order stage from the private
  economy log: a LedgerAudit without them breaks the browser audit oracle (E9-browser rows).

See [[mc-reg1-parallel-2026-11-11]].
- Re-check rev 3 (c93bc37): all round-1 findings have text. Still open: the call counter
  (movement.rs:13-21 cfg(test) thread_local pattern) counts calls, so a second branch inside the
  same function stays green; trace starts after A2b, so A2a/A2b have no position proof on
  base/haul/cedar (stream has no positions); G1 counter "not committed" is not evidence.
