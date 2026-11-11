

## C384: give the M2 package HUDs a plain text build list, so that the M2 browser cases can produce (2026-11-11)

Conflict: design 12.2 (design lines 1108-1112) says that base, haul, cedar and acme get a plain interface/hud.ron with text buttons and no images, and that the M2 browser cases run on these HUDs (13.3). The Task 18 HUDs hold one "Start production" Order button (mods/base/interface/hud.ron, hud_order_produce_button). RUN113 traced that an order button with a Content or Job target composes no order (crates/turts_interaction/src/lib.rs:1139 and widgets/mod.rs:1520-1522; Root read lib.rs:1125-1141, the rest is the agent's trace). So a pixels-only driver cannot start production on these packages; five base cases, the production parts of haul-economy and cedar-economy, and the plants E9-browser-missing-button and E9-browser-missing-output-mesh have no path.

Choice: option (a). Add to the HUDs of base, haul and cedar a plain text List with source BuildMenu and a plain text ProductionQueue list, in the pattern of mods/forge/interface/hud.ron (text only, no image). Keep the existing buttons. The packages' maps, units, orders and bindings do not change. This is a repair of Task 18 content to the design sentence "the M2 browser cases run on these HUDs"; it is not a change of the design, of a key, of a step or of an acceptance row, and these four packages are M2 test packages, not the default game. A package version or a pinned hash changes only if a test requires it; the frozen stream and the forge fixed point must stay green (G-FROZEN decides).

Evidence: design lines 1108-1112; mods/base/interface/hud.ron lines 3-8; mods/forge/interface/hud.ron line 46 (List source BuildMenu); plan line 676 and lines 680-689.

Alternatives not taken: (b) fixture HUD copies for the instrument only: the design names these HUDs as the place where the cases run. (c) retire the cases and two plants: a retirement is a CEO stop item and is not needed.

## C385: accept two re-target deviations from the plan's browser plant table, subject to the independent review (2026-11-11)

Conflict: plan lines 682 and 684 send E2-browser-hardcoded-label to widgets/sources.rs and E9-browser-debug-only-ui to widgets/snapshot.rs. RUN113 reports: the plain cedar HUD has no name widget, so a sources.rs plant never runs in the cedar case; snapshot.rs feeds the debug bridge, so a plant there breaks the instrument, not the product.

Choice: both plants go to crates/turts_client/src/visuals.rs (the world tooltip name; the selection ring loop), where the product path that the case judges runs. Each must keep its failure class (a label not from content; UI visible only through the debug path). The Task 21 reviewer checks the two anchors and the real reds. Open question for that reviewer: RUN113's M2 schema check admits unit_orders and max_health in public state; whether unit_orders of foreign units is permitted public state is NOT TRACED.

Evidence: RUN113 reply (plant table, anchor counts 1); plan lines 680-689 (instrument detail; brief section 4 line 49).

Global114/170 after RUN114 registration.
