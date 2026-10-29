# gate1-td evidence (technical-director, 2026-10-29)

Probe tree: scratchpad/mc/td/repo (rsync copy of the repo, no target, no .git, no vault).
Probe crate: td/repo/crates/turts_interaction (deps turts_mod_api, turts_net_protocol, glam =0.33.12).

1. Rows of design 2.4 added after boundary.sh:37 -> full run exit 0; new lines:
   checked: turts_interaction x86_64-unknown-linux-gnu: no bevy*
   checked: turts_interaction x86_64-unknown-linux-gnu: no turts_sim
   checked: turts_interaction wasm32-unknown-unknown: no turts_sim
2. cargo check -p turts_interaction --target wasm32-unknown-unknown -> exit 0.
3. --rule forbid --crate turts_interaction --target wasm32-unknown-unknown --names turts_mod_api
   -> "forbidden edge: turts_interaction -> turts_mod_api", exit 1 (rule resolves the real wasm32 tree).
4. T1X as written (plain copy of the member manifest + bevy_render) -> exit 1 with
   "cargo tree failed for turts_interaction: ... error inheriting `edition` ... failed to find a workspace root".
   Same exit code as a real red; only an exact-stderr assert separates them.
5. Copy with its own [workspace], explicit fields, absolute path deps, bevy_render -> 
   "forbidden edge: turts_interaction -> bevy_app", exit 1.
6. Real crate + [target.'cfg(target_family = "wasm")'.dependencies] bevy_ecs -> full run exit 0 (gap);
   --rule no-bevy-prefix --target wasm32-unknown-unknown -> "forbidden edge: turts_interaction -> bevy_ecs".
7. K13: ENGINE_API_VERSION 0.2.3, RELEASED += 0.2.3, row exports.orders since 0.2.3:
   ^0.2, ^0.2.1, ^0.2.2 refused ("admits 0.2.2"); ^0.2.3, >=0.2.3 accepted; semver ^0.2/^0.2.1/^0.2.2 match 0.2.3.
8. cargo tree -i glam@0.33.12 --workspace: bevy_math 0.20.0 -> turts_client; Cargo.lock also holds glam 0.34.1.
9. CancelProduction carries job_id: JobId(u64): messages.rs:67-70, economy.rs:13; used in 19 files
   (client scenario/economy_ui/tests, server session/tests, pack tests, wire.rs).
10. Lightyear registration: Hello first (turts_net_lightyear/src/protocol_plugin.rs:26), CommandApplied :40,
    CommandRejected :42. Version guards: acquire.rs:36 (protocol), :42 (engine equality), join.rs:275 and
    server net.rs:44 (netcode protocol_id), session.rs:97 (Hello).
