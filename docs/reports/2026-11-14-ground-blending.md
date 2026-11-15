# Ground blending: clearings and timber paths

Status: the corrected release passes all acceptance rows, scripted player probes, and the actual launcher.

Both castles and timber stands have dirt clearings. Two soft paths connect the areas.
The game reuses its existing grass. Dirt uses a colour layer, with no new art download.
No unit statistic, map dimension, placement, resource, command, or protocol changes.

Contract: `docs/superpowers/specs/2026-11-14-ground-blending.md`.
Implementation: `cdb6b88b`. Review correction: `91f5790e`.
Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/ground-blending/`.

## Engine and mod contract

API 0.2.11 adds optional `ground.overlay`.
A map names a colour, optional image, tile size, roughness, and at most 16 soft rectangle patches.
Coordinates use map world X/Z. Overlaps use the maximum mask.
The layer blends colour, roughness, and surface normals before the existing PBR lighting.
It adds no geometry, collision, navigation, or authoritative terrain.
Absent layers retain the original material and shader.
All qualities retain the patches. Optional images use the existing texture limits.
The shader adds a bounded loop and one texture sample.
The server validates the data. Its simulation does not read the layer.
Protocol 6 remains unchanged.

## Development checks

- Client tests: 676 passes and five existing ignored tests in `client-tests.log`.
- Focused ground and render tests: 16 passes in `ground-tests.log`.
- API and runtime suites: 368 passes in `api-runtime-tests-complete.log`.
- Simulation comparisons: two passes in `simulation-tests.log`.
  The layer comparison runs 2,000 steps with real movement and harvesting. Both audited worlds match.
- Server and package suites: 544 passes, with 33 existing ignored tests, in `server-pack-tests.log`.
  This compatibility run precedes the one-line feather correction.
- Strict client, web, runtime, and API Clippy passes for all native targets in `clippy-final.log`.
  Runtime Clippy also passes after the correction in `feather-clippy.log`.
- Actual preview GPU: `pixels-import-fixed/result.json` passes the plain, legacy, and textured cases.
  Colour changes reach 117.92 summed RGB levels inside a clearing and 84.53 on a path.
  The feather edge changes by 2.65. Untouched ground changes by zero.
  The same world sample changes by only 0.16 after a camera pan.
  A 1024-pixel layer image becomes 512 on Low and returns to 1024 on High.
  All six patches stay present. Screenshots are in each case folder.

The first shader attempt fails: `patch` is reserved and WESL imports must precede declarations.
Both errors are fixed before review. Failed evidence remains in `pixels/`, `shader-diagnostics/`, and `pixels-fixed/`.
The driver now checks renderer error messages that Chrome publishes through styled console output.
Old contract fixtures omit the new presentation field before declaring an older API edition.
The shipped package still runs through direct package checks.

The first release batch passes actual pixels and environment bounds, then stops before measurement.
The benchmark needs unpacked fixture directories. The generator now writes them.
Failed setup evidence: `final/battle400/`. Its result is not a performance measurement.
A second setup fails because the field matcher also matches `background`.
The generator now matches the exact field. Both generated battle packages pass the release packer before measurement.
Second failed setup: `final/battle400-checked/`.
The passed release checks stay accepted. Only the remaining checks run again.
First batch receipt: `target/mc-evidence/ground-blending-final-checks/receipt.json`.

## Independent review and correction

One independent review runs code from `cdb6b88b` in an isolated checkout.
Its 22 native checks and full preview pixel suite pass.
It finds one small-patch contract issue. The shader widens a feather below 0.00001 world units.
Its real GPU reproduction returns a 0.5 centre mask for a patch that should reach 1.
Report and original reproduction: `review/review.md` and `review/small-patch-gpu.json`.

The contract and both validators now require a feather of at least 0.00001 world units.
This is the only spec correction. It makes the existing precision guard explicit before packages load.
Four focused checks pass, including the rejected tiny patch and accepted minimum boundary.
Evidence: `feather-fix.log`.
The actual GPU reaches a centre mask of 1 at the supported minimum.
Evidence: `minimum-mask-gpu.json` and `minimum-mask-probe.cjs`.
The shipped clearings and paths are unaffected.
No second review pass runs. The release build restarts after this correction.

## Final release acceptance

Prepared build: `target/mc-evidence/skirmish-ground-blending-prepared`.
Compiled source: `91f5790e`. Later commits repair only the fixture generator and documentation.

| Row | Result | Evidence under the evidence root |
| --- | --- | --- |
| GB1, legacy and API floor | PASS | `api-runtime-tests-complete.log`; `feather-fix.log`; `final/pixels/reference/` |
| GB2, validation and supported minimum | PASS | `feather-fix.log`; `minimum-mask-gpu.json` |
| GB3, actual plain/legacy pixels, soft edges, world alignment | PASS | `final/pixels/result.json`; `final/pixels/plain/` |
| GB4, actual textured layer and High/Low/High controls | PASS | `final/pixels/textured/`; `ground-tests.log` |
| GB5, audited match equivalence and real map bounds | PASS | `simulation-tests.log`; `final/environment/result.json` |
| GB6, 400/2000 High/Low and 400-unit pan | PASS | `final/battle400-measure/`; `final/battle2000-measure/`; `final/pan/` |
| GB7, independent review fixed, final probes and launcher | PASS | `review/review.md`; `feather-fix.log`; `final/player/`; `final/launcher/result.json` |

All three scripted player probes pass. The checker reports PARTIAL-PASS because manual checks remain open.
The remaining checks return 0 with unchanged source. Receipt: `target/mc-evidence/ground-blending-final-measurements/receipt.json`.
Full performance percentiles and memory numbers are in `docs/measurements/models.md`.

The global launcher serves 15,090,559 gzip bytes for 60,951,106 decoded WASM bytes.
Its browser receives 15,090,804 wire bytes. The decoded hash and live scene pass.
The fresh test profile is removed. All three launcher paths select this release.
Only the single-player launcher runs again for this presentation change.

## Limits and parked scope

No new human playtest, full normal-stat victory/defeat, shared human match, or other-machine measurement runs yet.
Water, heightfield geometry, sky, weather, and more lights remain later P4 work.
The layer is one bounded composition step. It does not complete P4 or add custom mod shader code.
No acceptance threshold is reduced. Main remains unchanged. No branch is pushed.
