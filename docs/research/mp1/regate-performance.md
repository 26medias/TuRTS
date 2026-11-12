# MP1 design revision 4 re-gate: performance

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=0
(Agent contract line: VERDICT=pass-with-changes FINDINGS=7 HIGH=0 RUNS=6 NOT_CHECKED=4)

Artefact: `docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md` rev 4 (head 5158f96). Tick rate assumed: 20 Hz (Open, D18). Native probes ran on the dev host (load 4.9). Logs and probe sources are in `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-perf/`.

## Status of my gate-1 findings
- H1 debug state: CLOSED. The fix is in design:1823-1826, 1886-1893 and row DBG1:2045, which has a build counter and today's tree as its red build. Nit: `publish_debug` copies the debug state in every frame (`turts_client_web/src/lib.rs:370-371`). Make it run only when the state changes.
- H2 UnitId index: CLOSED. The fix is in design:217 and 1855-1873, with DES1 plant (5).
- H3 acquisition: CLOSED. The fix is in design:551-564, with the TGT1 counter and plants (2) and (4).
- H4 A3 entry condition: CLOSED. The fix is in X0e:218, 1240-1246 and 1925.
- M1 wire: PARTIAL. ORDW1 catches a `unit_orders` regression. No row catches the "sent on change" rule for weapon, site, job or supply records (N2).
- M2 replica: PARTIAL. The open point is acceptable. Probe: one replica clone costs 0.029 ms at 400 units and 0.363 ms at 5,000. `replace_keyed` costs 0.247 ms per delta at 5,000 units with 500 changes. Point 12 (design:2163-2167) closes at gate 2 with recorded numbers. That is enough for MP1. But the `health_trace` prune carries load, and it has no number and no row (N4).
- M3 charge: CLOSED. The fix is D-A (a), with u128 and coalesced events, and PAY1 plants (3) and (5). Under the free income, a starved job writes about 1 ledger event per income period, not 1 per job end. That is still bounded.
- M4 and M5: CLOSED in the text (design:1875-1884). Neither has an instrument.
- L1 pool: CLOSED (8.3:1799-1800, FX1 plant b).
- L2 floor: CLOSED as D-N. The instrument is weak (N6).
- L3 memory: PARTIAL. Open point 13 closes in stage 2, before MDL1. That is acceptable. But the D-M caps do not bound decoded memory (N3).

## Cost table (U units, A armed, S sites, J jobs, P slots)
| Mechanism | Work per tick | Bound | MP1 / benchmark |
|---|---|---|---|
| Acquisition and retaliation | grid O(U) + (A_idle,ready / K) x 25 cells; attackers O(hits) | D-J, TGT1 counter | ok / ok |
| Sites, held orders | O(S), with S ≤ P x `open_sites_per_slot`; placement O(U) only on events | mod data (6.1:382-394) | ok / ok |
| Power | sum O(U_supply) + O(P x R) (6.2:480) | linear | ok / ok. Event counts, as KeyUnitsLost uses (972-973), are cheaper |
| Income | O(U_income); 1 event per period | U7 | ok / ok |
| Production queues | O(J) u128 ops; index replaces the `production.rs:282` scan | H2 | ok / ok |
| Victory | O(P), counts kept on events | 972-973 | ok / ok |
| Start order | O(placements), once | - | ok / ok |
| Export | O(P x U) visible set + binary-search `owner_of` | linear per viewer; time recorded (exit 5) | ok / recorded |
| Client hints | H x C x O(U) per frame ("each client update", 7.8:1677) | linear | ok / debt, not recorded |
| Client alerts | OwnUnitDamaged is O(U²) per frame (`feedback.rs:623-631`) | **open** | 0.05 ms / 5.0 ms native (2 rules) |
| Client HUD | 3 replica clones per frame + every bound text re-measured | grows with `health_trace` | see N1 |

## Wire (model, postcard varints; `model.log`)
Record sizes: Fired 6 B, Destroyed 3 B, WeaponRecord 12 B, SiteProgress 16 B, Supply 20 B with a string id or 6 B with an index.

At 1,000 units in combat, weapon records fall from 5,000 B/tick (sent every tick) to 600 B/tick (sent on change), and effects cost 302 B/tick. Together that is about 9 % of the M1 payload of 9,858 B/tick (`m1.md:136`).

At a 2x200 MP1 fight: weapon records 90 B/tick, effects 45 B/tick.

Exit item 5 records bytes split by record kind, but it has no red. Only ORDW1 can go red, and only for `unit_orders`.

