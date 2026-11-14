# Shared skin palettes

Status: final release checks and scripted player probes pass. Launcher verification follows.

The client shares bone matrices between units that already share an evaluated pose.
Each unit keeps its own transform, clock, facing, animation cadence, material, and named anchors.
The server, packer, and package are byte-identical to the delivered rig-batching build.
Public API 0.2.10 and protocol 6 stay unchanged.

Implementation: `66e0d0d3`. Retained review regression: `9e5f0f5f`.
Contract: `docs/superpowers/specs/2026-11-14-shared-skin-palettes.md`.
Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/shared-skin-palettes/`.

## Engine change

The palette removes the source instance anchor from each joint matrix.
The vertex shader applies the destination instance anchor.
This retains the same weighted world position and inverse-transpose normal.
The forward and prepass paths retain Bevy 0.20 PBR outputs, shadows, colours, UVs, and tangents.
Private mesh copies rename the joint attributes so Bevy does not extract a full skin for each draw.
Source meshes, source materials, and inverse-bind assets stay unchanged.
CPU animation poses and the whole named hierarchy stay in place.
This feature does not remove CPU bone copies or lower animation quality.

The GPU buffer is bounded by 65,536 matrices and the device limit.
Invalid or missing data, overflow, unsupported layouts, morphs, and active motion-vector prepasses use the existing renderer.
The reference path is a private test URL option. It adds no player HUD control.
GPU readiness uses the actual mesh and material asset type after a backend change.
The flash flag owns bit 0. Palette addressing uses the other tag bits.

## Executed development checks

- `client-tests.log`: 675 client checks pass, with five existing ignored tests.
- `retained-review-test.log`: nine focused checks pass after retaining the independent test.
- `clippy-fixed.log`: strict client and web checks pass for all native targets.
- `prepass-fixed-pixels/result.json`: real combat passes; eight draws use two palettes and 82 matrices.
- `reference-pixels/result.json`: the existing renderer passes with eight ordinary draws and no shared draws.
- `budget-fallback-pixels/result.json`: a one-matrix test budget safely uses eight ordinary draws.
- `quality/result.json`: quality switching, map bounds, and debug/scene coherence pass.
- `walking-registered/result.json`: real walking pixels pass the unchanged 10 percent threshold.
- `walking-registered-frozen/result.json`: the frozen clip is correctly rejected.
- `unit-death-checked/result.json`: a Death_A body is visible and expires.
  Neutral armour pixels drop by 683. Shared draws drop from eight to five.
  This visual fixture uses 15 health. It is not a normal-stat match result.

The 2,000-unit preview reaches 29.65 FPS High and 37.18 FPS Low.
All 2,000 actors stay visible and take damage.
All 5,000 skin draws use 33 shared palettes and 1,353 matrices.
Evidence: `preview-2000/result.json`.
These are preview numbers. Final release results follow below.

## Independent review

One independent review runs code from `66e0d0d3` in an isolated checkout.
It finds no actionable production issue.
The independent library result is 382 passes and one ignored test.
The focused result is eight passes, including one independent regression.
The GPU probe runs 18 cases through the production skin and normal math.
Maximum position error is 0.000003815 map units.
Maximum normal and tangent error is 0.000000120.
Report: `review/review.md`. Raw logs and the independent patch are in the same folder.
The independent regression is retained in the repository.
No second review pass runs.

The GPU JSON reports NVIDIA, Ampere, and no software fallback.
The browser hides the adapter description.
Host commands identify one display adapter: NVIDIA GeForce RTX 3080, driver 580.178.04.
Evidence: `hardware.json` and `review/gpu-math/result.json`.

## Failed development checks and repairs

The first draw attempt fails GPU validation.
Prepass joint locations are 5 and 6. Forward locations are 6 and 7.
The corrected pipeline uses the proper layout for each pass.
The following real combat check passes.

The old walking probe also fails with the reference renderer.
It clicks with a stale camera matrix and compares images with stale unit positions.
The unit stops after 2.87 units instead of the intended longer walk.
A fresh camera fixes the click. Fresh scene capture and translation registration fix the comparison.
The pixel threshold stays unchanged. The positive and frozen-control results above pass.
Earlier failures remain in the walking folders.

The first unit-death command has a mistyped probe mode and runs the wrong case.
The driver now rejects unknown modes before it opens a browser.
The accepted unit-death result is `unit-death-checked/`.
Clippy first rejects a redundant map lookup. The cache now uses the entry API.

## Release validation

Prepared build: `target/mc-evidence/skirmish-shared-skins-prepared`.
Sealed source: `9e5f0f5f`, hash `e1d147a041c3...`.
Server, packer, and package byte identity: `native-package-reuse.json`.
The final web tests pass all five checks. Strict client and web Clippy passes for all native targets.

| Acceptance | Result | Evidence under `final/` |
| --- | --- | --- |
| Native equivalence, attributes, tags, fallback, and readiness | PASS | `../client-tests.log`; `../retained-review-test.log`; `../final-clippy.log` |
| Production GPU math | PASS, all 18 cases | `gpu-math/result.json` |
| Combat and actual shared draws | PASS | `pixels/result.json` |
| Palette budget fallback | PASS, ordinary draws | `fallback-pixels/result.json` |
| Rendered walking | PASS, unchanged pixel threshold | `walking/result.json` |
| Frozen animation control | EXPECTED_RED | `frozen-control/result.json` |
| Rendered death and body expiry | PASS | `death/result.json` |
| Real quality controls, bounds, and coherence | PASS | `quality/result.json` |
| 400 and 2,000 units, High and Low | PASS, all visible and damaged | `battle400/result.json`; `battle2000/result.json` |
| Real 400-unit pan | PASS | `pan/result.json` |
| Same-binary reference comparison | PASS | `reference-2000/result.json` |
| One independent code-running review | PASS, no actionable finding | `../review/review.md` |
| Scripted first-click probes at both sizes | PASS | `player/cold-1280/`; `player/cold-1024/` |
| Scripted buttons probe | PASS | `player/cold-buttons/` |

The complete final batch command and wrapper return 0. Source stays unchanged.
Receipt: `target/mc-evidence/shared-skins-final-checks/receipt.json`.
The checker reports PARTIAL-PASS because its manual cold-player checks remain open.

At 400 units, High reaches 59.30 FPS and Low reaches 59.20 FPS.
The pan reaches 59.60 FPS on both settings.
At 2,000 units, High reaches 31.45 FPS and Low reaches 39.14 FPS.
The same release with the reference renderer reaches 27.21 FPS High.
That paired High result improves by about 16 percent.
High p95 remains 50 ms. Some stress frames remain slow.
Complete frame percentiles, memory, and protocol details are in `docs/measurements/models.md`.

## Limits

No new human playtest, full normal-stat win/loss match, shared human match, or other-machine measurement runs.
The gameplay package and server stay unchanged.
Motion-vector rendering uses the ordinary skin path.
Shared palettes do not provide previous-frame bone data.
Draw-call counts and GPU memory remain unavailable.
The original static detailed-model acceptance is not rerun for this skin-only change.
No spec item or acceptance threshold is reduced.
Main is unchanged. No branch is pushed.
