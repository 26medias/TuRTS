# Paper play of the MC Player path (design lines 1283-1319), forge proving_ground

Facts used (all read at HEAD):
- Turret slot 0 at (17,12): mods/forge/maps/proving_ground.ron:11; range 8.0, damage 10, cooldown 1000 ms: mods/forge/units/turret.ron:11-13.
- Drone A slot 1 at (24,12): proving_ground.ron:13. Distance to turret = 7.0 < 8.0 -> in range at tick 0.
- Drone B slot 1 at (30,4): proving_ground.ron:14. Distance = sqrt(13^2+8^2) = 15.3 -> out of range.
- Drone health 100, no weapon: mods/forge/units/drone.ron.
- Press: 2 ore -> 1 shell per 2 s: mods/forge/recipes/make_shells.ron. So at most 0.5 shell/s for turret + lab.
- Hardened Shells: +10 damage, 10 s, 10 shells: mods/forge/technologies/hardened_shells.ron:13.
- slot_economy: [] -> forge has no slot balances: proving_ground.ron:19.
- Server ticks count from process start: crates/turts_server/src/stats.rs:131. No wait-for-join found (grep).
- Browser player id is random: crates/turts_client/src/lib.rs:76 (web entry sets none).
- Slot claim: a claimed slot never reopens; same id -> AlreadyClaimed; new id -> NoOpenSlot: crates/turts_sim/src/slots.rs:45-55; called at crates/turts_server/src/session.rs:103.
- Design: map unchanged (12.1:832); SelectionPrimary fields (8.4:596) have no weapon stats; Balance only (8.4:595).

Play, in order:
1-3. Load, "Click to play", guide. OK.
4. Hover drill: ring yes. Tooltip "Bore Drill": no world tooltip in 4.1:327 or 9.1:652. I must click to learn the name.
5. Lab: after 1-2 min of guide reading, the lab may already hold 10 shells (0.25-0.5 shell/s). "3 / 10 disabled" may never show.
6. Belt D / E. OK if the card shows Disable/Enable with keys. D and E are named only in the path.
7-8. Research 10 s with a bar; alert. Click turret: "damage 20" has no source. I cannot see the upgrade.
9. Turret right-click ground: no rule applies -> local notice no_applicable_order (4.3:355-356). Its text is not defined (9.3:680-682). "Cannot move" is not guaranteed.
-- meanwhile, from tick 0, the turret shoots drone A (in range). 10 shots at 10 dmg. Drone A dies in well under 2 min.
10. Window B: camera shows ONE drone, not two. FAIL.
11. Find turret: OK (small map).
12-13. "box round both drones", group of two: FAIL (one drone).
14. Shift waypoints, S: OK with one drone.
15. "Inside its range": no range sign. I guess. "Spread side by side": one unit, nothing to see.
16. Health drops; no shot visual or weapon sound in MC (1.3:64-67; 9.5:716). I do not see who shoots me.
17. Last drone dies (20 dmg, 5 shots). Slot 1 has no units. No message.
18. Rebind Stop: no unit to test it. Reload: new random id, both slots claimed -> rejection screen. FAIL.

Four-part check (M3 demo): control yes; want: slot 0 research, slot 1 nothing; obstacle: lab needs shells; win/lose: none. Decided scope (S:722, MP1 adds match flow).
