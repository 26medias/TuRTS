# Selected-unit information

The Skirmish HUD now shows a selected unit's portrait, name, and health.
It uses the existing mod interface contract. No Rust, server, wire, or game rule changes.
All five game unit types have distinct static portraits from their existing CC0 models.
The source art and slot colours on world models stay unchanged.

Implementation: f88cb70e. Browser driver: 44d1160c.
Evidence root: /home/julien/Projects/TuRTS/target/mc-evidence/selection-panel/.
The native server, packer, and complete browser distribution match the previous accepted build byte for byte.
Evidence: binary-reuse.json. Package validation passes; only existing model footprint warnings remain.
The final package adds five 256×256 PNG files.

## Executed checks

browser/result.json passes all five selections at 1280×720 and 1024×768.
Each portrait has distinct non-flat screenshot pixels. All names and full health values match.
The enemy castle's health text follows actual combat damage.
An empty selection hides the panel. Multiple selection retains its primary information.
All panel child bounds stay inside both viewports.
Ten per-unit screenshots and enemy-damaged.png record the results.
Every test browser has a fresh profile and null ALSA output. The driver removes its profile.

All three scripted probes pass: `final/cold-1280/probe-first-click.json`, `final/cold-1024/probe-first-click.json`, and `final/cold-buttons/probe-buttons.json`.
The batch receipt exits 0 with unchanged source: `../selection-panel-final-checks/receipt.json`.
The checker reports PARTIAL-PASS; the full fresh-agent and manual readiness procedure was not rerun.
The actual global launcher passes: `launcher-checked/result.json`.
The first launcher attempt refuses a stale play checkout (exit 66). Updating the detached checkout fixes the source/package mismatch.
Shared-human and two-player paths use this package but are not rerun.
The prepared build is `target/mc-evidence/skirmish-selection-panel-prepared`.
Run `target/mc-evidence/play-skirmish.sh`. Select a unit. Its portrait, name, and health should appear above the build list.
No new full normal-stat win/loss match, shared-human match, other-machine test, or performance measurement runs for this content-only change.
The static portrait colour is blue. It does not track the owner's colour.
No independent foundation review is needed: this feature uses the existing interface contract without changing it.
Main is unchanged. No push.
