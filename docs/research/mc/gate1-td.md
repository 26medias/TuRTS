VERDICT=needs-decision FINDINGS=8 HIGH=2 RUNS=14 NOT_CHECKED=5

D = the MC design. Evidence: scratchpad/mc/gate1-td-evidence.md.

Findings

1. D:396, D:786. High. `OrderTarget` and `TargetKind` have no job variant, but `CancelProduction` needs `JobId` (messages.rs:67-70). The forge `cancel` order (D:834) and HUD2 (D:640) cannot be sent. ORD1 tests targets, not behaviours, so it cannot go red. Fix: add `Job(JobId)`. Make ORD1 round-trip every `Behaviour`.
2. D:819-821, K3. High, needs decision. K3(a) puts orders, smart rules, bindings and an interface in `turts_mod_api`. Decided 4.3 (S:171) puts default gameplay "in the default mod or an optional reusable mod module". The fallback is outside the lockfile (D35, invariant 10), and its id namespace has no name (D:450). D:820 contradicts D:821. Options: (b) migrate; (c) a library package that the server locks; (a) plus a 4.3 exception from the CEO.
3. D:238. Medium. No wasm32 `nobevy` row, but T1X (D:950) claims one. A wasm-only `bevy_ecs` passed the full run. Fix: add `nobevy turts_interaction wasm32-unknown-unknown ""`.
4. D:242. Medium. With a plain copy, T1X exits 1 on "cargo tree failed", which is the same code as a real red. T1X stays green when someone deletes the rows. Fix: give the copy a `[workspace]` and assert the exact stderr. Add the new lines to boundary.rs:92-101 and a present-crate red (boundary.rs:105-122).
5. D:370-376, K2. Medium. The behaviours hard-code targeting policy ("A moving unit does not fire", attack-move acquisition). In 4.3 (S:179, S:185) this is mod policy. Fix: the engine gives the mechanism, and `OrderDef` gives the policy as parameters. A new `Behaviour` is a minor bump with a content-level version row (version_table.rs:6-10 works at manifest level only).
6. D:367, D:441, D:80. Medium. These choices are not in the calls table: queue depth 16 (D39), the rate limit 32/8,192, and the K4 merge "in one tick". A group that arrives in two ticks splits the 6.3 spreading. `glam` has no pin (Cargo.lock: 0.33.12 and 0.34.1). Fix: add the calls, a merge key, and `glam = "=0.33.12"`.
7. D:966-993. Medium (plan). No dependencies, no critical path, no shared-file map: messages.rs (tasks 2, 7), schema.rs (3, 4, 8), economy_ui.rs (11, 12, 16, 19). Repairs are 15 %, but the cited M3 overrun is 60 %: 97 x 1.6 = 155, above 150.
8. D:107, D:394. Low. D18 is "Decided / implementation open" (S:47). Use `ContentRef` (economy.rs:16). Forbid `wgpu` and `winit` too (D74 "no renderer dependency").

What I ran (command | claim | refute | observed)
- 2.4 rows, full boundary.sh | D:238 | exit 1 | green
- wasm32 check of the crate | D:192 | error | exit 0
- forbid turts_mod_api, real crate, wasm32 | rows go red | exit 0 | red
- T1X plain copy | D:242 | edge line | "cargo tree failed"
- T1X own-workspace copy | D:242 | exit 0 | "bevy_app"
- wasm-only bevy_ecs | D:950 | full run red | green
- K13 row probe | D:815 | ^0.2.2 accepted | refused
- semver match 0.2.3 | D:813 | no match | match
- cargo tree -i glam | D:197 | one version | two
- grep CancelProduction | D:396 | unused | 19 files
- grep protocol_plugin | D:447 | Hello moves | index 0
- grep version guards | K14 | none | acquire.rs:36,42
- spec rows D15-D74 | D:104-120 | mismatch | D18
- vault grep | D:963 | other counts | 75, 26 match; "47" NOT TRACED

Not checked: sections 3, 4.1-4.4, 6-10, 13, 14; lightyear message size; avoidance determinism; F-G1; K4 wire size.

What I changed: none.

Open questions
1. Builder of `turts_interaction`: `technical-director` builds the skeleton, the contract types, T1 and T1X (2 runs). `bevy-rendering-engineer` builds the logic and the adapter. `ux-designer` tests it headless (ux-designer.md:108).
2. K3: (a), (b) or (c)?
3. A cheaper split: merge tasks 2 and 7, and cut items 1, 3 and 6. Saves about 14 runs.
