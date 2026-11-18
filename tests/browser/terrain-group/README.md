# M4b public group fixtures

Status: source preparation only. No typed archive, simulation, browser or performance acceptance has run.
Merge accepted M4 before final M4b acceptance. Keep the accepted launcher unchanged.

`scenes.json` freezes the finite physical inputs before timing. Every main map is64×48 with unit cells, quarter-unit height steps and a four-step ridge atX[30,34). GM1 has the Y[20,28) opening. GM2 starts atY[23,25). Both use exactly24 radius0.4 workers and12 radius1.1 vehicles at the approved grid coordinates. Speeds remain3.5 and2.5; both profiles use max_step0 and max_grade0/1.

Generate one isolated public package:

```sh
python3 tests/browser/terrain-group/generate.py --root . --package quarry --scene GM1-wide --output FRESH_OUTPUT
```

Repeat for `terrain_stockyard`. Add `--reverse-allocation` or `--reverse-members` for the independent reversals; combine them for the fourth variant. Physical labels/coordinates and actual intent remain fixed. Opaque unit handles are regenerated from the authored placement order. The browser driver binds actual public handles by content, owner and their original f32 physical pose. It never chooses a winner by handle order.

The generator copies the existing public package. It changes public map/unit/profile/tool records only. Detailed worker/vehicle meshes, skeletons, clips, ground textures and credits stay byte-identical. Model scale follows the explicit new public footprint; quality/LOD/shadow settings do not change. Both material catalogs use exact per-prism resource rates for the frozen quarter-cube cells. Private seeds and realized columns have no fixture field. The ordinary runtime supplies a private seed outside the served package.

Each output contains the package, `fixture.json` with complete public file hashes, and the finite `commands.json` native replay inputs. Native commands are for authoritative/replay tests only. They cannot substitute for the GM8 real-pointer browser input.

## Frozen scenes

| Scene | Intent and expected result |
| --- | --- |
| GM1-wide | One ordinary move order for36 actors to(50,24). All complete within2400 ticks without overlap or cliff penetration. |
| GM2-narrow | Same move. Small24 complete by2400 ticks. Large12 keep their original move and wait safely. At2401, one small worker receives ordinary excavation overX[30,34),Y[20,28), target0. No large move is reissued. All large actors must finish within2400 ticks after the opening. |
| GM3-opposing-large | Nine actors per owner, six small/three large, move at tick0 in opposite directions. The three-unit corridor cannot fit two diameter2.2 bodies abreast. Both cohorts must finish. Stable blocked is a failure. |
| GM3-settled-blocker | A radius1.4 vehicle completes a move to its initial center before traffic starts at tick10. Each remaining side gap is0.1, smaller than small diameter0.8. Ridge reaches both map edges. Later arrivals must not move it. Stable safe waiting is required. |
| GM3-movable-blocker | A small obstruction leaves the passage through an ordinary point move at tick200. Both cohorts must then finish. |
| GM3-shared-and-unique | Two same-viewer groups share one destination. At tick40 one rear small actor receives a unique ordinary destination. Others retain the common endpoint. |
| GM4-edit-during-approach | Exactly36 actors:34 in the main move, one editor, one distant route. FillX[31,32),Y[20,28) to4 at tick20. Excavate it to0 at tick240. The editor has exactly32 finite fill units. No debug mutation or private resource pool exists. |

GM3 passing bays areX[24,30) andX[34,40), bothY[20,28). Their authored bodies leave the radius1.1 transit stripY[22.4,24.6] empty. A concrete feasible witness is written in the JSON: right cohort waits at its bay poses, the left cohort leaves its parking columns and crosses, then the right cohort crosses. The actual opposing commands are simultaneous. The controller must choose the sequential crossing; the witness is not a test-script Hold workaround.

GM4's distant radius1.1 route runs from(4,46) to(22,2). Its authored direct swept X bounds are[2.9,23.1]. Conservative native f32 bounds[2.8999999,23.1000001] remain disjoint from the edit. Edited chunk X bounds are[24,32). It stays active across the close/reopen window. Exact unchanged dependency revisions/memberships and stale-result rejection need native instrumentation; the current wire does not expose those counters. Browser evidence must capture the unchanged public order and continued physical motion instead of inventing private metadata.

Additional frozen scenes cover a legal step/grade1/4, max_step0 refusal, grade249/1000 refusal, exact radius1 tangency in a two-unit opening, next f32 radius `0x3f800001` refusal, a diagonal cliff-corner detour and a speed100 attempted one-cell wall sweep. Legal cases must finish. Illegal committed segments are never allowed.

## Prepared browser campaign (AUTHORED_UNRUN)

`run.js` uses a source-matching PREPARED bundle. It runs the existing `terrain-prepared.py` sealing checks against the manifest's actual package identity, verifies each committed public fixture, generates the frozen scenario, checks its file hashes, and invokes the prepared typed packer. It builds nothing. It starts only owned case servers, an owned Xvfb, and fresh silent Chrome profiles. Every Chrome uses `--alsa-output-device=null`. All contexts, profiles, servers, and Xvfb are cleaned up after success or failure.

