# Large-army rendering performance

Source: `a608d9e0` on `codex/large-army-performance`.
Main remains `b323057d`. Main is not merged or pushed.

The renderer now joins compatible skin parts once per model asset.
It keeps every vertex, triangle, UV, joint weight, animation node, and material group.
Only opaque or masked static identity siblings with the same parent and skin can join.
Animated mesh nodes, transparent parts, morph targets, and incompatible attributes stay separate.
Triangle strips also stay separate. Joining strips would create extra triangles.
The merged assets are shared by all instances.

Model damage flashes now use a per-instance flag.
They keep stable materials and the existing PBR path.
The flag adds a short white emission. Team colour returns when the flash ends.
The normal material and its texture maps remain unchanged.
Shape fallbacks retain their existing flash path.
The implementation uses a built-in shader. It adds no public shader API.

The browser builds scene snapshots when a test requests them.
A debug promise refreshes the scene in the same served frame, including requests delayed by the existing cap.
The browser also avoids serializing and reparsing the base snapshot before adding its fields.
Model updates index unit positions, weapon targets, and map nodes once per frame.
They preserve the first entry if a test fixture has duplicate keys.

The server, packer, and shipped package are byte-identical to the tested environment build.
See `target/mc-evidence/large-army-performance/native-package-reuse.json`.
No authority, movement, wire protocol, mod API, game stats, placements, or art files change.

## Diagnosis

The original release profile spent about 65 percent of sampled CPU time in rendering.
Shadow specialization took about 21 percent.
Damage flashes repeatedly changed materials across many mesh parts.
Disabling flashes was a diagnostic control. It is not a shipped change.
The same diagnostic workload rose from about 4 to 13 FPS with flashes disabled.

The old release stripped function names.
The retained symbolic module has exactly the same executable sections as that release.
Only its custom sections differ. This verifies the function index mapping used by the profile.
Evidence: `symbolization-proof.json`, `function-map.txt`, and `profile-symbolized.txt` under the performance evidence folder.

The first two optimizations reduced the render portion to about 22 percent in the next preview profile.
Scene snapshot capture then took about 14 percent.
That result led to the capture change.
Raw profiles and the summary script remain under `target/mc-evidence/large-army-performance/`.

## Verification before the release build

All 365 client tests pass. One existing test remains ignored.
All five browser-bridge native tests pass. Strict Clippy passes for both crates and all targets.
Ten existing JavaScript bridge/oracle checks pass.

The mesh tests use the shipped hierarchies and animation channels with isolated vertex payloads.
A separate mesh test checks joint indices, weights, UVs, and triangle order.
Animated nodes, transformed nodes, transparent materials, morph targets, and incompatible layouts are covered.
A native flash test checks the flag, stable material handle, Settings switch, and expiry.

The real browser checks bolt pixels, spark pixels, white flash pixels, and the return to team colour.
Its accepted preview result is `instance-flash-pixels-fixed/result.json`.
The first run found the wrong exposure weight in the new shader. That defect is fixed.
The first native flash assertion also had a wrong negation. The corrected assertion and final suite pass.
Those failed runs remain separate. They are not accepted results.

The final preview environment check passes.
It covers quality switching, map bounds, outside construction, coherent movement snapshots, and idle scene capture.
It checks all units over 20 consecutive default-capped debug requests during movement.
It then checks that an idle scene remains cached and refreshes on the next debug request.
Evidence: `preview-quality-coherence/result.json`.

## Measurement protocol

The preview comparisons use opt-level 2 without whole-program LTO.
They are diagnostic results. The delivery uses the full release profile.
All 2,000 units stay visible and take damage in these runs.
Flashes remain enabled in every candidate comparison.

| Preview change | High FPS | Low FPS | Scene reads |
| --- | ---: | ---: | --- |
| Previous renderer | 4.07 | not measured here | every animation frame |
| Join compatible skin parts | 12.16 | not measured here | every animation frame |
| Add stable instance flashes | 16.09 | 19.80 | every animation frame |
| Capture scenes on request | 19.67 | 28.51 | every 500 ms |

The last row uses a changed measurement protocol.
The old driver parsed the scene every frame but retained state only every 500 ms.
The new default reads it only when it retains that state.
Each result records `scene_sampling_interval_ms`.
`TURTS_EFFECT_SCENE_POLL=frame` retains the old protocol for comparison.
Do not compare the last row as if the protocol stayed unchanged.
The final indexed model lookup change is not included in these preview rows.

## Final release evidence

The 400-unit mixed battle reaches 59.40 FPS High and 59.50 FPS Low.
The camera-pan check reaches 59.70 FPS High and 59.40 FPS Low.
Every actor remains visible and takes damage.
The paired 2,000-unit High comparison improves from 4.24 to 21.49 FPS with the same 500 ms scene-read protocol.
Low reaches 28.05 FPS. Both stress rows remain below 30 FPS.
The detailed 400-unit model and LOD cases each exceed 58 FPS.
The detailed 2,000-unit LOD case reaches 58.90 FPS.
Complete measurements and their limits are in `docs/measurements/models.md`.

All three final scripted cold probes pass.
The accepted player receipt is `target/mc-evidence/large-army-performance-player-checks/receipt.json`.
Command and wrapper return 0. Source remains unchanged.
The checker reports PARTIAL-PASS because manual checks remain open.
The earlier measurement wrapper returns 1 after a false idle-baseline failure.
Its measurement rows all passed. They were not rerun after the driver-only fix.
The test now takes a fresh debug/scene baseline after the screenshot.
The accepted idle check advances only 0.012 seconds while it stays unread for 1.2 seconds.
It then refreshes with the next debug request.
The failed test record remains in `final-quality-coherence/`.
The accepted record is `final-quality-coherence-fixed/result.json`.

The final rendered effect check passes: bolt pixels, spark pixels, white flash, and team-colour return.
Evidence: `target/mc-evidence/large-army-performance/final-pixels/result.json`.
The actual global computer launcher also passes.
Evidence: `target/mc-evidence/large-army-performance/launcher/result.json`.
It transfers 14,974,676 gzip bytes for 60,309,891 decoded WASM bytes.
Its fresh silent profile is removed.
All three global launchers now point to this prepared build.
The shared and two-player launchers were not rerun for this client-only change.

## Limits and continuation

No new human playtest or other-machine GPU measurement was performed.
No new full normal-stat win/loss game was run. The server and content are byte-identical to the verified game.
The PBR shader follows the pinned Bevy 0.20.0 source. A Bevy upgrade must also update that source.
No public mod API or wire contract changes. No spec acceptance item is reduced.
The scene-read measurement protocol changes are explicit above.
Main remains unchanged. No branch is pushed.
Further large-army work will examine transform updates that do not change a model's rotation.

Run `bash target/mc-evidence/play-skirmish.sh`.
Build Barracks, harvest timber, train an army, and reach Victory or Defeat.
New skirmish must start another match.
