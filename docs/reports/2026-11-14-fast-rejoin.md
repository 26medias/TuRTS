# Fast page reload delivery

Status: RJ1–RJ5 pass. Independent review passes with no findings.
Implementation: `e0de418b`. Main is unchanged.

A closed transport no longer blocks its player's rejoin until the ten-second timeout.
The permanent slot, units, accepted orders, and admission bucket remain intact.
A real second live window remains rejected. Disconnected traffic cannot claim or issue orders.

| Row | Result | Evidence under `target/mc-evidence/fast-rejoin/` |
| --- | --- | --- |
| RJ1 | PASS. Transport closure retires Session. Same identity immediately recovers the same slot. | `green.log`; `review/independent.log` |
| RJ2 | PASS. Another live window is rejected. Other players and admitted unit orders remain. | `green.log`; `review/independent.log` |
| RJ3 | PASS. The admission bucket stays exhausted. Only the closed connection's unadmitted entries are dropped. | `green.log` |
| RJ4 | PASS. Both real public reloads use one WebSocket. No duplicate-window refusal occurs. Both admitted moves survive. | `public/result.json`; `public/services/server.log`; `public/player-0.png`; `public/player-1.png` |
| RJ5 | PASS. Both first-click probes and the buttons probe pass. | `cold-1280/`; `cold-1024/`; `cold-buttons/` |

The previous public run needed eight connection attempts for each reload.
The fixed run needs one. Reload takes 1,748 ms and 2,471 ms in this run.
These times include page startup. They are not an other-hardware performance claim.
Both clients issue ordinary orders after rejoining.
The server, both tunnels, and both fresh profiles are gone after the test.
All test browsers use null sound. User Chrome and system sound are untouched.

The named closed-session regression fails before the fix and passes after it.
Evidence: `red.log` and `green.log`.
All 107 server tests and strict Clippy pass.
The independent reviewer adds two meaningful tests and passes the complete 109-test suite.
Its report and test patch are retained under `review/`. No second review loop was started.

An old audit negative also fails on unchanged baseline `13d4663c`.
It labels automatic work as Other, which the validator can permit.
The corrected negative uses Command without a command record. Audit product behavior is unchanged.
Evidence: `audit-baseline.log`, `server-tests-final.log`, and the independent report.

Cold wrapper: `target/mc-evidence/fast-rejoin-cold-checks/receipt.json`.
Command and wrapper exit 0. Source stays unchanged.
The tested client, packer, and package are unchanged from the shared-host build. Only the native server is rebuilt.
No new 400-unit FPS test, another-computer test, or two-person playtest was done.
Next: a ready lobby for a fair two-player start.

The updated global computer launcher passes its real startup check.
Evidence: `target/mc-evidence/fast-rejoin/launcher/result.json`.
It loads models, the minimap, gzip, and sound off. Its profile is removed.
Artifact comparison confirms that only `bin/turts_server` changes.
