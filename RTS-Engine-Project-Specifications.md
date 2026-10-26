# Mod-First RTS Engine — Project Specifications

Version: 0.7 • Date: October 26, 2026 • Status: architectural baseline with research-backed implementation direction

## 1. Purpose and interpretation

This document records the decisions and design direction established throughout the project discussion. It describes a Rust + Bevy RTS engine targeting browser WebAssembly first and desktop executables second, with multiplayer, comprehensive modding, production and logistics, editable terrain, buried resources, and optional underground utility networks.

The objective is to build original games whose general capabilities can span traditional RTS mechanics and industrial automation. StarCraft, Warcraft, Command & Conquer, Age of Empires, Factorio, Minecraft, and Cities: Skylines were used as functional references—not as sources of assets, characters, maps, branding, or proprietary content to reproduce. The mod-first approach is an architectural inspiration, not a claim that this project will reproduce any reference engine internally.

Status terms used throughout:

- **Decided:** explicitly selected or accepted direction from the discussion.
- **Baseline:** concrete architecture recommended in the discussion and retained as the working design; refinements are permitted during prototyping.
- **Open:** no final choice has been made.
- **Proposed:** additional implementation or validation detail included to make the specification actionable; not a previously approved commitment.

Examples illustrate what mods could do. They do not require every game to enable every system. This is an architectural specification, not a complete production schedule or a finished low-level API design.

## 2. Decision register

| ID | Decision | Status |
| --- | --- | --- |
| D01 | Use Rust + Bevy. | Decided |
| D02 | Browser WASM online multiplayer is primary; desktop executable delivery is secondary, with native versus web-wrapper implementation open. | Decided |
| D03 | Multiplayer is a core requirement; design for native/browser interoperability. | Decided |
| D04 | The official game is a default mod on top of the engine. | Decided |
| D05 | Mods define game content and rules through the same public facilities as the default game. | Decided |
| D06 | Support a broad range of RTS mechanics, not arbitrary game genres. | Baseline |
| D07 | Include research, factories with inputs and outputs, belts, and inserters. | Decided |
| D08 | Allow terrain excavation, filling, leveling, trenches, and earthworks. | Decided |
| D09 | Use a heightfield with one surface elevation per horizontal location. | Decided |
| D10 | No playable tunnels, caves, overhangs, or vertically stacked ground surfaces. | Decided |
| D11 | Store buried deposits separately from the visible surface; digging may yield resources. | Decided |
| D12 | Enable mod-defined prospecting, detectors, scanners, and research related to discovery. | Decided |
| D13 | Support underground infrastructure as networks, not traversable underground spaces. | Decided |
| D14 | Prefer simple, localized, low-computation simulation where practical. | Decided |
| D15 | Keep heavy simulation in Rust and expose a versioned mod API independent of Bevy internals. | Baseline |
| D16 | Start underground infrastructure with one logical layer and explicit connections. | Baseline |
| D17 | Store placed infrastructure at fixed elevation; later terrain edits change burial depth. | Baseline |
| D18 | Use authoritative server simulation. Networking library, transport, scripting language, terrain appearance, and numeric budgets remain open. | Decided / implementation open |
| D19 | Provide a default server index with a user-changeable URL and a self-hostable index interface. | Decided |
| D20 | Servers advertise their game mod/version and provide download URLs for automatic client setup. | Decided |
| D21 | One server supports multiple independent rooms; players can create rooms and configure mod-provided rules. | Decided |
| D22 | Room creators can set player count, a short message, and an optional joining password. | Decided |
| D23 | Support creator-initiated start and automatic start when slots are filled, subject to mod policy; starting with empty slots is allowed when the mod permits it. | Decided |
| D24 | Mods decide whether and when late admission into unclaimed slots is allowed. | Decided |
| D25 | A claimed player slot belongs to that player for the entire match, including after departure; it cannot be reassigned. | Decided |
| D26 | Support open, blocked, closed, and claimed slot states; mods determine gameplay eligibility. | Decided |
| D27 | Slots represent participation and ownership, independently of spawning and starting locations. | Decided |
| D28 | Do not impose spawn radii, deadlines, resource requirements, or disconnect gameplay through the engine. | Decided |
| D29 | Target large maps, many players, tens of thousands of units, and high command rates from AI participants. Exact workload budgets require measurement. | Decided |
| D30 | Support non-player spectators, with an audience potentially numbering thousands. | Decided |
| D31 | Use separate spectator distribution infrastructure to keep viewer-dependent work away from simulation; exact deployment remains to be validated. | Baseline |
| D32 | Reuse open-source networking foundations; prefer permissive licensing suitable for future commercial distribution. | Decided |
| D33 | Libraries are publishable, reusable packages; mods and libraries may depend on other libraries. | Decided |
| D34 | Use shared sandboxed scripting plus declarative content for downloadable packages; Rust executes heavy engine operations. Language remains open. | Decided |
| D35 | Resolve dependencies into exact version/hash lockfiles; one library version per game configuration initially, with side-by-side cache versions. | Decided |
| D36 | Seeded map-generation algorithms belong in optional libraries, not mandatory engine gameplay. | Decided |
| D37 | Use 3D assets and rendering; mods control camera, lighting, and presentation. Classic RTS view is expected, not enforced. | Decided |
| D38 | Support very large maps and multiple maps per match, including client loading/switching. | Decided |
| D39 | Avoid arbitrary gameplay caps; practical hardware and operator budgets remain necessary. Economical small/medium hosting is a requirement. | Decided direction |
| D40 | Our server starts with SQLite; alternative servers may choose their own internals while respecting protocols. | Decided |
| D41 | Saves live on servers; no standalone solo mode or local game saves. Replays and version migration are secondary. | Decided |
| D42 | Initially restore games using their exact mod versions rather than migrating to newer mods. | Decided |
| D43 | Development is agent-first; agents build and validate authoring/debug tools as concrete needs emerge. | Decided |
| D44 | First game: one-faction C&C-style test mod with three infantry types, one resource/harvester, tank, turret, command center, power station. | Decided |
| D45 | Mods can support conventional CPU-controlled opponents without GPU/model inference; external models may connect as players. | Decided |
| D46 | Reuse established RTS designs and suitable components, validating performance rather than inventing subsystems unnecessarily. | Decided |
| D47 | Conventional opponents use modular controllers, authored opening plans, reactive priorities, squad management, and small tactical state machines. | Decided |
| D48 | AI libraries provide reusable decision machinery; mods provide roles, game knowledge, priorities, and replaceable behaviors. | Decided |
| D49 | Coordinate AI spending and unit assignments through shared priorities/reservations; recovery can interrupt an opening plan. | Decided |
| D50 | Use portable cryptographic player identities with locally held private keys and no central account dependency. | Decided |
| D51 | Recovery initially relies on the user's copied/saved private key; no central recovery service. Offer backup/import and optional encrypted export. | Decided |
| D52 | Allow an identity on multiple servers, but only one controlling session per claimed slot, with explicit session replacement. | Decided |
| D53 | Provide a default package registry, independent registries, and direct package URLs; publisher keys are separate from player keys. | Decided |
| D54 | Desktop browsers with keyboard/mouse first; mobile/tablet support later; executable secondary. | Decided |
| D55 | Adopt the initial mod gameplay rules in Section 16.2. They are not engine-wide policies. | Decided |
| D56 | Adopt room administration defaults in Section 6.6, preserving permanent slot ownership. | Decided |
| D57 | Initial hosting benchmarks: 4 players/1,000 units/1 map; 8 players/5,000 units/2 maps; tens-of-thousands-unit stress scenarios. | Decided benchmark scope |
| D58 | Test small/medium matches on an initial 4-vCPU, 8-GB-RAM server allocation; measure performance and cost, not promise capacity. | Decided benchmark budget |
| D59 | User plans to grant a GCP service account so the implementation agent can provision and monitor VM tests. | Decided plan; access pending |
| D60 | Separate authoritative simulation, replicated state, and visual representations; detailed models/skeletons are not server simulation objects. | Baseline |
| D61 | Compile and validate mod assets before distribution; share resources and support rendering/animation detail levels. | Baseline; formats and tooling to validate |
| D62 | Evaluate instanced baked skeletal animation for mass infantry, rigid-part animation for vehicles, and individual animation where needed. | Baseline; implementation selected by measurement |
| D63 | Combine reusable high-level navigation with bounded local steering and localized terrain invalidation. | Baseline; navigation components remain candidates |
| D64 | Expand benchmarks to isolate rendering, animation, movement, replication, streaming, and their combined worst cases. | Baseline validation plan; thresholds open |

The earlier suggestion of chunked voxel terrain is superseded by D09–D10. Underground utility placement does not reopen the voxel or underground-navigation scope.

## 3. Product goals and boundaries

### 3.1 Goals

The engine must allow mods to build games with different combinations of combat, resource collection, construction, manufacturing, logistics, research, terrain engineering, and utility infrastructure. A traditional RTS should be possible without a factory economy; an industrial RTS should be possible without inventing a separate engine.

The default game must exercise the public mod API. If a mechanic requires a privileged shortcut available only to the default game, either the engine needs a reusable capability or the mechanic must be implemented through the supported mod interface.

The browser WASM client is the primary product and must drive compatibility, loading, memory, and rendering validation from the start. Desktop delivery is secondary. A wrapper such as Tauri is a possibility, not a selected implementation or a guarantee of equivalent webview capability. A native Bevy client remains another option. Desktop browsers with keyboard and mouse are the initial target. Phones/tablets and mobile controls are deferred to a later phase. Exact supported browser versions, desktop executable OS support, and minimum client hardware remain open.

