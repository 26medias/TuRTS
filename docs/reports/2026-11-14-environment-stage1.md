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