## New findings
N1 Medium. The HUD does per-frame work that the design does not name.
- `sync_widget_tree` runs in every frame with no gate (`widgets/mod.rs:79-90`).
- It clones the replica, content and interface (`:272-283`), and so do `cursor.rs:32-40` and `feedback.rs:393`.
- It builds a JSON key of the whole interface (`:311-319`).
- It iterates all entities 3 times (`:148`, `:298`, `:801`).
- It writes Text, Node and Visibility with no comparison (`:831`, `:857`, `:916`). Bevy then re-measures every text in every frame (`bevy_ui-0.20.0/src/widget/text.rs:325`).
- There is no full-HUD rebuild per tick: the rebuild key is interface, selection, window and assets (`:311-318`, `:356`). But each selection change rebuilds the whole tree (`:360-397`).
- Repair text for section 9: "The widget, cursor and alert systems read the replica by reference and write a component only when its value changes (`set_if_neq`). They run on a new delta, selection or input, not in each frame. VIS3 records the frame time with the HUD on and off." The cost in ms is NOT TRACED.

N2 Medium. No row tests "sent on change" for WeaponRecord, SiteProgress, job and SupplyRecord (6.8:1172-1176). Repair: add to ORDW1: "with 200 idle armed units and one site at a constant rate for 100 ticks, the weapon, site and job bytes of each delta after the first are 0; plant: WeaponRecord in each tick."

N3 Medium. D-M caps file bytes, not decoded pixels.
- Worst case: 9 models x 2 images x 2048² decodes to 384 MiB, while the GLB bytes stay under 16 MiB. The advice set decodes to 16.0 MiB.
- The existing cap of 256 MiB is per package (`content_mc_validation.rs:16`, `:272`). Design 8.1:1774 does not say that GLB images count in it.
- Triangles: 400 units at the 8,000 cap give 3.2 M triangles. The advice mix gives 0.80 M.
- Repair for 8.1: "Model images count in `MAX_DECODED_IMAGE_BYTES`; all model images decode to at most 32 MiB RGBA8 (an upper bound that 8.5 can only lower)."

N4 Low. The `health_trace` prune (design:1894) has no window and no row. It carries load: with 400k entries, one clone costs 1.40 ms, and 3 clones per frame cost 4.2 ms. `OwnUnitDestroyed` reads the last entry of a removed unit (`feedback.rs:643-647`). Repair: "keep the last 64 entries for each unit; a row checks the entry count after 10 min at 400 units in combat; REG2 and ALR2 stay green."

N5 Low. OwnUnitDamaged is quadratic (0.025 ms at 400 units, 2.513 ms at 5,000, per rule), and the design does not record this debt. Repair: "index the previous healths by unit id; evaluate hints and alerts once for each applied delta, plus timers."

N6 Low. The VIS3 plant stalls every frame, so it cannot show that the p95 clause works alone. Add a plant `frame_stall_ms=60` on 1 frame in 10 (p50 stays below 33 ms, p95 goes above 50 ms). Also name the configuration of the floor: models on, debug state off.

N7 Low. The grid cell is the smallest weapon range (6.3:560). For `frontier` (cell 5, acquire 9) a query visits 25 cells. A mod with weapon range 0.5 and acquire 16 makes a query visit 4,225 cells. Repair: "cell = max(2.0, smallest acquire.range / 2)."

## Question 5: are D-M and D-N defensible?
- D-N: at 1,001 units with primitives, the frame time was 16.7/16.8 ms on an RTX 3080 (`m1.md:203-205`). At 5,000 units it was 133.3/199.9 ms (`mc.md:11-12`). On SwiftShader it was 633 ms (`m1.md:200`). So 33/50 ms at 400 units can be met on a discrete GPU. It is not known on the play-test machine, which the design does not name.
- VIS3 can fail (stall plant). MDL1 can fail on each cap plant, but it cannot catch a memory or frame regression.

## What I ran
| Command | Claim tested | Refute if | Observed |
|---|---|---|---|
| `cargo test --release --locked -j 6 --test clone` (exit 0) | point 12 / design:1895: the clone per delta is tolerable | ≥1 ms at 400 units, no trace | 0.029 ms; 1.401 ms with 400k entries |
| `--test alert` (exit 0) | 7.9 alerts and point 12 `replace_keyed` | alert cost grows linearly | quadratic; `replace_keyed` 0.247 ms at 5,000 |
| `python3 -I model.py` (exit 0) | 6.8:1172 on-change saving; 8.1 D-M bounds memory | on-change is not cheaper; decode ≤ 16 MiB | 5,000→600 B/tick; 384 MiB |
| grep `widgets/`, `feedback.rs`, `bevy_ui text.rs` (exit 0) | per-frame HUD path | gated or compared before write | neither |
| grep design rows for record bytes (exit 0) | M1 instrument | a row asserts it | none, only ORDW1 |
| grep `content_mc_validation.rs` (exit 0) | 8.1 image caps | no decoded cap | a 256 MiB per-package cap exists |

## Not checked
- The browser cost of N1 (NOT TRACED; no browser run).
- Lightyear framing.
- Protocol 5 sizes: the model only, no real messages.
- The size of the `ClientContent` clone.

## What I changed
None in the repository (`git status` is clean; `target/` is ignored). I made a copy of the build cache at `target/mc-mp1-regate-perf` (it was seeded from the gate-1 cache), and I wrote the probe and model files in the scratchpad.

## Open questions for the orchestrator
- Does N1 belong in lane 0a, next to DBG1, or is it a measured-only item for VIS3?