Use 3D assets and 3D rendering. Mods can control camera angle, motion constraints, lighting, and other supported presentation settings through the public API. A classic RTS camera is expected for the default game but is not an engine restriction.

### 3.2 Explicit exclusions and deferred scope

- No Minecraft-style requirement to assemble buildings block by block. Buildings remain conventional RTS structures with footprints and foundations.
- No underground movement for units, caves, tunnels, or navigable stacked terrain.
- No simulated landslides, falling terrain, or structural collapse physics in the initial scope.
- No requirement for detailed pressure-based fluid simulation or physically realistic electrical simulation by default.
- No guarantee that arbitrary native plugins can run in a browser.
- Scale ambition is tens of thousands of units, many players, large maps, high AI command rates, and potentially thousands of spectators. Initial benchmark player counts and server allocation are settled in Section 13.2. Maximum supported scale, map dimensions, engine version, tick rate, frame rate, and client hardware remain open; ambition is not measured capability.
- No production commitment to all illustrative mechanics at the first release.

Bridges with traversable space underneath would introduce multiple ground surfaces and are outside this baseline. Flying units remain a possible RTS capability; their movement and rendering rules require a separate design and do not imply navigable underground terrain.

## 4. Architecture and ownership

### 4.1 Logical modules

| Module | Responsibility |
| --- | --- |
| Simulation core | Authoritative world state, entities, orders, fixed simulation ticks, transactions, gameplay systems |
| Presentation | Bevy rendering, input, audio, UI, overlays, visual interpolation |
| Mod runtime and API | Content registration, behavior composition, event hooks, script execution, API versioning |
| Multiplayer | Commands, authority, validation, synchronization, session compatibility |
| Terrain and geology | Heights, materials, excavation/filling, geological queries, discovery state |
| Production and logistics | Inventories, recipes, transport, transfer systems, reservations |
| Utility networks | Nodes, edges, ports, topology, selected utility simulation models |
| Headless host | Executable reusing simulation without graphics |

**Baseline:** client-side package acquisition (Section 6.5) is split. Multiplayer owns the join handshake that fetches the server advertisement and lockfile. Mod runtime and API owns the package cache, hash verification, and loading; this is the host side of the module, not the script sandbox. Both run inside the trusted client shell (Section 6.13), not inside any mod.

These are responsibility boundaries, not a finalized crate layout. The simulation may use ECS internally, but must be runnable without a window or rendering plugins. Presentation consumes simulation state; visual meshes must not become the authoritative terrain representation.

### 4.2 Entity composition

Use capabilities and components rather than separate incompatible hierarchies for RTS units, factory buildings, and infrastructure. Illustrative compositions:

| Entity | Composed capabilities |
| --- | --- |
| Worker | Movement, inventory, gathering, hauling, construction or excavation orders |
| Smelter | Input/output inventories, recipe processor, optional power consumer |
| Barracks | Production queue, resource cost rules, unit-spawn output |
| Research lab | Input consumption, research contribution, optional power consumer |
| Turret | Targeting, weapon, optional ammunition inventory and power requirement |
| Pump station | Surface footprint, utility ports, pump behavior, optional power consumer |

Rust structs, traits, and implementation blocks provide encapsulation where appropriate; Bevy entities remain component compositions rather than class inheritance trees. Implementation should use clean, DRY code and four-space indentation.

### 4.3 Engine mechanisms versus mod policy

**Decided governing principle:** the engine enforces simulation integrity and exposes capabilities. Mods define gameplay policy. Default gameplay behavior belongs in the default mod or an optional reusable mod module. This applies across all domains below, not only admission and spawning. Platform resource budgets, identity, access controls, and explicitly agreed slot invariants remain engine responsibilities.

| Domain | Engine mechanisms | Mod policy and content |
| --- | --- | --- |
| Entities | Lifecycle, identifiers, components, queries | Unit/building definitions and capabilities |
| Movement | Navigation, collision, formation primitives, movement categories | Speeds, terrain permissions, unit-specific traversal |
| Economy | Storage, balances, reservations, transactions | Resource types, upkeep, population rules |
| Construction | Placement, footprints, progress, occupancy | Prerequisites, worker-built versus instant placement, costs |
| Combat | Target queries, damage events, projectile support | Weapons, armor, damage types, targeting restrictions |
| Abilities | Orders, targeting, timers, effects | Spells, mana, cooldowns, buffs, transformations |
| Progression | Prerequisites, unlocks, modifiers | Technology graph, hero levels, upgrade definitions |
| Visibility | Visibility queries and fog-of-war mechanisms | Sight ranges, stealth, detection, shared vision |
| Match rules | Players, teams, lifecycle, event hooks | Factions, victory conditions, diplomacy, modes |
| Interface | UI primitives and controlled state access | Resource displays, command panels, tooltips, screens |
| AI | Queries and command submission | Build orders, economic priorities, army behavior |

### 4.4 Separate simulation, replication, and presentation representations

**Baseline:** a gameplay entity has a compact authoritative representation, a selective network representation, and a client visual representation. These need not have one-to-one component or object layouts. The server must run without visual meshes, skeleton hierarchies, animation controllers, or cosmetic particle systems. It may load compact gameplay geometry and metadata derived from assets where rules require them.

For example, a tank can be represented by footprint, position, heading, turret angle, orders, and combat state on the server; permitted properties and events on the wire; and detailed meshes, attachments, tracks, recoil, and effects on the client. Animation does not determine authoritative hit timing or damage. Gameplay-relevant aim or timing must have explicit simulation data; cosmetic playback interpolates or follows that data.

Changing animation/detail levels, hiding an offscreen object, or switching the viewed map must not change gameplay outcomes. The mod API describes presentation capabilities without exposing Bevy render internals. Research and validation details are in Sections 13.3–13.9.

## 5. Mod system

### 5.1 Three extension layers

1. **Definitions:** declarative units, buildings, recipes, resources, technologies, maps, assets, and UI data.
2. **Composition:** combine built-in behaviors such as gathering, production, transporting passengers, gaining experience, and area damage.
3. **Scripts:** event-driven rules and custom behavior that cannot be expressed with definitions and composition alone.

High-volume operations such as pathfinding, inventory processing, transport movement, and production scheduling execute in Rust. Mods must not need to run a script for every unit or every transported item every frame.

### 5.2 Public API boundary

Mods target a versioned engine API rather than Bevy entity internals, scheduler details, or rendering implementation. Stable content identifiers and controlled commands/queries should cross that boundary. Script effects enter through supported simulation operations, not unrestricted mutable access to the entire world.

The default game has no private gameplay privileges. Reusable engine features may be added when needed, but must be exposed consistently to third-party mods.

### 5.3 Runtime and packaging

The scripting language is **open**; Rhai remains a prototype candidate. The accepted initial direction is one shared sandboxed script runtime for mods and libraries, with declarative data packages allowed without scripts. Rust is the engine implementation language, not the downloadable package format. Independently executable Rust/WASM modules are deferred pending a concrete need; host WASM compilation does not provide that runtime automatically.

Native Rust dynamic libraries are not the baseline public mod format because they do not meet browser parity requirements. Content packages and portable scripts are the preferred direction.

**Decided:** servers advertise the required mod and version and provide a URL for automatic download/setup. **Baseline package documents:** two documents exist. The *package manifest* is written by the package author; Section 5.5 owns its one field list. The *server lockfile* is produced by the server's resolver; Section 5.5 defines its entries. A package manifest states dependency version constraints and never holds its own hash, size, or download URL. Exact versions, content hashes, package sizes, and download URLs live once, in the server lockfile. The server advertisement (Section 6.4) names the game mod, its version, the required engine and network protocol versions, the lockfile URL, and the lockfile hash. Dependency resolution and loading follow Section 5.5. Archive format, publisher trust, registry protocol, override semantics, and cache implementation remain open; save migration is secondary. See Section 6.5 for the joining workflow.

### 5.4 Execution controls

The runtime must provide bounded script execution, controlled host capabilities, and useful errors. Scripts should respond to events such as production completion, research completion, discovery, and match lifecycle changes. The exact instruction, memory, and event budgets are open.

Multiplayer sessions must verify compatible gameplay mods and versions. Cosmetic-only differences require an explicit distinction rather than assuming that an asset or UI mod cannot affect gameplay.

### 5.5 Libraries, publishing, and dependencies

**Decided:** libraries are first-class publishable/loadable packages using the same package system and script runtime as mods. A game mod selects content and composes optional libraries; both can depend transitively on other libraries. Libraries export functions, definitions, and explicitly registered behavior. Importing a library must not silently activate gameplay; the consumer registers or instantiates its behavior. Mutable state is isolated per match even when immutable code/assets are cached.

**Baseline:** a package manifest declares nine fields. They are: namespaced identity, version, package kind, engine API compatibility, dependency version constraints, exports/entry points, execution context (server, client, shared), requested capabilities, and license metadata. Resolve before a match starts. Reject missing/incompatible dependencies and cycles with actionable diagnostics. Then produce an immutable server lockfile. The lockfile lists every package, the root game mod included. Each entry holds seven fields. They are: package identity, exact version, content hash over the exact downloaded package bytes, package size, download URLs in preference order, execution context, and engine API compatibility. The client fetches the lockfile from the URL in the server advertisement and verifies it against the advertised hash. The lockfile entry is authoritative; a package whose manifest disagrees with its entry is a load error. Clients acquire the server's locked set rather than independently selecting newer compatible releases. Start with one version per library per configuration; caches and separate server configurations may retain several versions. Dependency order does not substitute for explicit simulation scheduling constraints. The concrete resolver, registry/publishing API, packaging format, and optional-dependency semantics remain implementation choices.

