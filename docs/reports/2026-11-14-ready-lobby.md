# Ready lobby delivery

Status: LOB1–LOB8 pass. Implementation: `e6319f1d`. Main is unchanged.

Human games now wait for both players. Each player chooses a colour and presses Ready.
Both see a three-second countdown. Cancel Ready or leave to stop it.
The computer game still starts immediately.

| Row | Result | Evidence under `target/mc-evidence/ready-lobby/` |
| --- | --- | --- |
| LOB1 | PASS. One human waits. Audited gameplay state stays unchanged for 20 seconds. Typed gameplay commands get MatchNotStarted. | `native-fixes.log`; `public-complete/waiting.png` |
| LOB2 | PASS. Two distinct seats. Each first Ready click acts. Both see the countdown. | `local-complete/result.json`; `public-complete/result.json`; `public-complete/countdown.png` |
| LOB3 | PASS. Cancel and disconnect prevent start. An unclaimed seat accepts a replacement. | `native-fixes.log`; `public-complete/cancelled.png`; `client-lobby-final.log` |
| LOB4 | PASS. Stale revisions, duplicate identity, occupied and invalid colours are refused. The real-time action budget exhausts at 64 and refills at 32 per second. | `native-fixes.log`; `review/server-suite.log` |
| LOB5 | PASS. Each seat is claimed once. Both clients get one match ID and colours [1,0]. Body materials, ring materials and minimap colours agree. All eight lobby choices have the correct rendered colour. | `native-fixes.log`; `client-overlay-final.log`; `public-complete/result.json`; `public-complete/palette-0.png`; `public-complete/palette-1.png` |
| LOB6 | PASS. Both ordinary moves work. Each reload keeps its slot, colour, army, and admitted move. | `public-complete/result.json`; `public-complete/services/server.log` |
| LOB7 | PASS. Immediate computer mode works. AllSeated with zero countdown works. API floors and protocol 6 checks pass. | `native-fixes.log`; `cold-complete-1280/`; `cold-complete-1024/`; `cold-complete-buttons/` |
| LOB8 | PASS. Normal local match ends with Victory and Defeat at the same tick. Public HTTPS/WSS and final scripted cold probes pass. Owned processes and fresh profiles stop. | `local-complete/result.json`; `public-complete/result.json`; `cold-complete-1280/`; `cold-complete-1024/`; `cold-complete-buttons/` |

The local winner harvests 180 credits, builds Barracks, and trains two Swordsmen and three Crossbowmen.
Both browsers end at tick 2421. The surviving castle keeps 1,000 health.
The driver activates each test window before input. Both windows report document focus before Ready.
No stats, map placements, or wire commands are injected.

The independent review ran code and found two issues.
Ready cancellation could be lost while an older Ready action was in flight.
Malformed palette strings could render as white. Both findings are fixed and have retained regressions.
The review report is `2026-11-14-ready-lobby-independent-review.md`.

Browser checks also found three UI problems.
Lobby reception required a refusal receiver that Lightyear creates only after a refusal arrives.
The generic button style replaced the palette colours.
The game focus overlay darkened the lobby before its first gesture.
All three are fixed. The final public check measures all eight colour buttons from rendered pixels.

The final client passes 354 native tests, with one existing ignored test.
The server, protocol, and mod runtime pass 566 tests, with 32 existing ignored tests.
Strict Clippy passes for the changed packages.
The independent server review passes 113 checks. The retained findings now pass on the author branch.

The final lobby client runs the 400-unit mixed battle at 46.42 FPS High and 56.91 FPS Low.
Evidence: `target/mc-evidence/mixed-opponent/400-guard/result.json`.
That build uses the byte-identical final client and package. Only its native opponent policy differs.
The benchmark disables the computer player. Both timing windows keep all 400 units visible and taking damage.
No compiler or other test browser runs during the samples. No new 2,000-unit row was run.

Final cold wrapper: `target/mc-evidence/ready-lobby-complete-player-checks/receipt.json`.
Command and wrapper exit 0. Source stays unchanged.
The scripted probes pass. The checker reports PARTIAL-PASS because manual checks remain open.
No new human playtest, another-computer test, or full manual readiness claim is made.
Every test browser uses a fresh profile and --alsa-output-device=null.
The user's Chrome and system sound are untouched.

API 0.2.9 supports the new package start policies. Protocol 6 requires the lobby handshake.
Palette values must be distinct #RRGGBB values in the first interface. Host overrides use the same validator.
No acceptance item was dropped. Lobby bots, shared rematch, surrender, and disconnect defeat remain later work.

Cache cleanup removes 14 inactive, obsolete test executables and recovers 5.76 GiB.
The latest test executables and all evidence remain. See `cache-cleanup.json`.
