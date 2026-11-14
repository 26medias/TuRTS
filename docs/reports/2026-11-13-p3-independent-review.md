# P3 independent review

Approved for the P3 scripted acceptance scope.
The reviewed build is `856582374907ba2e7b24c2a6ef1b66362e16a888`.
The complete row map is [acceptance.json](/home/julien/Projects/TuRTS/target/mc-evidence/p3-independent-review/acceptance.json).
All relative evidence paths below use `/home/julien/Projects/TuRTS/target/mc-evidence/p3-independent-review/` as their base.
All thirteen rows have passing evidence.

The final browser batch ran on the frozen author checkout.
Its receipt is `../p3-review-verified-player/receipt.json`.
The command and wrapper returned 0. The source remained unchanged.
No compiler ran during the batch.
Every browser used a fresh profile and `--alsa-output-device=null`.

## Findings and fixes

- Common decal delay and line offset were ignored. Independent tests failed on those values. The author fixed both and retained the tests.
- Model particles ignored Add blending. Their shared material cache now includes the blend mode.
- Lines and decals ignored some common image, blend, size, and color parameters. Shared line batches now use image and blend keys. Native tests inspect real mesh colors, widths, material images, blend modes, and decal transforms.
- A delayed or transparent authored trail could hide a rejected projectile. The full-pool fallback now supplies an undelayed opaque moving line. The regression fills all 512 Low slots with critical particles.
- The first FX3 mask counted attack markers and weapon pixels. Those early PASS results are invalid. The final fixture uses a fixed-tick attack without a marker. Its hit mask measures rising wood pixels against a pre-hit baseline. The effects-disabled control has zero bolt and spark pixels.
- Small shipped particles were too faint. The author increased bolt and spark size and trail width. Particle counts stayed unchanged.
- A destroyed final castle left no death effect. The server publishes the final tick through a reliable snapshot without shot or destruction facts. A narrow client exception now shows fresh, known terminal deaths. It does not infer a shot.
- A new camera speed caused the 1024-pixel probe to fail after an edge-unit click. The author reverted that unrelated change. Both final first-click sizes pass.
- The warning covered the credits. Its final row is at y108. The final UI records show no overlap.

These fixes belong to this one review. No second review stage was started.

## Final executed evidence

I ran 28 native FX, freshness, and warning tests on the final code. All passed.
The log is `terminal-native-independent.log`.
I also ran five package tests in `pack-green.log`.
The API and runtime code are unchanged since that package run.

| Row | Evidence |
| --- | --- |
| FX1 | `terminal-native-independent.log`; `final-verified/health-hold-example.json` |
| FX2 | The same timeline evidence; `FX2-mutation-red.log`; `FX2-restored-green.log` |
| FX3 | `final-verified/pixels/result.json`; `final-verified/pixels-negative/result.json` |
| FX4 | `final-verified/pixels/result.json` |
| FX5 | `terminal-native-independent.log` |
| FX6 | `terminal-native-independent.log`; `terminal-red/result.json`; `final-verified/victory/result.json` |
| FX7 | `terminal-native-independent.log`; `FX7-mutation-red.log`; `FX7-restored-green.log` |
| FX8 | `terminal-native-independent.log` |
| FX9 | `final-verified/warnings/result.json`; `final-verified/warning-layout.json`; `FX9-mutation-red.log`; `FX9-restored-green.log` |
| FX10 | `pack-green.log` |
| FX11 | `final-verified/hash-comparison-normalized.json`; both normalized canonical state files |
| FX12 | `final-verified/cold-check.json`; all three final probe files; `final-verified/victory/result.json` |
| FX13 | `../../../docs/measurements/models.md`; `../p3-contract-final-table/result.json`; `../p3-terminal-400-guard/result.json` |

Final pixels: 153 bolt pixels and two rising spark pixels.
The effects-disabled control returned the expected FX3 failure with zero of each.
The white flash exceeds its unflashed baseline. Team color returns within 300 ms.

The measured health example schedules arrival after 196.9 ms.
Health stays at 10000 before arrival.
The next sample, after 231.0 ms, shows 9985.
The presentation limit remains 300 ms.

The final castle fight shows Victory, one scorch, and 37 live particles.
I inspected the final screenshot and saw the smoke, debris, and scorch.
The earlier missing-scorch failure remains in `terminal-red/`.

At both 1280 and 1024, three selections, six order buttons, and four extra clicks pass.
The buttons probe also passes, including its first click through the start overlay.
The checker has zero failed checks.
Its exact result is `PARTIAL-PASS`, because the old agent/reference stages were not repeated.
The earlier 1024 failure remains in `final/cold-1024/`.

## Required fault checks

All three temporary faults were built in an isolated review worktree.
Each exact test returned 101 with its named semantic failure.
Each returned 0 after restoration.

| Fault | Row | Red evidence | Restored evidence |
| --- | --- | --- | --- |
| Health changes before arrival | FX2 | `FX2-mutation-red.log` | `FX2-restored-green.log` |
| Pool permits one extra slot | FX7 | `FX7-mutation-red.log` | `FX7-restored-green.log` |
| Warning permits damage in view | FX9 | `FX9-mutation-red.log` | `FX9-restored-green.log` |

The faults are recorded in `required-faults.patch` and are absent from delivered code.
Machine summaries are `mutation-red.json` and `mutation-restored.json`.

## State and performance

High and Low produce the same normalized server state at tick240.
Random claim IDs become `claimed`; unclaimed slots stay null.
All gameplay fields remain in the comparison.
Claim presence is `[true,false]` in both runs.
The SHA256 is `63fb54be2b15212d1bd521d37ab5e621600ad66202571d610af77cfc59287ef1`.

The full eight-row table records renderer engine `f6b686e1`.
The final terminal-only fix has a separate guard on engine `85658237`.
It reaches 41.43 FPS at High and 54.71 FPS at Low with 400 soldiers shooting.
The benchmark fixtures disable victory. The renderer is unchanged by the terminal fix.
High800 remains below 30 FPS in the full table. The required 400-unit guard passes.

## Contract corrections and limits

- A bounded decal exception resolves the spec's 60-second scorch and general 10-second limit. The client caps decals at 128.
- J is the declared jump key because Space already centers the selection.
- Spec section4.4 now describes the terminal snapshot exception. It requires the same session, a coherent prior state within four ticks, and a first Victory or Draw result. It excludes pending deaths, ordinary recovery, stale gaps, late joins, and duplicates. A lone Defeat is not a terminal match.
- The wire has no damage-reduction fact. `hit_reduced` remains reserved until that mechanic exists. No reduction is inferred from health differences.
- No minimap exists in the skirmish HUD. The conditional minimap requirement does not apply.
- Optional effect sounds are absent. Physical sound output was not tested.
- Optional glTF model particles were compiled and inspected. Their live browser rendering was not exercised. Custom image and blend transitions have native render-system tests.
- The victory fixture uses a 30HP enemy castle. It is a fast fight probe, not a complete normal-length skirmish.
- This review does not certify a new unaided 300-second player session or the old reference runs. All requested scripted player probes pass.

No remaining blocker was found for the defined P3 acceptance scope.
