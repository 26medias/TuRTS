---
name: mc-pixel-fix4-2026-11-12
description: HUD2 lane (codex/mc-fix-hud2 from 9742b6b) timing repairs of ORD3, MM1, SEL3; SLOT1 and PER1 proposal facts; harness facts found
metadata:
  type: project
---

Verified 2026-11-12 in /home/julien/Projects/TuRTS-mc-fix-hud2 (evidence: target/mc-evidence/hud2-fix4-*).

- The "timing" cause given for MM1 was only half right. Window 2 never got a gesture, so the "Click to play"
  overlay dimmed the ping (255,204,0) to (179,143,0): the pixel check failed in every run (replay of
  hud2-run130-m-mm1 run 1). Window 2 needs F9 after io.use(2). The minimap pings never expire
  (feedback.rs keeps 64; minimap.rs draws all): only the target dummy's death removes the health record.
- Alert order in MM1: the trooper hits dummy 45 first, target 46 about 40 ticks later, 46 dies about 120
  ticks after its first alert (6 runs).
- ORD3 had two host-time races, not one: A landing 1.7 s after the first Append, and the 17 overflow
  appends (0.85-0.98 s each with the 150 ms wait) against a 15.9 s walk. Fix = more distance and no wait,
  never a weaker assert. Click cost on this host: 0.32-0.45 s, max 0.9 s (load average 13).
- Harness: io.click(x, y, button, { count: 2 }) is one action, frame-held; the DOM events in actual_events
  carry wall_time_ms and client_x/y. Product double click uses frame time (core step now_ms), so a frame hold
  makes the gap 0; the window itself is covered by crates/turts_interaction/tests/km3.rs (lines 238, 249).
- Checkpoints are read inside io.screenshot (after capture). exec_in_lane forbids edits in the lane while a
  run is active (source hash before/after); commit before the headed run, use --allow-dirty only for plants.
- RESULT: lane head 39e62d9 (pushed): ORD3, MM1, SEL3, SLOT1 each 2 of 2 green, plants red (labels hud2-fix4-*).
  ORD3 had a third race: Gather lasts 1.6 s (worker 2 ore/s), fixture now 1/s. SLOT1 repair = hold the owner
  frames (about 1.2 s) while window 3 starts (slot1-owner-hold.js), plus skip the closed page in the
  client-error check (392e2e6 broke SLOT1 for any run that got that far).
- SLOT1 needs three rendering windows (row contract: window 3 pixels). Green once: root-t16-slot1-current-full-v3
  at lane head 51170cb (before 3d0cf2f), same Chrome flags. Red 2 of 2 at 9742b6b with the GPU lock free
  (hud2-fix4-slot1-idle). Bare 3-window probes (scratchpad gpuprobe, exact 12 features/31 limits, continuous
  draws) fulfil all three devices in 0.1-0.5 s with --disable-vulkan-surface and 3.2 s without it. So the host
  can give three devices; the stall is specific to the product pages (vault C308-C320 history).
- PER1-binding-corruption-proposal.json has row id PER1 and fixture per1-invalid-binding-v1 (ruling C334).
  The plan (Task 16 step 1 and 2) names only rows/PER1.json and the settings case; no script or CI names it.
