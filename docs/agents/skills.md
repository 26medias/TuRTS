# Third-party skills for this project

Installed on 2026-10-26 at user level (`~/.claude/skills/`, symlinks into `~/.agents/skills/`)
after seven studio seats read each SKILL.md and checked its claims against primary docs.
Install form: `npx skills add <package> -g -y`. Update all: `npx skills update`.

## Installed

| Skill | Package | Seat that voted | Caveat |
| --- | --- | --- | --- |
| bevy-core-concepts | chrisgliddon/bevy-skills@bevy-core-concepts | simulation-core | Pinned Bevy 0.19. Calls `Update` the default; the core uses `FixedUpdate` (spec 6.1). |
| bevy-ecs-systems | chrisgliddon/bevy-skills@bevy-ecs-systems | simulation-core | Pinned 0.19. |
| bevy-ecs-queries | chrisgliddon/bevy-skills@bevy-ecs-queries | simulation-core | Pinned 0.19. |
| bevy-ecs-components | chrisgliddon/bevy-skills@bevy-ecs-components | simulation-core | Pinned 0.19. |
| bevy-testing | chrisgliddon/bevy-skills@bevy-testing | simulation-core, qa-lead, bevy-rendering | Pinned 0.19. Headless `App` with `TimePlugin` only; the test instrument for 6.1. |
| bevy-rendering | chrisgliddon/bevy-skills@bevy-rendering | bevy-rendering | Pinned 0.19. Runs a stdlib-only audit script; read it before you run it. |
| bevy-wasm-webgpu | chrisgliddon/bevy-skills@bevy-wasm-webgpu | web-platform, bevy-rendering | Pinned 0.19. Build path: wasm-bindgen CLI, `wasm-release` profile. |
| bevy-input-actions | chrisgliddon/bevy-skills@bevy-input-actions | bevy-rendering | Pinned 0.19. Frame-to-fixed input bridge fits 6.2. |
| bevy-cameras | chrisgliddon/bevy-skills@bevy-cameras | bevy-rendering | Pinned 0.19. API names only: it hardcodes a camera at `Startup`; D37 makes the camera a mod setting. |
| bevy-diagnostics-profiling | chrisgliddon/bevy-skills@bevy-diagnostics-profiling | performance, bevy-rendering | Pinned 0.19. Only skill that states WebGPU gives CPU timings only. |
| bevy-platform-web | thedivergentai/game-dev-agentic-skills@bevy-platform-web | web-platform | Bevy 0.20. One wrong path: `FixedMain` is in `bevy::app`, not `bevy::time`. |
| bevy-ecs | gamedev-skills/awesome-gamedev-agent-skills@bevy-ecs | bevy-rendering (install), simulation-core (skip) | Bevy 0.20 pitfalls; use as the 0.20 correction layer over the 0.19 skills. Its frame-time motion advice applies to presentation only, never to the fixed-tick core. |
| rust-best-practices | apollographql/skills@rust-best-practices | simulation-core | The one Rust style skill. |
| rust-async-patterns | wshobson/agents@rust-async-patterns | network | Tokio patterns for transport and host. |
| m07-concurrency | actionbook/rust-skills@m07-concurrency | network | Send/Sync triage. |
| rust-testing | rewrite-rs/skills@rust-testing | qa-lead | "A new test must be observed failing before it is accepted." |
| property-based-testing | trailofbits/skills@property-based-testing | qa-lead | For lockfile and serializer round-trips. |
| rust-profiling | mohitmishra786/low-level-dev-skills@rust-profiling | performance | Native Linux only. |
| linux-perf | mohitmishra786/low-level-dev-skills@linux-perf | performance | Cache-miss counters for the 4-vCPU server (D58). |
| rust-performance | full-stack-skills/rust-skills@rust-performance | performance | p50/p95/p99 contract; no commands of its own. |

Bevy version: no `Cargo.toml` exists. crates.io stable was 0.20.0 on 2026-10-26. Ten skills pin
0.19. When the workspace pins a version, re-check the pinned skills, and read the `bevy-ecs`
pitfalls before any 0.19 snippet.

## Deferred, with the decision that unblocks each

| Package | Unblocked by |
| --- | --- |
| po4yka/rust-skills@rust-sanitizers-miri | first `unsafe` block in the tree |
| google/skills@gcloud | D59 service-account access; also decide on its telemetry prefix and web-search ban |
| nodnarbnitram/claude-code-extensions@tauri-v2 | D02 resolves to a Tauri wrapper |
| codspeedhq/codspeed@codspeed-setup-harness | decision: SaaS baseline or in-repo baseline from the 13.2 VM runs |

## Rejected, with the reason

- omer-metin/skills-for-antigravity@game-networking: recommends lockstep for RTS and tells the agent to correct users who disagree (against D18).
- thedivergentai bevy-economy-system: wallets mutated on `Update`, no reserve/commit (against 7.2).
- thedivergentai bevy-genre-rts: client writes move orders from the keyboard (against 6.2).
- chrisgliddon bevy-ui: uses `Button` + `Interaction`, deprecated in 0.20.
- mohitmishra786 wasm-wasmtime: pins wasmtime 24 (current 47), pipes an install script to bash, native only.
- mohitmishra786 rust-cross: lists the removed `wasm32-wasi` target.
- pluginagentmarketplace rust-wasm: labels a copy as zero-copy.
- mindrally tauri-development: Tauri v1 APIs, prescribes Next.js, unchecked file paths.
- affaan-m/ecc@rust-testing: invented 80 percent coverage gate.
- mohitmishra786 cargo-workflows: CI retries count flaky tests as passes.
- oakoss realtime-sync: "database is the source of truth" (wrong for an authoritative simulation).
- jeffallan cloud-architect: mandates multi-region HA for one benchmark VM.
- SQLite Database Expert, rusqlite-guide, terraform skills: Tauri or AWS scope, Python examples, not a Rust server.

## Gaps: write project-local skills when the need appears (D43)

- `mod-runtime-sandbox`: event hooks not per-tick (5.1, 5.4), capability allowlist, budgets marked open, per-match state isolation (5.5), same script on native host and browser client.
- `mod-package-manifest-lockfile`: one field list (5.3 and 5.5 disagree today), D35 lockfile rules, resolver diagnostics, one-byte hash test, client cannot upgrade past the lockfile.
- `publisher-signing`: 5.7 and D53, keys separate from player keys, signing in the trusted shell (6.13).
- `browser-wasm-measurement`: resident memory high-water mark, tick p99 via `performance.now`, DevTools memory. No registry skill covers this (spec 13).
- RTS pathfinding and flow fields; WebTransport or WebRTC from Rust; lightyear or bevy_replicon. None in the registry.