### 5.6 Seeded generation libraries

**Decided:** map-generation algorithms are optional libraries. The engine supplies seeded random streams, bulk terrain/material operations, queries, placement, and serializable map state. Libraries choose landscapes, deposits, vegetation, and optional start/accessibility policies. Mods can select, configure, combine, or replace generators; handcrafted maps require none.

Record seed, parameters, generator/dependency versions, and relevant engine compatibility together; a seed alone is insufficient for reproducibility. Do not give clients the data needed to regenerate secret deposits when secrecy is required. Server-generated permitted map state is the baseline approach in that case. Scripts should orchestrate bulk engine operations rather than requiring per-cell interpreted processing.

### 5.7 Registries and publishing

**Decided:** operate a default package registry, support independently hosted compatible registries, and allow direct package URLs. Registries provide discovery, metadata, and version listings; package bytes may be delivered by separate static hosting/CDNs. Players joining a server receive its exact locked package set automatically without manually configuring the originating registries.

Publisher signing keys are distinct from player identity keys. Automated publishing must not require a developer's player private key. Package signatures and hashes do not by themselves guarantee trustworthy behavior; the runtime sandbox remains required. Registry API, namespace ownership/conflict handling, signature formats, publisher enrollment/rotation, and trust UX remain implementation choices.

### 5.8 Compiled 3D asset pipeline

**Baseline:** mods provide standard model/animation sources plus metadata describing clips/states, material slots, attachment points, bounds, and separate gameplay footprints. glTF is the preferred interchange candidate; exact supported features and compiled formats remain open. Package publishing/build tools produce validated runtime assets rather than depending on expensive ad hoc optimization during a match.

The pipeline should support mesh optimization, levels of detail, texture compression, animation preparation, shared-resource deduplication, and machine-readable diagnostics. Preserve named turret pivots, weapon sockets, and other referenced nodes through optimization. Validate the compiled output against the pinned client importer, not only against the source format. An extension accepted by a glTF validator is not automatically supported by the chosen Bevy version.

Record compiler/version/settings and source-content hashes so compiled assets can be cached and associated with exact locked packages. Keep gameplay data independent of cosmetic quality variants. Tooling should report mesh/material counts, skeleton sizes, clip counts, texture memory estimates, and unsupported features; numeric budgets and warning/rejection policies remain to be measured. Agents should receive actionable structured errors and create representative test scenes through the same public workflow as third-party mods.

