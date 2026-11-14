# Rigid attachment batching

The client joins compatible rigid gear with the unit body skin.
It keeps the authored motion and every animation node.
The engine also avoids unchanged root and rig rotation writes.

## Contract and implementation

Source: `ed8b563e`, with rotation guards from `207e7af4`.
The short contract is `docs/superpowers/specs/2026-11-14-rig-batching.md`.
The engine changes private mesh copies. It leaves source assets unchanged.
A glTF node selects its skin. Unbound joint attributes do not select one.

A static attachment below one joint uses that joint with weight 1.
Its baked transform cancels the existing inverse-bind transform.
Positions, normals, tangents, UVs, triangle order, and palette indices retain their meaning.
Unsupported geometry, animated attachment paths, reflections, and unsafe scale curves retain separate draws.
All hierarchy walks have a bound. Transparent geometry retains separate sort positions.

The rig-only candidate has a byte-identical server, packer, and shipped package.
The final package also includes the separate hit-origin fix described below.
Proof: `target/mc-evidence/stable-model-transforms/native-package-reuse.json`.
API 0.2.10 and protocol 6 stay unchanged.

## Executed checks

Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/stable-model-transforms/`.

- `rigid-final-native.log`: 666 client tests pass; five existing tests are ignored.
- `rigid-contract-tests.log`: 28 animation checks pass.
  They include shipped joint hierarchies, several joint poses, and malformed parent cycles.
  Synthetic vertex payloads test the hierarchy independently of the supplied vertex buffers.
- `rigid-contract-clippy.log`: strict client and web checks pass for all native targets.
- `rigid-contract-pixels/result.json`: rendered bolt, sparks, white hit flash, and team-colour return pass.
- `rigid-contract-quality/result.json`: real quality controls, bounds, and scene coherence pass.
  These two browser rows use the fast preview. Final release rows follow below.

The initial compile of the last robustness checks failed on two local type errors.
The code now uses the actual mesh attribute return type and a named test fixture lifetime.
The accepted native logs above record the corrected source.

## Independent review

One independent review runs the source in an isolated checkout.
It finds no actionable production issue.
Its 31 animation tests include three independent checks of unsafe scale keys, palette mismatch, and source asset immutability.
Report: `target/mc-evidence/stable-model-transforms/review/review.md`.
The test patch and raw logs are in the same directory.
No second review pass runs.

## Measurement method

Use the same RTX 3080, Chrome, 1280 by 720 viewport, fixtures, and quality settings as the previous delivery.
Each row has a five-second warm-up and a ten-second presentation sample.
Scene observations occur every 500 ms. The driver records that interval explicitly.
All 400 or 2,000 actors must remain visible and take damage.
The pan rows send real arrow-key input and check the changed camera matrix.
FPS measures browser presentation intervals. It is not an isolated GPU throughput measure.
Wasm linear memory gives a high-water mark. Draw-call counts and GPU memory are unavailable.
The `draw_meshes` counter counts entities. It is not a draw-call counter.

The five original static model cases passed in the previous delivery.
They do not exercise this rig-only optimization and are not rerun here.

## Rig-only release measurements

The 400-unit mixed fight reaches 59.30 FPS High and 59.60 FPS Low.
The 400-unit pan reaches 59.70 FPS High and 59.50 FPS Low.
The 2,000-unit fight reaches 27.26 FPS High and 31.95 FPS Low.
All actors stay visible and take damage in each row.
The previous release reaches 21.49 FPS High and 28.05 FPS Low under the same protocol.
High at 2,000 remains below 30 FPS. This stress limit remains open.
The 400-unit High threshold passes.
Full frame percentiles, memory, and evidence paths are in `docs/measurements/models.md`.

The old wrapper first refused an untracked report before any browser ran.
Its existing `--allow-dirty` option permits the report. The compiled source stays sealed and unchanged.
This did not require a product change or another review.

## Visible hit origin

The rig-only release pixel probe finds one spark pixel and fails the unchanged two-pixel threshold.
The preview had passed with two pixels. This did not establish a reliable rendered hit.
The effect starts at half the model height, inside the soldier armour.
Its 250 ms lifetime ends before a vertical spark can clear the helmet.
The content now raises the spark origin by 0.65 map units.
Particle size, count, lifetime, colour, and animation cadence stay unchanged.
The same pixel probe then finds 42 spark pixels and passes.
Evidence: `visible-hit-origin/result.json`.

Two attempted capture-driver changes did not resolve the failure and were removed.
A WebGPU canvas-to-2D copy returned transparent pixels. A compositor crop still failed the original spark mask.
Their failed records remain in `final-pixels-crop/`, `crop-diagnostic/`, and `final-pixels-compositor/`.
The final driver is byte-identical to the original driver. No threshold is reduced.

The content fix is commit `68f9d456`. It does not change the engine source.
The final prepared folder reuses the exact client compiled at `ed8b563e`.
Both sealed manifests have source hash `9f7370c15551...`.
Proof of client byte identity: `visible-hit-client-reuse.json`.
The server and packer stay unchanged. The package hash changes for the authored spark origin.
The final fixtures copy that content change before the measurements and probes.

## Final acceptance

Final content source: `68f9d456`. Compiled engine source: `ed8b563e`.
Prepared build: `target/mc-evidence/skirmish-rig-batches-visible-prepared`.
Evidence below is relative to `target/mc-evidence/stable-model-transforms/visible-hit-final/`.

| Acceptance | Result | Evidence |
| --- | --- | --- |
| Geometry, hierarchy, skin binding, and fallback | PASS | `../rigid-final-native.log`; `../rigid-contract-tests.log` |
| Independent code-running review | PASS, no actionable finding | `../review/review.md`; `../review/final-animation.log` |
| Real bolt, sparks, white flash, and team-colour return | PASS, 154 bolt pixels and 42 spark pixels | `final-pixels/result.json` |
| 400 and 2,000 visible, damaged actors on High and Low | PASS | `final-battle400/result.json`; `final-battle2000/result.json` |
| 400-unit High threshold and real camera pan | PASS | `final-pan/result.json` |
| Real quality controls, map bounds, and scene coherence | PASS | `final-quality/result.json` |
| Scripted first-click probes at both viewport sizes | PASS | `player/cold-1280/`; `player/cold-1024/` |
| Scripted buttons probe | PASS | `player/cold-buttons/` |

The final batch command and wrapper return 0. Source stays unchanged.
Receipt: `target/mc-evidence/rig-visible-final-checks/receipt.json`.
The checker reports PARTIAL-PASS because manual cold-player checks remain open.
The earlier `rig-batching-final-checks-v2` batch returns 1 at the spark pixel check.
Its measured rig-only rows remain recorded above.

The final 400-unit fight reaches 59.50 FPS High and 59.60 FPS Low.
The final pan reaches 59.60 FPS High and 59.50 FPS Low.
The final 2,000-unit fight reaches 25.12 FPS High and 31.85 FPS Low.
High at 2,000 units remains below 30 FPS. This stress limit stays open.

## Limits and spec clarification

No new human playtest or other-machine GPU measurement runs.
No new full normal-stat Victory/Defeat match, public-host match, or two-player match runs for this client feature.
The server binary and gameplay data stay unchanged.
The only content difference raises the presentation-only spark origin.
Draw calls and GPU memory are unavailable.
The original static detailed-model acceptance already passes; it is not rerun for this rig-only change.

The spec now states that batching itself leaves package files unchanged.
The separate hit-origin correction changes the final package because the real rendered hit was unreliable.
No geometry, motion, animation cadence, API contract, protocol, or acceptance threshold is reduced.
Main is unchanged. No branch is pushed.

## Delivered launcher

The actual global computer launcher passes after the final cold probes.
Evidence: `visible-hit-final/launcher/result.json`.
The fresh silent profile is removed.
WASM is 60,332,465 decoded bytes. The gzip body is 14,984,643 bytes.
The measured network transfer is 14,984,888 bytes.
All three global launchers now select `skirmish-rig-batches-visible-prepared`.
The shared and two-player launchers were not rerun for this client and effect-origin change.

Run `bash target/mc-evidence/play-skirmish.sh`.
Build Barracks, harvest timber, train an army, and reach Victory or Defeat.

The remaining 2,000-unit High limit is the next performance investigation.
No new gameplay or presentation milestone is declared complete by this optimization.