Run only after root announces the final candidate and browser window:

```sh
node tests/browser/terrain-group/run.js PREPARED FRESH_OUTPUT
```

The default campaign covers GM1-GM4 in both public packages. Opposing-owner scenes use two real peers and both connection orders. Profiles are created first; reversed navigation reverses which peer connects first. Pages bind commands to their observed receiving slots. Optional `--package`, `--scene`, `--connections normal|reversed|both`, `--reverse-allocation`, and `--reverse-members` select a recorded variant. A narrowed run proves only its selected cases. Frozen actor coordinates, footprints, assets, commands, and native time bounds are unchanged.

Selection uses actual projected bodies and Shift clicks. Movement uses ordinary right clicks. Work chooses the explicitly bound public tool, captures a permitted reference elevation, and drags the full ordinary rectangle. It reads only the existing public debug/scene/UI bridge. It cannot inject an order, teleport a unit, edit terrain or read private geology. It logs intended and actual completed-tick input times. The driver requires a normal server admission receipt for every selected actor. It checks observed complete circles, no overlap, full known plateau support, retained settled points, and final public order outcomes. GM2 checks small completion and the original waiting large orders before issuing only the editor's ordinary work command. It gives the editor no teleport or extra movement order. The large actors must finish within2400 ticks after the actual full opening. GM3 distinguishes feasible completion from stable impossible waiting behind the immutable settled blocker. GM4 requires an observed full closure and reopening plus the unchanged active distant public order through tick340.

Each case retains screenshots, public observations, intended command offsets, actual admission ticks/sequence numbers, generated archive hashes, results, and cleanup evidence. Initial cohorts are selected before the action clock. Opposing peer inputs run concurrently. Later UI selection takes real time, so actual acceptance ticks are recorded separately from native script ticks. Browser polling can skip server ticks. It does not claim every committed sweep, private route lease equality, internal order identity, or exact same-seed trajectories. Those remain native checks.

High detail and the authored camera clamp remain enabled. This functional runner makes no FPS claim. Use the unchanged `tests/browser/terrain-fps.js`400-model instrument separately with the final prepared build and frozen terrain FPS fixture. No browser, Cargo, package generator, typed packer, GPU or performance run has occurred for this source checkpoint. Only lightweight JavaScript syntax checks have run.

Pending: typed archive/seed-backed staging for both packages and reversals; native full swept/arrival/determinism/replay/retention/privacy assertions; final browser GM1-GM4 outcome assertions and connection-order reversal; unchanged M4 native workload and400 detailed model FPS at all three camera distances; final cold-play probes. Nothing in this source checkpoint is a product or capacity PASS.

The fast-wall authoring value was corrected before timing from200 to the public maximum100. At20Hz this still attempts5 world units fromX28 through the one-cell wallX[30,31). This correction preserves the swept-body case. It does not bypass package validation or change the public speed limit. Generated fixture hashes must be regenerated from the corrected source before acceptance.


## Native real-tick harness (source checkpoint)

`crates/turts_server/tests/terrain_group.rs` runs the generator, verifies every public file hash, packs and checks the archive, and boots it through the ordinary seed-backed server path. It resolves each envelope through the actual server catalog and drains the ordinary command queue during real fixed ticks. It never writes actor positions, terrain columns, order components, or navigation state.

After the route audit API is integrated and the shared native lease is available, run:

```sh
TURTS_TERRAIN_GROUP_EVIDENCE=/home/julien/Projects/TuRTS/target/mc-evidence/terrain-group-movement/native CARGO_BUILD_JOBS=4 ~/.codex/bin/turts-cargo test -p turts_server --test terrain_group --locked -- --test-threads=1
```

The harness covers both packages, all four allocation/member variants for the main scenes, all seven geometry scenes, full-circle overlap during each committed segment, exact terrain sweeps, settled-point retention, original large orders after widening, ordinary fill/excavation, and the distant route lease through tick340. Physical comparisons preserve every position bit and actual movement endpoint. The independent circle oracle includes a crossing with clear endpoints, exact tangency, and a one-ULP overlap.

GM5 covers public Append, Replace, Stop,32 repeated cancellation cycles, same-seed physical replay, retained guidance bounds, and a fresh match. Death, owner change, forged movement audit rejection, and GM6 paired hidden worlds are NOT covered by this target. Root owns their simulation/audit additions. Peer connection order requires the later real network/browser checks. No test here claims those results.

Per-tick checks include field/capacity limits, pending membership consistency, expansions, edges, admission, invalidation, retirement pages, neighbor visits, and zero stale activations. These use the route owner's agreed on-demand privileged audit API. It must be integrated before compilation. No private diagnostic row enters a player projection.

Each run retains the frozen fixture and submitted-command hashes, archive hash, final positions, trajectory hash, exact submitted commands, and checked result. Generated public package copies are removed on fixture teardown to preserve disk space. The partial native harness is AUTHORED_UNRUN at this checkpoint. Only `rustfmt --check` and source whitespace checks ran; no package generator, Cargo, simulation, browser, or performance check ran during the M4 quiet window.