Candidate tools: [gltfpack](https://meshoptimizer.org/gltf/) for mesh/animation optimization and [Khronos glTF Validator](https://github.com/KhronosGroup/glTF-Validator) for format validation. Neither supplies the complete engine-specific asset pipeline.

## 6. Simulation and multiplayer

### 6.1 Shared simulation principles

Use fixed simulation ticks independent of rendering. Clients issue orders; authority validates and applies them. Headless operation enables repeatable scenarios, replay/debug tooling, and performance measurement.

Rust and ECS do not by themselves guarantee determinism or performance. Deliberate scheduling, randomness, and mutation rules remain useful for correctness and reproducible server tests; full deterministic client synchronization is not required by the selected authority model.

### 6.2 Authoritative server simulation

**Decided:** the server computes authoritative movement, combat, economy, terrain, and other simulation results. Clients submit validated commands and present permitted state. Selective replication, client interpolation, and immediate input acknowledgement are the baseline; final networking library and transport remain open. Full deterministic client lockstep is not the chosen multiplayer architecture. Repeatable server scenarios remain useful for debugging but cross-platform full-world determinism is not required for synchronization.

Economical small/medium hosting is a core requirement. Authority does not imply broadcasting all state or all entities every tick. Reuse proven RTS simulation techniques and compatible open-source infrastructure. StarCraft-style lockstep performance cannot be used as direct evidence of authoritative server hosting cost.

WebSockets remain a straightforward prototype option; WebTransport and WebRTC are candidates subject to library and browser validation. See Section 6.10. No transport or crate has been selected.

### 6.3 State requiring synchronization

The chosen model must cover unit orders and outcomes, inventory and balance changes, production, research, terrain edits, geological depletion, utility topology and operating state, and mod-generated events. Discovery information must be scoped to the appropriate player or team.

**Decided:** saves are server-side; there is no standalone solo mode or local game-save product. Server-hosted matches against bots are compatible with this scope. **Baseline save contract:** preserve authoritative state, slot ownership, and the exact server lockfile. Reconnection restores a player to their claimed slot; late joining can claim an eligible unclaimed slot only when permitted by the mod. Those behaviors are decided; snapshot/catch-up and crash recovery remain implementation work. Replay storage/offering is optional server functionality and secondary. Initially load exact mod versions rather than requiring migration; archive availability and engine-version compatibility still need handling. Browser package caches are not local game saves.

### 6.4 Discovery and hosting topology

**Decided:** the client connects to a configurable index that lists servers. Ship a default index operated by the project, but allow users to change its URL and communities to operate their own compatible index.

A server is dedicated to a game mod; it hosts multiple rooms rather than a single match. The working refinement is one exact mod/dependency set per server, with room-specific settings. **Baseline:** the server advertisement is the document a server publishes. It names the game mod, its version, the required engine and network protocol versions, the server lockfile URL, and the lockfile hash. The server lockfile is frozen for the server's lifetime unless an explicit versioned mechanism replaces it. Hosting unrelated game mods within one server has not been requested.

| Layer | Responsibility |
| --- | --- |
| Index | Discover servers and advertise compatible game offerings |
| Server | Publish the server advertisement, list rooms, authorize room creation, supervise matches |
| Room | Own configuration, creator/admin identity, access settings, slots, lifecycle |
| Match instance | Run isolated simulation for that room |
| Package host | Deliver mod/map packages; may be separate static hosting or CDN |
| Spectator distribution | Buffer and distribute permitted match state independently of player admission |

**Baseline operational design:** registration with expiring heartbeats, direct connection, and saved servers independent of index availability. An index outage must not terminate established matches. Listing metadata can include endpoint, mod/version, engine compatibility, region, occupancy, and spectator availability. Revalidate against the actual server advertisement on connection; the listing can be stale.

A room may exist before simulation and persist afterward for results or rematches. Room management and simulation need separate lifecycles. Separate worker processes per active match are a **recommendation**, not a selected deployment requirement. The aim is fault and resource isolation. World sharding across machines is not selected.

### 6.5 Automatic mod acquisition and admission preparation

**Decided user experience:** selecting a server/room should download and set up missing required mods without manual download or installation steps. The server supplies the content URLs; it need not serve package bytes itself.

**Baseline workflow:** fetch the server advertisement and lockfile → verify the advertised engine and network protocol versions against the client (the server's resolver has already proved each locked package's engine API compatibility) → compare the local cache against the locked hashes and list the missing packages whose execution context is client or shared → download each missing package and verify its hash before extraction → load an isolated session profile → complete admission/loading → obtain initial state and catch up if the match is running. The client never resolves versions itself; it takes the server's locked set (Section 5.5, D35). The client does not download server-only packages.

Use exact package hashes in addition to version strings. Permit side-by-side versions in the cache so joining one server does not break another; one version per library per configuration (D35). Packages must use the supported portable mod runtime, not arbitrary native installation scripts. Hash verification proves identity/integrity, not publisher trust. Sandbox downloaded scripts, constrain extraction and execution, and apply capability limits. Player identity and separate publisher keys are settled in Sections 6.13 and 5.7; detailed signature verification and publisher trust policy remain implementation work.

For browser clients, setup uses browser-managed storage; content delivery must support HTTPS and appropriate cross-origin access. Storage eviction and download failures require recoverable behavior. Incompatible engine requirements must yield a clear error; mod setup does not silently replace the client executable.

**Baseline:** the server lockfile is frozen at server level (Section 6.4); freeze each room's rule configuration for the running match unless an explicit versioned mechanism supports changing it. Download/setup and slot admission are separate phases; temporary reservations, loading timeouts, and the exact claim boundary require design so content preparation does not accidentally consume permanent slots.

### 6.6 Rooms, creators, and configuration

**Decided:** players can create rooms, choose player count, set a short message such as “Noobs only,” set an optional joining password, and configure rules exposed by the mod. The engine must not hardcode a specific RTS rule list.

**Baseline:** mods declare a settings schema with types, defaults, ranges, choices, labels, and validation. The client generates configuration controls; the server validates all submissions. Example settings such as teams, map, starting resources, research speed, and victory conditions are mod options, not universal requirements. Hosting limits constrain resource use without inventing gameplay rules.

Password checks occur server-side; passwords are not exposed in directory results. Protected rooms may remain listed. Unlisted invitations are an optional **proposal**, not a decided feature. Spectator access is a separate permission so a private match is not accidentally public.

The creator administers the room but does not supply the authoritative simulation host. Their departure must not inherently terminate a dedicated-server match. **Decided administration defaults:**

| Situation | Behavior |
| --- | --- |
| Creator leaves the lobby | Transfer administration to another connected human; close after a timeout if none remain |
| Creator disconnects during play | Match continues and their claimed slot stays theirs |
| Rule changes | Creator can edit before start; during-play changes require explicit mod support |
| Adding bots | Creator selects mod-provided bots for unclaimed lobby slots |
| Removing participants | Creator may remove participants before start; during-play moderation belongs to server operators |
| Everyone disconnects | Apply server-configured pause/save/shutdown policy; retain slot ownership on restoration |

Room administration is independent of player-slot ownership and does not grant control of another player's units. Transfer selection, timeout values, and exact permission messages remain implementation details. None of them may reassign an already claimed player slot during that match.

### 6.7 Match start and late admission

**Decided capabilities:** a creator may start a game, or it may start automatically when player slots are joined. A match may begin with empty slots. Mods determine which start policies/configurations are valid and whether late joining is permitted. The engine supplies lifecycle hooks and validated operations rather than universal minimum-player or auto-fill rules.

A short automatic-start countdown and optional readiness toggles were **suggestions only**. Neither is mandatory engine gameplay. Required clients must complete technical initialization before participating; the loading barrier and failure handling are implementation concerns.

Late admission can claim an eligible unclaimed slot, up to the room's configured capacity. It cannot replace a departed player. An empty slot need not remain eligible forever: the mod can block or close it for any supported gameplay reason. There is no built-in late-join deadline.

### 6.8 Slot state, identity, and ownership contract

**Decided:** a slot represents participation and ownership independently of a spawn location, faction, base, territory, or initial units. Those associations, if any, are defined by the mod. A slot may be claimed without creating anything in the world.

| State | Engine contract |
| --- | --- |
| Open | Unclaimed; eligible for a claim when room/match admission is permitted by the mod |
| Blocked | Unclaimed; unavailable until the mod reopens or permanently closes it |
| Closed | Unclaimed and permanently unavailable for the remainder of this match |
| Claimed | Owned by one player for the entire match, whether connected or disconnected |

Permitted match-time transitions: open ↔ blocked; open/blocked → closed; open → claimed through validated atomic admission. Closed and claimed are terminal for the match. Connectivity is a separate property: disconnecting, leaving, or reconnecting does not alter ownership. Room-admin privileges do not bypass this invariant. Lobby edits before match initialization require their own lifecycle semantics and cannot reset ownership in a running match.

The engine authenticates identity, prevents simultaneous duplicate claims, records ownership, and restores the correct player on reconnect. Use portable cryptographic identity as defined in Section 6.13; display names alone are insufficient. Spectating does not claim a player slot.

**Mod-owned policy:** whether joining is permitted, when unclaimed slots become open/blocked/closed, what joining creates or assigns, and how a disconnected player's entities behave. Queries for nearby entities, terrain, occupancy, or ownership are available but optional.

**Explicitly rejected as engine rules:** spawn exclusion radii, mandatory predefined start positions, nearby-building thresholds, resource availability requirements, spawn reservation zones, automatic late-join cutoffs, expansion-priority rules, and automatic AI takeover. These may exist in the default mod or optional mod modules. No human replacement or slot recycling is allowed within the match, even if a mod changes what happens to the disconnected player's assets.

### 6.9 Scale, command processing, and replication

**Decided ambition:** large maps with many players, tens of thousands of units, high APM from AI participants, and spectators potentially numbering thousands. These are target workloads, not measured capabilities. Exact simultaneous commander count and concentration of units remain open.

**Selected authority model / baseline replication:** authoritative commands and selective state replication. Interest is based on gameplay authorization and information needs, not only camera position. Off-screen owned units, production status, alerts, and minimap data may still be required; hidden enemy/deposit state must respect mod visibility policy.

Use reliable validated orders, prioritized movement updates, batched economy/research changes, versioned terrain chunk data, and compact transport/production representations. Do not assume every unit, belt item, or field must be transmitted every simulation tick. Spatial filtering is less effective when a large battle is visible to everyone; test that case explicitly.

**Proposed platform controls:** budgets for commands per tick, affected entities, pathfinding work, edits, script execution, observations, and queued work. Packet size is not a sufficient cost measure. Support group orders, batching, and persistent intentions. AI models operate asynchronously through controlled observation/command interfaces; inference must not block the simulation tick. Budget values remain open and do not prescribe game balance or human-versus-AI rules.

### 6.10 Open-source networking reuse

**Decided:** reuse an existing open-source foundation rather than rebuilding transport/replication machinery from scratch. Favor permissive licensing compatible with potential commercial distribution; inspect pinned dependencies and required notices before shipping.

Shortlist recorded from the discussion:

| Candidate | Rationale | Status |
| --- | --- | --- |
| [Lightyear](https://github.com/cBournhonesque/lightyear) | Bevy-oriented transport and replication integration; current examined architecture uses Replicon | First prototype recommendation, not selected |
| [Bevy Replicon](https://github.com/simgine/bevy_replicon) | Server-authoritative replication and visibility control | Alternative assembly foundation; may also be used through Lightyear |
| [Naia](https://github.com/naia-lib/naia) | Rust/native/WASM networking and protocol/replication facilities | Comparison candidate |

The examined repositories expose permissive MIT/Apache licensing, but exact version compatibility and dependency licensing must be verified during selection. No cited library establishes the target performance envelope. Transport reliability, serialization, and replication can be reused; game-specific relevance, authority, mod schemas, and simulation costs still require engine work.

**Baseline mod protocol design:** engine-defined message envelopes, stable content IDs, validated mod schemas, supported field types, visibility declarations, and bounded payloads. Downloading a mod that defines a unit or property must not require recompiling the engine client. Do not expose library-specific or Bevy-internal types as the public mod protocol.

### 6.11 Spectators and distribution

**Decided:** support non-player viewers. **Baseline proposal:** simulation publishes a bounded feed to a separate spectator service, which handles viewer connections and can distribute through additional relays. Audience growth must not create one additional feed or expensive query stream per viewer on the match server.

Use periodic checkpoints plus sequenced state changes and presentation events; an event-only stream is insufficient unless viewers reproduce a complete deterministic simulation. A joining viewer loads matching content, receives a checkpoint, and catches up. Relays may maintain read-only state and filter it by viewer needs. Replay retention, seek, and exact transport remain open.

Slow viewers, full relay buffers, and relay failure must not block match simulation. Export has a nonzero fixed cost; the goal is approximately bounded simulation-server work as the audience grows, not literally zero overhead. Relay provisioning and bandwidth must scale separately.

Viewer permissions and information policy belong to the mod/room configuration within server access controls. Possible modes include omniscient, delayed, or player-perspective views; none is universal gameplay. When delay or restricted perspective is selected, enforce it before unauthorized information reaches the viewer. Viewing must not consume or claim a player slot.

### 6.12 Multiple maps per match

**Decided:** a match can contain several maps (for example separate planets), including very large maps, with client loading and switching between them. This does not create stacked navigable terrain within a map.

**Baseline design:** match-level players, slots, configuration, lifecycle, and cross-map operations; map-local terrain, geology, entities, spatial queries, navigation, and visibility. Mods decide which progression, resources, and networks span maps. Use explicit map identifiers and cross-map references/transfers. Stream map/region presentation and permitted state so browsers need not keep every map fully resident.

Leaving a map changes the client's view, not automatically its simulation. Unobserved factories/combat may still affect the match. Sleeping or simplified simulation needs explicit semantics preserving gameplay. Map-level work isolation is a performance opportunity, not a commitment to distributed world sharding. Exact streaming, transfers, and inactive-map execution remain implementation work.

### 6.13 Server implementation and identity

**Decided:** our initial server uses SQLite. Third-party compatible servers may choose different databases and internal architectures; the required boundary is protocol behavior, not a prescribed persistence engine. Storage must not dictate the public gameplay API.

**Decided:** portable cryptographic identity independent of a central account service or server index. The trusted client generates and holds a key pair; a stable player identifier is derived from its public key. Display names are mutable labels, not authentication. Servers verify proof of key possession using a fresh challenge and signatures, then establish authenticated sessions. Use established authentication components and a reviewed protocol; algorithm, challenge/session binding, encoding, and wire details remain engineering choices.

Private keys never go to game servers, logs, or mod scripts. Keep signing and identity storage in the trusted client shell, outside the mod runtime. Store identity locally; browser storage alone is not a durable backup. At account creation offer copying/downloading the private key, explain recovery consequences, and support import in another browser/device. A password-encrypted export may also be offered. Recovery initially depends entirely on the user's saved key: losing every copy loses access to that identity; possessing the key permits impersonation. There is no central recovery service. Device enrollment and key rotation are deferred implementation topics.

Servers bind claimed slots, permissions, and saves to the verified identity and retain their own authorization and ban policies. Using the same identity across servers is linkable; a key proves continuity, not a unique human, since a user can generate another identity. Restoring a key restores identity access, not a local copy of server-owned saves.

**Decided session policy:** an identity may connect to multiple servers, but each claimed slot has at most one active controlling session. A second connection to that slot must explicitly replace the first; prevent simultaneous controllers. Exact takeover UX, session revocation, and spectator-session behavior remain implementation details.

## 7. Resources, production, and logistics

### 7.1 Resource representations

| Kind | Examples | Representation and behavior |
| --- | --- | --- |
| Abstract balance | Credits, gold, research points | Player/team-owned amount without physical location |
| Physical item | Ore, ammunition, machine parts | Stored in inventories, on transport systems, or in the world |
| Network quantity | Electricity, water, heat | Distributed according to the selected network model |

Mods may use any combination. A traditional RTS may use global balances; an industrial game may require physical deliveries; a hybrid may combine credits with local ammunition supplies.

### 7.2 Inventories and transactions

The engine provides capacity checks, filters, reservations, transfers, and consistent accounting. Mods define accepted items, capacities, access rules, and costs. Concurrent producers or haulers must not consume or reserve the same stock twice.

Different production policies are valid: charge when queued, reserve then charge when processing starts, or consume over time. Cancellation, refunds, interrupted production, output blockage, and destruction policies must be explicit rather than accidental side effects.

### 7.3 Production and factories

Recipes define inputs, outputs, processing duration, eligible machines, and optional operating conditions such as power. The engine schedules execution and handles input/output transactions. Shared primitives should support item production, unit production, and research contribution without assuming identical queue semantics.

### 7.4 Belts and inserters

Belts require item movement, lanes where enabled, junctions, occupancy, and backpressure. Mods define speed, connections, appearance, and permitted items. Inserters transfer between compatible endpoints with mod-defined reach, rate, filters, and stack size.

Baseline performance direction: use a specialized compact transport representation rather than assuming every belt item is a full ECS entity updated individually. Keep simulation and rendering separate. Exact belt segment representation, junction fairness, and update scheduling require benchmarks and design work.

### 7.5 Hauling and utility integration

Provide delivery requests, reservations, and routing primitives for worker or vehicle hauling. Mods define priorities and allocation policy. Factories and weapons may connect to utilities, but power consumption is not mandatory for every production entity.

## 8. Research and progression

Research is a prerequisite graph, not necessarily a strict tree: one technology may require several earlier technologies. Technology definitions are separate from player/team research state.

Mods define costs, prerequisites, research facilities, progression rate, and effects. Effects may unlock recipes, units, buildings, abilities, detectors, or numerical modifiers. Laboratories may consume physical items or abstract balances and may require power.

Research cancellation, parallel research, exclusive branches, repeatable technologies, team sharing, and modifier stacking are **open policies**, not fixed engine rules. Their APIs should be designed when concrete default-game needs are established.

## 9. Terrain representation and editing

### 9.1 Chosen model

The world has one ground-surface elevation at each horizontal location. Units cannot enter excavated underground volumes or stand on overlapping traversable ground surfaces. Excavation lowers the surface; filling raises it. Buildings are separate RTS entities, not terrain blocks.

Baseline implementation direction: a regular grid divided into chunks, with integer height steps and independent visual presentation. Grid resolution, vertical step size, smooth versus stepped rendering, and map dimensions remain open. Smooth rendering must not conceal movement or placement rules from the player.

### 9.2 Supported terrain operations

- Excavate individual cells or designated areas.
- Lower ridges or portions of a mountain to create a passage.
- Dig trenches and open pits.
- Add ground to form dirt walls or embankments.
- Level a site for construction.
- Extract resources from removed material when the mod permits it.

Expose operations as work orders that workers or machines can execute over time. Mods may also permit instant edits for creative modes, abilities, or map editing. Costs, range, tool requirements, hardness, and allowed materials are mod rules.

### 9.3 Authoritative edit transaction

Baseline sequence: request work → validate permissions, occupancy and costs → advance work → commit material removal/addition and associated resource accounting → invalidate dependent regions → update presentation.

Terrain is edited in simulation state, not by manipulating the visible mesh. Excavation and its resource yield must be committed consistently so the same removed material cannot be awarded twice. Filling consumes or otherwise accounts for material according to the mod’s policy.

### 9.4 Dependent systems

| System | Required response to terrain change |
| --- | --- |
| Navigation | Re-evaluate affected traversability, slopes, routes, and clearance |
| Placement | Re-evaluate footprints, flatness, and foundation validity |
| Visibility/combat | Refresh affected line-of-sight or firing obstruction data |
| Surface logistics | Re-evaluate support and connection rules for belts/pipes |
| Underground utilities | Recalculate burial depth and detect exposure or conflicts |
| Rendering | Update affected chunk geometry and associated visuals |

Avoid rebuilding the entire map after each edit. Batch and localize work where possible, while ensuring gameplay cannot exploit stale navigation or placement results. The precise scheduling and consistency rules are open implementation work.

### 9.5 Access and foundations

Deep pits need ramps or another mod-supported access method for ordinary units. Traversal limits may differ by unit type. Trenches can block some units while allowing others. Buildings need explicit slope, flatness, and foundation requirements. Digging under a structure may be forbidden, invalidate it, or trigger a mod-defined consequence; the engine should not silently pick one policy.

## 10. Geology, extraction, and discovery

### 10.1 Compact geological columns

Below each surface location, store material intervals rather than a full voxel volume. An example column contains 3 m of soil, then 5 m of rock, then a 3 m iron-bearing interval, followed by bedrock. Removing the upper layers exposes the next material without creating a subterranean space.

Baseline conceptual data: current surface elevation, remaining material intervals or equivalent depletion representation, and separate per-player/team knowledge. Geological coordinates must remain anchored to a stable world datum or otherwise account for edits; repeatedly measuring immutable deposits relative to the changing surface would incorrectly move them.

Filling adds material above existing remaining geology. It must not regenerate previously mined ore. Placed material has explicit provenance or yield rules sufficient to prevent unintended placement/extraction resource loops.

### 10.2 Prospecting mechanics supported by the API

| Mechanic | Possible mod behavior |
| --- | --- |
| Exploratory digging | Reveal material only when exposed |
| Prospecting drill | Sample below one location |
| Basic detector | Report approximate presence within a radius |
| Advanced scanner | Estimate material, depth, and extent |
| Research upgrades | Improve range, resolution, accuracy, or material coverage |

The engine provides geological queries and discovery records. Mods define equipment, operating costs, timing, uncertainty, and presentation. Random digging remains a valid discovery strategy when the game permits it.

### 10.3 Information secrecy

Actual geology and player knowledge are distinct. A UI overlay must not reveal undiscovered data. If competitive secrecy is required, authoritative hosting must also withhold hidden deposit data; hiding it only visually is insufficient. This requirement influences the multiplayer decision in Section 6.

## 11. Underground infrastructure

### 11.1 Representation

Underground infrastructure is a network of nodes and connections with optional spatial routes, fixed elevations, capacity, and operating state. It is not an excavated void, underground room, or navigable level. No underground unit navigation or terrain-volume meshing is required.

Begin with one logical underground layer. Depth/elevation metadata may support burial checks or gradient-sensitive rules without introducing multiple playable floors or a full multilayer underground editor.

### 11.2 Placement and connectivity

- Routes crossing in plan view do not connect automatically.
- Explicit compatible junctions establish connections.
- Different utilities may share a corridor if mod rules permit it.
- Surface buildings connect through defined ports.
- Route validation can inspect burial, clearance, terrain material, and placement conflicts.
- Initial elevation is derived from requested burial depth or route placement rules; once placed, it remains fixed unless explicitly relocated.

If terrain is lowered, a pipe becomes shallower or exposed. If terrain is raised, it becomes more deeply buried. Infrastructure does not automatically follow the moving surface. Mods decide whether exposure is allowed, damages the line, removes it, or requires repair.

### 11.3 Utility behavior

| Utility example | Suitable possible model |
| --- | --- |
| Electricity | Aggregate supply and demand per connected network |
| Water | Capacity-limited distribution with optional pumps |
| Sewage | Optional directional/gradient constraints |
| Heat | Supply, consumption, optional losses |
| Communications | Connectivity or capacity requirements |

Share topology and common mechanisms without assuming all utilities are interchangeable. A generic graph does not itself solve fluid distribution, pressure, congestion, or losses. Each supported simulation model needs explicit semantics. Initial scope should favor simple models; detailed physical behavior is optional future work.

### 11.4 Visualization and scheduling

Provide an inspection/placement mode that fades terrain and shows colored routes, junctions, building ports, and operational information. Mods define colors, symbols, and labels. A fully rendered underground environment is unnecessary.

Update connectivity when topology changes. Simulate utility operation at a suitable fixed cadence independent of rendering. Exact cadence and models remain open; lower update frequency must not introduce inconsistent resource accounting.

## 12. Cross-system invariants

These are baseline correctness requirements derived from the agreed mechanics:

1. The default game and external mods use the same supported gameplay API.
2. Terrain, inventories, deposits, research, and networks have authoritative simulation state independent of visuals.
3. Removing terrain cannot award its resources more than once.
4. Adding material cannot recreate depleted geological deposits.
5. Reservations and transfers cannot double-spend shared stock.
6. A change in terrain must notify every affected dependent system.
7. Underground infrastructure cannot create a second playable surface.
8. Network crossing and network connection are distinct operations.
9. Discovery state is separate from geological truth and scoped to the correct player/team.
10. Multiplayer compatibility includes gameplay mod versions and content, not merely engine version.
11. Claimed player slots cannot be reassigned or reopened during a match, including after departure.
12. Player slots do not inherently require spawn locations or world entities.
13. Mods determine admission and gameplay eligibility; the engine enforces identity, atomic transitions, access controls, and budgets.
14. Spectators do not claim player slots, and viewer backpressure must not stall simulation.

## 13. Performance strategy

Prefer bounded work and compact representations: geological intervals instead of voxels, networks instead of underground scenes, localized terrain invalidation, compact belt storage, and event-driven scripting. Expensive work belongs in optimized Rust systems rather than per-object script callbacks.

Measure browser and native builds using representative mixed workloads. A scene with many stationary units is insufficient: include path requests, congested belts, production completion, terrain edits, visibility updates, and utility changes. Browser compatibility, memory, startup payload, script overhead, and network traffic all need budgets.

Initial workload and server-allocation budgets are agreed in Section 13.2. Numeric latency, bandwidth, and other acceptance thresholds remain **open** and must be set before performance claims or maximum world sizes are promised. ECS, Rust, or WASM alone does not establish a supported scale. Browser threading and graphics assumptions must be validated against the actual deployment matrix.

### 13.1 Reuse and economic hosting

**Decided:** assemble established RTS techniques and suitable reusable components; do not rebuild mature infrastructure without a concrete need. Evaluate licensing and integration before selection. Published proprietary-engine techniques can inform implementation without providing reusable source code.

**Baseline candidates to benchmark:** cache-friendly storage; event-driven wakeups; scheduled work; spatial indexes; hierarchical/group navigation and route reuse; localized terrain invalidation; compact transport lines; batched transactions; delta replication, quantization and interpolation; independent map/task parallelism with controlled shared state; per-package script profiling; lightweight room management and bounded spectator export. These are techniques to validate, not guarantees or a requirement to use every technique in every subsystem.

Avoid arbitrary player/map/entity caps, but enforce technical capacity and operator budgets. Finite hardware still has limits. Define small, medium, and stress workloads as benchmark scenarios rather than universal gameplay restrictions. Measure tick time including tail latency, memory, bandwidth, and hosting cost; include concentrated battles, mixed industry, multiple maps, terrain edits, bot commands, and multiple rooms. Keep automated performance regressions. Initial workloads and hardware allocation are agreed below; exact acceptance thresholds remain open.

Architectural references researched during discussion:
- [Planetary Annihilation architecture](https://www.forrestthewoods.com/blog/tech_of_planetary_annihilation_chrono_cam/): authoritative multi-planet simulation, changed-property replication, spectator distribution; adopting ChronoCam/replay history is not required.
- [Factorio belt optimization](https://www.factorio.com/blog/post/fff-176): compact transport lines and gap-based item movement.
- [Factorio independent transport groups and wakeups](https://www.factorio.com/blog/post/fff-364): parallel independent work and event-driven inactivity.
- [Deterministic simulation presentation](https://media.gdcvault.com/gdceurope2016/presentations/Salz_David_Deterministic_Simulation.pdf): lockstep background, not the selected synchronization design.

### 13.2 Initial hosting benchmarks and GCP testing

**Decided benchmark scope; these are test workloads, not engine or mod limits:**

| Scenario | Workload |
| --- | --- |
| Small | 4 players, 1,000 total units, one map |
| Medium | 8 players, 5,000 total units, two maps |
| Stress | Tens of thousands of units, concentrated battles, multiple maps, high command rates |

For small and medium matches use an initial allocation of **4 vCPUs and 8 GB RAM**. Measure actual CPU, memory, bandwidth, simulation latency, and hosting cost. These are agreed evaluation conditions, not a demonstrated performance guarantee. GCP machine family/CPU, region, disk, tick rate, run duration, player/bot mix, map dimensions, and pass/fail thresholds still need implementation-time specification for comparable results. Do not infer that multiple medium rooms fit this allocation until measured.

**Decided plan:** the user will grant a GCP service account so the implementing agent can set up and monitor VM tests. Credentials/access have not yet been supplied or verified in this discussion; no VM has been provisioned by this specification update. The implementation should use the granted project and permissions for reproducible deployment, load generation, telemetry, result collection, and test-resource cleanup. Record machine configuration and workload/package versions with results. Concrete cloud cost limits, VM lifetime, and provisioning parameters remain to be set when access is provided.

### 13.3 Research status and evidence boundary

The research recorded in this revision inspected published primary-source engineering articles, Bevy animation/instancing example code, Detour crowd header structures, and candidate-library documentation. No project benchmark, integration prototype, or full dependency audit was executed as part of this research. Published demonstrations establish techniques and reference implementations, not this engine's supported unit count or browser performance. Candidate repositories must be evaluated at pinned, mutually compatible versions before adoption.

### 13.4 Large-scale rendering and animation

**Baseline:** reuse meshes/materials, batch compatible objects, spatially organize render instances, and choose visual detail by projected size and visibility. A mod-controlled camera makes fixed-distance assumptions insufficient. Keep separate budgets for geometry, animation, shadows, transparent effects, and resource uploads.

| Content | Recommended path to evaluate |
| --- | --- |
| Buildings | Shared static meshes, with animated moving parts where required |
| Tanks and harvesters | Instanced geometry, independent turret/part transforms, suitable track/wheel material animation |
| Mass infantry | Instancing with baked skeletal clips/bone palettes evaluated on the GPU |
| Close-up or complex units | Flexible individual animation when required and affordable |
| Distant units | Simplified meshes, reduced animation detail, optional mod-appropriate strategic icons |

GPU skinning does not remove CPU animation-controller or transform-hierarchy work. [Bevy's Many Foxes example](https://bevy.org/examples/stress-tests/many-foxes/) shares model/animation assets while setting up individual animation players and transitions. It is a baseline to profile, not proof of mass-battle performance.

For the mass-animation path, evaluate precomputed clip bone transforms stored in GPU-readable data, with per-instance clip/time/speed/blend state. [NVIDIA's animated-crowd implementation](https://developer.nvidia.com/gpugems/gpugems3/part-i-geometry/chapter-2-animated-crowd-rendering) documents animation textures, instancing, variation, and LOD. Its historical native benchmark numbers must not become browser promises. Bone-palette baking is distinct from baking every vertex of every frame; compare memory, blending, attachments, and procedural pose needs before choosing a format. Do not force all units onto one animation path.

[Bevy's custom-instancing example](https://bevy.org/examples/shaders/custom-shader-instancing/) recommends built-in batching first and disables standard frustum culling because instance transforms are not represented by the base mesh bounds. A production implementation therefore needs correct spatial batch/per-instance bounds and visibility handling. Validate shadows, picking, attachments, animated bounds, and detail transitions alongside draw throughput. Copying a demonstration shader is not a complete crowd renderer.

Effects must have bounded emission, lifetime, distance/detail, and simultaneous-instance costs. Cosmetic effects may be reduced under load without changing combat. [Hanabi](https://github.com/djeedai/bevy_hanabi) is a GPU-particle candidate whose documented WASM path requires WebGPU. WebGPU as a requirement versus an additional WebGL fallback remains **open**; a fallback would need a compatible effects path and its own testing. No minimum browser/GPU capability is selected by this revision.

### 13.5 Simulation work reduction and visibility

**Baseline:** optimize frequently accessed data layout and reduce unnecessary work before relying on threading. Use spatial indexes for bounded local queries; sleep inactive production systems with explicit wake-up conditions; schedule strategic decisions and other suitable work independently of the rendering cadence. Gameplay accounting and temporal semantics must remain correct when systems sleep or use different update rates. Prevent per-unit/per-item script callbacks from becoming the default high-volume execution model.

[Factorio's optimization report](https://www.factorio.com/blog/post/fff-421) describes idle roboport wakeups and incrementally maintained radar coverage. These support evaluating event-driven inactivity and incremental visibility updates. Simple coverage counters do not by themselves implement terrain occlusion, stealth, or arbitrary mod sight rules; dirty-region invalidation must respect the actual visibility model.

[Factorio's transport-group parallelization](https://www.factorio.com/blog/post/fff-364) demonstrates both independent-group opportunities and shared-state/wakeup complications. Parallel tasks must have explicit ownership and synchronization boundaries. Rendering visibility cannot decide whether authoritative factories, combat, or movement run.

### 13.6 Navigation, steering, and terrain changes

**Baseline:** separate long-range routing from local motion. Evaluate hierarchical region/portal routes, cached/shared paths or flow fields for appropriate group destinations, and bounded-neighbour local avoidance. Flow fields share route guidance; they do not solve formations, congestion, deadlocks, or unique orders automatically. Avoid all-pairs nearby-unit queries.

[bevy_flowfield_tiles_plugin](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin) is a Rust candidate with tiled cost/integration/flow fields, caching, and asynchronous route requests. Its documentation explicitly leaves steering to the game. Assess sparse residency, actor-radius classes, mutable terrain, cache growth, task integration, and compatibility before adoption; do not assume its full-world initialization strategy fits extremely large maps.

[Recast/Detour](https://github.com/recastnavigation/recastnavigation) is an implementation reference and possible component, not a selected backend. The inspected [Detour crowd header](https://recastnav.com/DetourCrowd_8h_source.html) separates path corridors, proximity queries, local boundaries, neighbour lists, and steering parameters. Bounded local data structures are useful patterns, not evidence that its crowd controller handles our stress workload unchanged. Dynamic obstacle support must not be confused with arbitrary terrain deformation support.

Queue path requests and cap per-tick work with explicit pending states. Version affected navigation regions after terrain edits; asynchronous results must be checked for staleness before use. Rebuild/invalidate locally, with defined temporary movement behavior while updates are pending. Bound route caches and invalidation work. These details must preserve valid movement rather than allowing units to traverse stale blocked terrain. Test shared destinations, unique destinations, repeated order changes, chokepoints, mixed footprints, and edits during movement.

### 13.7 Compact replication and large-world residency

**Baseline:** transmit permitted gameplay properties and meaningful state changes, not skeleton poses or individual cosmetic particles. Interpolate visual movement between snapshots; suppress unchanged data and prioritize updates within bandwidth budgets. Apply authorization/fog-of-war filtering before distance/detail filtering. Preserve offscreen owned-unit status, alerts, and other required information. Large battles visible to everyone remain the critical case where spatial interest offers little relief.

[Planetary Annihilation's published architecture](https://www.forrestthewoods.com/blog/tech_of_planetary_annihilation_chrono_cam/) is a reference for authoritative multi-planet state and changed-property curves. [Lightyear's examined README](https://github.com/cBournhonesque/lightyear/blob/main/README.md) documents Replicon-based world replication, browser transport support, interpolation, customizable serialization, and bandwidth priorities. These facilities do not supply our mod schemas or visibility policy. Do not assume whole-world client prediction/rollback is needed for a large authoritative RTS.

Illustrative bandwidth arithmetic, not a measured wire format: 5,000 units × 16 payload bytes × 10 updates/s = 800,000 bytes/s per receiving client before overhead. Measure actual encoded payloads, initial snapshots, resynchronization, and multi-client fan-out.

Treat coordinate precision, storage residency, navigation residency, and simulation scheduling as separate problems. Use explicit map/chunk identifiers, chunked data, streamed presentation assets, and independently tracked active simulation work. Rendering one map need not retain all other maps' visual resources. Unviewed maps continue gameplay where required.

[big_space](https://github.com/aevyrie/big_space) is a candidate for integer-grid/local-coordinate floating-origin rendering. It addresses large-coordinate precision, not automatic world streaming or economical simulation. Exact coordinate representation, chunk dimensions, streaming budgets, and loading behavior remain open. Avoid full-world allocation merely because coordinate ranges permit a huge world.

### 13.8 Benchmark expansion and acceptance evidence

Retain the agreed hosting workloads and allocation in Section 13.2. Add the following prototype scenarios; these are validation cases, not maximum mod limits or performance promises.

| Scenario | Required evidence |
| --- | --- |
| 1,000 / 5,000 / 10,000 animated units, then larger stress cases | Separate animation/controller CPU, render preparation, GPU frame time, memory and draw counts |
| Concentrated battle with shadows and effects | Worst-case visible cost, transparent overdraw and effect-budget behavior |
| Mixed models/materials/animations from several packages | Batching and memory behavior with representative mod diversity |
| Shared versus unique destinations; repeated commands; chokepoints | Path request latency, queue depth, movement/congestion outcomes and CPU cost |
| Terrain edits while units move | Local rebuild cost, stale-result rejection and correct movement |
| Idle and active factories/belts; multiple active maps | Wake-up correctness and simulation cost independent of the camera |
| Eight clients viewing a large battle; joins/reconnects | Per-client/aggregate bandwidth, serialization/fan-out cost and snapshot spikes |
| Map switching and distant coordinates | Streaming peaks, visual precision, bounded resident assets and continued offscreen gameplay |
| Combined combat, industry, edits, scripts and networking | Interaction costs and correctness beyond isolated subsystem results |

Use representative meshes, skeletons, textures, materials, effects, and camera views; a cube-only scene is insufficient. Record content hashes, dependency versions, browser/graphics backend, client CPU/GPU, server machine configuration, tick/update rates, and scenario parameters. Test a documented modest desktop/laptop client as well as a stronger GPU; exact hardware and frame-rate targets remain open.

Measure server tick distribution including p95/p99 and spikes, client CPU/GPU frame times, memory peaks, download/streaming sizes, bandwidth, and hosting cost. Set concrete pass/fail thresholds before claiming supported capacity. A headless GCP run cannot validate browser rendering; browser crowd throughput cannot validate server simulation. No benchmark was run for this document update.

### 13.9 Reuse and prototype selection gates

Start from stock Bevy shared assets/batching and representative animated scenes, profile the CPU and GPU separately, then compare the baked/instanced path where it addresses demonstrated costs. Keep room for an RTS-specific renderer without committing to unnecessary custom infrastructure. Navigation, particles, floating-origin support, and replication candidates remain replaceable behind engine-owned interfaces.

Before selecting dependencies, pin a mutually compatible stack; inspect required licenses/notices; verify WASM and headless applicability; and test the required behavior with our scenarios. Final animation format, crowd renderer, navigation backend, WebGPU/fallback policy, update cadence, numeric budgets, and client minimums remain engineering decisions. Sources above document techniques or inspected candidate behavior, not a complete code audit or an endorsement of untested capacity.

## 14. Proposed validation milestones

These milestones organize implementation risk; they are not a committed delivery schedule.

| Milestone | Demonstration | Acceptance evidence |
| --- | --- | --- |
| M1 — Shared foundation | Browser client, authoritative headless server, one mod-defined unit | Browser joins a match and presents server results; secondary executable interoperability follows its selected packaging approach |
| M2 — Mod API | Default mod plus alternative economy mod | Traditional global-balance economy and physical-item economy work without privileged default-game code |
| M3 — Industrial loop | Mine → belt → factory → inserter → ammunition turret → research upgrade | Inputs/outputs remain consistent under congestion, interruption, and cancellation |
| M4 — Terrain and geology | Excavate ore-bearing ridge and use opened passage | Correct resource yield, depleted deposits stay depleted, navigation updates locally |
| M5 — Discovery | Hidden deposit, exploratory digging, research-unlocked scanner | Each player sees only permitted knowledge; chosen networking model respects secrecy goals |
| M6 — Underground utilities | Utility routes, surface ports, crossings, underground overlay | Crossings do not accidentally connect; terrain edits alter burial without moving infrastructure |
| M7 — Integrated workload | Combat, factories, belts, edits, utilities, scripts on both targets | Measured performance against agreed budgets; no accounting or synchronization failures |
| M8 — Discovery and automatic setup | Alternate index, advertised mod, missing dependencies, side-by-side versions | Join works without manual installation; bad/incompatible content fails cleanly |
| M9 — Multi-room lifecycle | Multiple rooms, password, creator settings, manual/automatic start | Isolated configuration and state; server validation; start policies supplied by mods |
| M10 — Slots without spawn assumptions | Start with empty slots, late claim, block/reopen/close, disconnect/reconnect | Atomic ownership; no recycling; a test mod claims slots without spawning entities |
| M11 — Spectator isolation | Many simulated viewers, late viewer, slow/failing relay | Checkpoint catch-up works; player-slot counts unchanged; simulation cost stays within export budget |
| M12 — AI and worst-case scale | Tens of thousands of units, concentrated battles, high-rate AI commands | Measured CPU/memory/bandwidth; bounded work and fair technical admission under stress |
| M13 — Compiled assets and animated crowds | Representative default/mod assets through validation, compilation, shared rendering and animation detail paths | Browser CPU/GPU timings, correct culling/shadows/attachments/picking, bounded effects, and reproducible assets; compare stock and specialized paths |

Run the early rendering/navigation probes from Section 13.8 alongside foundation work; their position in this table does not defer performance validation until all other milestones are complete.

Representative combined scenario: workers excavate an ore-bearing ridge; belts carry ore to a factory; inserters supply a turret; a lab unlocks a detector; an army uses the opened passage; underground power or water serves the production site. Excavation over a buried line triggers the selected exposure policy.

Additional proposed validation: locked transitive libraries and conflict diagnostics; seeded generator reproducibility; two-map switching with activity continuing off-screen; server save/restart using exact package versions; playable default mod against a conventional bot; agent-readable diagnostic scenarios. These extend the milestone list, not a commitment to implement every subsystem before the first playable build.

## 15. Open decisions and dependencies

| Topic | What must be decided | Why it matters |
| --- | --- | --- |
| Game scale | Initial workloads and 4-vCPU/8-GB test budget settled; acceptance thresholds, detailed scenario mixes, map dimensions and ultimate capacity remain open | Determines measurable performance claims |
| Presentation | 3D/mod control and compiled-asset direction settled; select importer features, animation formats, crowd path, camera/lighting API and terrain appearance | Determines asset compatibility, readability and rendering cost |
| Platform matrix | Desktop browsers/keyboard/mouse first; mobile later settled. WebGPU requirement versus WebGL fallback, exact browsers, minimum client hardware and executable OS targets remain open | Defines graphics capabilities and validation requirements |
| Navigation and residency | Select routing/steering components, region invalidation, coordinate representation, sparse storage and streaming budgets | Large coordinate ranges alone do not establish scalable worlds |
| Networking | Authoritative server is settled; select library, transport, schemas, update policies and dependency versions | Drives bandwidth, secrecy, hosting and reconnect implementation |
| Scripts | Runtime, sandbox, budgets, native/WASM consistency | Defines mod ergonomics and safety/performance boundaries |
| Terrain grid | Horizontal resolution, vertical steps, chunk size, slope rules | Determines edit precision, navigation, memory, visuals |
| Production API | Express mod-selected reservations, payment, refunds and blocked outputs; do not impose one universal policy | Preserves accounting and mod freedom |
| Mod packaging | Concrete format/resolver, registry API and namespaces, overrides, trust, cache; registry topology and dependency locks settled. Open formats: content hash algorithm, identifier format, version scheme and constraint syntax, package kind values (including a cosmetic-only kind, Section 5.4), initial capability names | Required for a usable ecosystem |
| Utility scope | Initial network types and simulation models | Prevents a generic graph from hiding unresolved simulation work |
| Persistence | Server saves settled; serialization, recovery and reconnect implementation open; replay/migration secondary | Determines restoration contracts |
| Tools | Agent-first workflow settled; create tools iteratively from observed authoring/debug needs | Enables verifiable autonomous development |
| Default game | Roster and basic rules settled; names/art, numeric balancing, timings, placement constraints and edge cases remain | Constrains the first playable release |

Additional open implementation choices: authentication protocol and key/session encoding; index API/versioning; content trust and caching; loading reservations and claim boundaries; room worker isolation; exact administration transfer/timeout behavior; spectator feed format and deployment; dynamic mod-state serialization. Portable identity and room-administration defaults are settled. The ownership and gameplay-policy decisions above are settled and should not be treated as open merely because implementation remains to be designed.

## 16. Agent-first development and initial game

### 16.1 Development workflow

**Decided:** AI development agents are expected to build the engine and mods. Tooling should emerge from their real implementation, testing, debugging, and profiling needs. Agents should track missing capabilities and create/test tools while building the game. Do not precommit to a large manual editor suite.

**Proposed initial foundations:** headless scenario execution, machine-readable API/package schemas, structured diagnostics, state inspection, and performance reports. These are recommendations, not a selected tool stack. Runtime game bots are separate from development agents.

### 16.2 First playable default mod

**Decided:** a simple C&C-style game to validate basic mechanics, with original content and one faction:

| Content | Initial scope |
| --- | --- |
| Human infantry | Rifleman, anti-vehicle soldier, engineer |
| Resource | One type collected from ore fields |
| Harvester | One gathering vehicle type |
| Armor | One tank type |
| Defense | One turret type |
| Base | Command center |
| Power | Electric/power station |

**Decided rules for this default mod only:**

| Mechanic | Initial behavior |
| --- | --- |
| Engineer | Repairs buildings and vehicles |
| Harvesting | Harvester collects ore and delivers it to the command center for credits |
| Command center | Produces units and manages building construction |
| Construction | Pay credits, select a valid site, then construction progresses over time |
| Electricity | Power stations supply electricity; insufficient power slows production and disables turrets |
| Combat orders | Move, attack, attack-move, and automatic targeting |
| Victory | Last player or team with a surviving command center wins |
| Starting assets | One command center, one harvester, and enough credits to establish power |

These policies are mod-owned and replaceable, not mandatory engine rules. Concentrating production/delivery in the command center avoids additional initial building types. Dedicated barracks, factories, and refineries may be added later. Names/art, statistics, timings, costs, repair accounting, construction restrictions, power-shortage multiplier, and simultaneous-destruction edge cases remain to be specified during implementation. Industry, research, excavation, utilities, and multiple-map demonstrations may be separate follow-on scenarios; their engine scope remains intact.

## 17. Conventional AI opponents

**Decided:** mods must be able to offer basic conventional CPU-controlled opponents for playable games and manual testing. They need no GPU, LLM, or other model inference and need only be competent enough for initial testing. External model-based opponents remain possible through the player protocol.

### 17.1 Engine, library, and mod responsibilities

**Decided architecture:** reusable modular controllers, authored opening plans, reactive priorities, and small tactical state machines. Libraries provide the decision machinery; mods provide game knowledge, unit/building roles, economic requirements, strategic priorities, and composition targets. Mods may replace the entire opponent or reuse selected pieces. The engine does not prescribe a strategy and a universal bot is not expected to infer arbitrary mechanics automatically.

**Baseline execution integration:** bots run on the authoritative server through bounded observation and validated command facilities. Normal movement, targeting, combat, and other high-volume order execution remain in engine systems. Strategic decisions need not run every simulation tick. Exact scheduling and resource budgets require benchmarks. This architecture requires no GPU, model inference, training, or sophisticated search.

### 17.2 Controller composition

| Controller | Responsibility |
| --- | --- |
| Opening/strategy | Follow mod-authored opening plans and conditional priorities; allow recovery to override the opening |
| Economy/construction | Maintain mod-defined income and operating conditions, replace losses, propose construction |
| Production | Fill mod-defined desired army composition subject to prerequisites and available resources |
| Squad/task management | Allocate units to scouting, defense, assembly and attack missions |
| Tactical state machines | Assemble, move, attack, defend, regroup or retreat based on mod-supplied conditions |
| Shared coordinator | Resolve competing spending requests and unit assignments using priorities, budgets/reservations, and pending tasks |

**Decided:** coordination is essential. Economy, production, and construction must not independently spend the same resources or issue conflicting assignments. A destroyed harvester can trigger recovery rather than leaving an opening script blindly pursuing its next step. Controller requests ultimately pass through the engine's transaction and command validation mechanisms.

For the first mod, the accepted behavioral direction is to maintain power and harvesting, replace harvesters, produce an army, defend, scout as needed, gather squads, attack, and recover. Exact thresholds, composition, tactical sophistication, and difficulty settings remain mod implementation choices.

### 17.3 Knowledge, profiles, and diagnostics

Baseline recommendation: use ordinary command validation and visibility-limited observations, with remembered past sightings distinct from current enemy truth. Any special knowledge or economic advantages must be explicit mod policy. Detailed observation/memory API, difficulty design, named profiles, and bot registration remain open; creators selecting mod-provided bots for unclaimed lobby slots is decided.

Proposed development aids: pending-task inspection, rejected-command reasons, decision traces, and headless bot scenarios. Testing controllers with privileged world-edit access are distinct from ordinary opponents. A single balanced profile is a proposed first implementation; support for additional profiles must not force all mods to use the same strategies.

### 17.4 Participants and slots

Creators may assign mod-provided bots to unclaimed lobby slots. Bot controller identity and runtime representation remain implementation details; human-owned match slots must not be recycled. Lobby removals are pre-match administration and do not reopen claimed slots in a running match. Optional temporary control of a disconnected human's assets remains mod policy; no automatic human-to-bot takeover is imposed.

## 18. Next design step

The core product decisions are sufficient to begin prototypes: identity/recovery, registry topology, desktop-first delivery, initial mod rules, administration defaults, bot architecture, and hosting benchmarks are settled. The research-backed baseline now separates simulation/network/render data, introduces compiled assets, and specifies scalable animation, navigation, work reduction, and residency directions (Sections 4.4, 5.8 and 13.3–13.9). Build early browser crowd and navigation probes with representative content, compare stock and specialized paths, and test the combined workload. Select scripting/networking components and the browser graphics baseline through these prototypes; define measurable acceptance thresholds and implement the public APIs against the first mod. GCP VM setup/monitoring can proceed when the promised service-account access and concrete test configuration are available. Terrain precision, serialization, scheduling, dependency selection, and numeric budgets remain engineering selections to validate, not universal gameplay policies.

Reference designs: [Blizzard's original StarCraft AI documentation](https://classic.battle.net/scc/faq/aiscripts.shtml), [StarCraft II authored build options](https://news.blizzard.com/en-gb/article/10053042/patch-2-0-4-now-live), and [OpenRA bot module documentation](https://docs.openra.net/en/release/traits/). OpenRA's modular approach is an architectural reference, not a decision to import its code or require its engine. The original C&C team/trigger approach informs squad missions; it is not assumed to describe every C&C title. Code reuse requires separate dependency/license evaluation.

### Revision history

- **0.1:** initial architecture through terrain, geology, and underground infrastructure.
- **0.2:** configurable discovery, automatic mods, multi-room servers, creator settings, late admission, permanent slot ownership, slots independent of spawning, mod-owned gameplay policy, scale targets, networking shortlist, and spectator distribution. Rejected slot recycling and engine-imposed spawn eligibility rules; retained implementation recommendations as nonfinal.

- **0.3:** publishable libraries and dependency locks; shared scripting direction; library-owned seeded generation; browser-first 3D presentation; multiple large maps; authoritative economical hosting and established-technique reuse; server-owned SQLite/persistence boundaries; agent-first tools; initial C&C-style roster; conventional bot requirement with detailed design still proposed.
- **0.4:** accepted modular conventional opponent architecture, authored openings and reactive recovery, game knowledge supplied by mods, shared resource/unit coordination, and tactical state machines. Retained identity, profiles, knowledge APIs, difficulty, and other integration specifics as open or baseline proposals.

- **0.5:** portable identity and user-key recovery; one controller per claimed slot; registries and separate publisher keys; desktop-first device scope; validated default-mod rules; room administration; initial small/medium/stress benchmarks and 4-vCPU/8-GB allocation; planned user-provided GCP service account for VM setup/monitoring.
- **0.6:** incorporated researched asset compilation, scalable rendering/animation, simulation work reduction, hierarchical/shared navigation plus local steering, compact replication, coordinate/residency separation, candidate components and primary-source links. Expanded browser/server and combined-workload benchmarks; retained unselected libraries, graphics fallback policy and numeric targets as open. No prototype or benchmark results are claimed.

- **0.7:** merge of the studio's 2026-10-26 revision onto 0.6: two package documents named, the package manifest (field list owned by 5.5) and the server lockfile (one entry per package, hashes live there only); the 6.5 workflow takes the server's lockfile instead of resolving versions, checks execution context, and verifies hashes before extraction; side-by-side versions named as a cache property (D35); client-side acquisition split between multiplayer and mod runtime (4.1); open manifest formats listed in Section 15; 6.4 names the server advertisement. The studio agents found the original defects in the first gate review of 6.5.

---

Source of record: the project conversation and the user's request to incorporate the subsequent large-scale 3D/RTS research. Version 0.7 preserves prior decisions and records the research recommendations as a working baseline, with unresolved choices explicitly identified. Linked primary sources support techniques and candidate behavior; they do not establish this project's capacity. Test budgets are not measured guarantees; no benchmarks, cloud access verification, or resource provisioning were performed by this update.
