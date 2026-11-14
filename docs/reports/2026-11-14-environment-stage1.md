# Environment stage 1

Implementation: `e95e628a`. Validation and HUD fix: `cecc9a96`.
This is the first P4 slice. It does not complete P4.

The skirmish now has wider textured grass, a light background, distance haze, and a map boundary.
High and Medium use FXAA and haze. Low disables both.
The objective and resource text have dark backdrops for contrast.
The decorative ground does not enlarge the playable map.
The minimap, orders, economy, and match results retain their existing rules.

The mod API is 0.2.10. The wire protocol remains 6.
The environment data belongs to the selected game map.
Old packages without environment preserve their existing presentation defaults.
The API contract is in `docs/mod-api/environment.md`.

## Independent review

One independent review ran code against `e95e628a`.
Its report is retained under `target/mc-evidence/p4-environment/review/`.
The review found one P2 issue: an extreme texture scale could overflow the expanded UV range.
The fix rejects that combination during both content and model validation.
The same texture scale remains valid without the overflowing extension.
The retained regression and fractional-margin texture phase test pass.
No second review was requested.
The acceptance wording now explicitly requires finite derived UV ranges.
This clarifies the accepted numeric contract. It does not reduce the scope.

## Native verification

The full workspace command ran once. It returned 101: 2,324 passed, 18 failed, and 40 ignored.
All 18 failures were then resolved through targeted checks.
The initial full-workspace log is retained. It is not an all-green command.

The source checker did not recognize existing combat and visual-effect exports.
Its export classifier now recognizes them and qualifies their identifiers.
An old projection assertion also failed unchanged on the previous feature baseline.
It now checks actual pool activity and projected/unprojected values across frames.
Old API fixtures now request the current release or omit newer optional fields as appropriate.
These repairs do not change production movement code.
No new process records or gates were added.

Targeted source checks, version checks, projection checks, model/lobby fixtures, environment checks, and strict Clippy pass.
The review fix adds four runtime checks and five client checks, all passing.
The authority comparison moves and harvests for 2,000 ticks and compares the exact audited state.

## Limits and next work

Water, heightfield visuals, blended terrain, a full sky, weather, and more lights remain later P4 work.
The 2,000-unit scene is slow. Performance work follows this delivery.
No new human playtest or other-machine GPU test was performed.
Manual cold-play checks remain open. Scripted probes are reported separately.
Main is not merged or pushed.
Every test browser uses a fresh profile and `--alsa-output-device=null`.

## Acceptance evidence

All paths in this table start at `target/mc-evidence/`.

| Row | Result | Evidence |
| --- | --- | --- |
| ENV1 | PASS. Legacy defaults and API floor checks. | `p4-environment/review-fix-api.log`; `p4-environment/review-fix-client.log` |
| ENV2 | PASS. Invalid scalars, colours, opacity, and derived UV ranges are rejected. | `p4-environment/review-fix-api.log` |
| ENV3 | PASS. Actual mesh extends on four sides. Boundary dimensions stay unchanged. Outside movement and construction are refused. | `p4-environment/review-fix-client.log`; `p4-environment/final-quality/result.json` |
| ENV4 | PASS. Final hardware browser screenshot shows background, textured ground, boundary, haze, models, and team colours. | `p4-environment/final-quality/high.png`; `p4-environment/final-quality/restored.png` |
| ENV5 | PASS. Real Settings clicks cycle High, Low, Medium, and High. Native legacy camera check passes. | `p4-environment/final-quality/result.json`; `p4-environment/review-fix-client.log` |
| ENV6 | PASS. Exact audited simulation equality after 2,000 ticks. Browser minimap and map size retain 64 by 48. | `p4-environment-api-tests.log`; `p4-environment-authority-tests.log`; `p4-environment/final-quality/result.json` |
| ENV7 | PASS. All required measurement rows are recorded. Final 400-unit pan: High 42.23 FPS, Low 49.02 FPS. | `p4-environment/{400-measure,2000-measure,400-pan,final-400-pan}/result.json`; `docs/measurements/models.md` |
| ENV8 | PASS. One code-running review, fixed finding, all three scripted player probes, and actual global launcher. | `p4-environment/review/review.md`; `p4-environment/review-fix-{api,client,clippy}.log`; `p4-environment/cold-{1280,1024,buttons}/`; `p4-environment/launcher/result.json` |

The final cold wrapper returns command 0 and wrapper 0 with unchanged source.
Receipt: `target/mc-evidence/p4-environment-final-player-checks-v2/receipt.json`.
The checker reports PARTIAL-PASS because manual checks remain open.
The first wrapper invocation had a mistyped commit id. It stopped before starting a browser.
Its failed preflight log remains separate from the accepted player run.
The final environment driver verifies browser profile removal and server shutdown.
The actual computer launcher verifies gzip transfer and starts the final game.
The shared and two-player launchers point to the same prepared build.
Those two launchers were not rerun for this environment-only delivery.

Run `bash target/mc-evidence/play-skirmish.sh` to play the build.
Build Barracks, harvest timber, train an army, and destroy the enemy castle.
A good result is Victory or Defeat with a working New skirmish button.
