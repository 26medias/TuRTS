# M2 scope cut and acceptance evidence (producer, answer mode, 2026-10-26)

Spec v0.7. Row M2 (RTS-Engine-Project-Specifications.md:677): "Default mod plus alternative
economy mod | Traditional global-balance economy and physical-item economy work without
privileged default-game code". Section 14 is Proposed; it is not a schedule (line 672).
Dependencies marked (spec) are read from the spec; (inferred) are mine.

## 1. IN

| # | Item | Demanded by |
| --- | --- | --- |
| I1 | Abstract balances per slot: mod-defined balance types, credit/debit transactions, reject on insufficient funds | M2 row; 7.1 :396; 4.3 Economy row :157; D05 |
| I2 | Inventories on entities: mod-defined item types, capacity, filter, reservation, transfer, one accounting | M2 row; 7.2 :402-406; invariant 5 :543 |
| I3 | Surface resource node placed by the map, finite amount (16.2 "ore fields"); no buried deposit | D44, D55, 16.2 :731 |
| I4 | Gather composition: gatherer fills its own inventory (I2), returns to a mod-named drop-off. Drop-off policy is mod data: convert items to a balance (base) or store items (alt) | 16.2 Harvesting :743; 5.1 layer 2 :180; 4.2 Worker :140 |
| I5 | Unit production from a building: queue, cost, duration, mod-selected payment policy, explicit cancel/refund | 16.2 CC row :744; 7.2 :406; 7.3 :410; section 15 Production API :707 |
| I6 | Buildings as map-placed static entities with a footprint (CC, depot). No placement by players | 16.2 Starting assets :749; D27/D28 (map places, claim creates nothing) |
| I7 | Commands Gather, Produce, CancelProduction; server validation as T4 | 6.2; D18 |
| I8 | Replication of own balance, own inventories, queue progress; client labels come from package data | M2 row (no default-game code in the client); 4.3 Interface row :164 |
| I9 | Packages: base re-cut to 16.2 economy subset (rifleman, harvester, CC, ore, credits); one new alt package (physical items) | M2 demonstration |
| I10 | Schema v0.2 + ENGINE_API_VERSION bump; acme migrated as regression | 5.2; invariant 10 |
| I11 | No-content instrument derived from package data (fix of the M1 carried gap) | vault/Decisions/2026-10-26 M1 closed.md:17 |

## 2. OUT (owner)

| Item | Owner |
| --- | --- |
| Player-placed construction: site, footprint check, progress | M3 (inferred: belts and factories need placement) |
| Power station, power shortage rule | needs decision (K7): M3 via 7.3 operating conditions :410, or M6 via 11.3 Electricity "aggregate supply and demand per connected network" :521 (both inferred) |
| Combat: tank, turret, AV soldier, attack, attack-move, auto-target | M3 for the ammunition turret (inferred); M7 for combat at load (spec) |
| Engineer repair, victory condition | first-playable validation, :694, no row -> needs decision |
| Item recipes, belts, inserters, research, general hauling (7.5) | M3 (spec demonstration) |
| Buried deposits, digging yield | M4, M5 (spec) |
| Hiding other slots' balances as policy (shared vision, secrecy) | M5 (spec: "permitted knowledge") |
| Shared economy library between mods (needs dependencies; M1 rejects them, crates/turts_mod_runtime/src/manifest.rs:44) | M8 (D33, D35) |
| Rooms, match start lifecycle | M9 |
| Bots that spend money | M12 / section 17 (M1 scope note) |
| Budgets, economy at scale, GCP VM | M7, M12; D57/D58; D59 pending |
| Desktop executable | D02 CEO item, carried from M1 |
| Script runtime | see decision K1 |

## 3. Acceptance evidence (each test names the build it fails on)

E1 derived no-content (fixes R3). Tokens = every package id, local id, content id, display
name, balance type and item type read from every manifest and definition under mods/.
Grep engine crates (string literals and identifiers, word-bounded) for each token.
Red on: an engine crate containing "credits", "ore", "harvester", or the alt package id.
Instrument proof: a fixture crate with one planted token must be red; a token list of size 0 is a failure (guards an empty parse).

E2 renamed clone. Generate base' from base: new package id, every local id replaced by a random string, display names changed. Run the same scripted headless scenario on base and base'. The economy event log (tick, slot, delta, reason) is equal after id mapping.
Red on: any `if package == "base"`, a constant content id, a default balance type, a namespace privilege.

E3 role swap. Swap the payment policy and the drop-off policy between base and alt (fixture copies).
Behaviour follows the declared policy, not the package.
Red on: a policy bound to a package or to a resource kind.

E4 locality (physical economy is not a hidden global pool). Alt: ore in depot A only; Produce at depot B.
B blocks/rejects with a structured reason; after transfer to B, it produces; A's count is unchanged.
Red on: an engine that pools inventories into a per-slot balance.

E5 kind separation. In base, production changes the balance and no inventory; in alt, the slot holds zero balance types and production consumes items.
Red on: an engine that always creates a default balance.

E6 conservation (invariant 5), every tick: mined = sum(inventories) + consumed; balance = start + income - spend + refunds.
Two producers reserve the last unit in the same tick: exactly one succeeds.
Cancel at each queue stage: refund equals the declared policy.
Red on: double reservation, lost or created items, an undeclared refund.

E7 claim neutrality (T3 extended). Claiming a slot creates no entity and changes no balance or inventory.
Red on: starting assets granted on claim.

E8 validation (T4 extended). Gather with another slot's harvester; Produce at another slot's building; cancel another slot's queue.
CommandAck then CommandRejected; state unchanged. Red on: a missing owner check.

E9 client names neither economy. Browser run (T7 hook) for base and alt: the economy panel lists exactly the types the package defines, with package names.
Red on: a hard-coded label, a client that shows a balance in alt.

E10 public path only. mods/ holds no .rs and no Cargo.toml; base loads only through turts_pack::load_package; a mod-builder writes a third economy package from the docs alone and E2/E6 pass on it.
Red on: default-mod native code; a doc gap.

## 4. Order (tsort, no cycle; deps.txt)

S0 decisions -> {S1 schema, S2 ledger, S3 inventory} parallel -> S4 gather (S1,S3), S5 produce (S2,S3)
-> S6 protocol -> S7 packages -> S8 client -> S9 acceptance (E2, E9, E10 end-to-end).
First red test: E6 double reservation, headless in turts_sim, before any network work.
Owners from ownership table (docs/superpowers/specs/2026-10-25-studio-agent-team-design.md:126-171):
S1 mod-runtime-engineer (5); S2-S5 simulation-core-engineer (7); S6 network-engineer (6.2/6.3);
S7 base: game-designer (16.2), alt: mod-builder; S8 bevy-rendering-engineer (4.3 Interface row), browser run web-platform-engineer; instruments qa-lead.

## 5. Decisions M2 forces

K1 Script runtime in M2? (section 15 Scripts open; D34 language open). Options: (a) IN, M2 blocks on language selection; (b) OUT of M2 acceptance, the probe continues in parallel; the M2 design lists each rule that cannot be declarative. Rec (b). Studio call for scope. The language itself: CEO tie-break, studio recommends from probes (modder-facing, hard to reverse).
K2 Payment policies (section 15 Production API open). Rec: mod-selected enum, charge-at-queue and reserve-then-charge-at-start, explicit refund per stage; consume-over-time to M3. Studio call, recorded in vault, gated.
K3 Harvester cargo uses the inventory primitive in base. Rec yes (one mechanism, two policies). Studio call (D55 decided; this is implementation).
K4 Alt mod: new package by mod-builder from docs; acme stays M1 regression. Studio call.
K5 Balance replication default: owner-only now, mod sharing policy at M5. Studio call.
K6 Starting balance/inventory: per slot from map data at boot, not on claim (D27, D28). Studio call.
K7 Where the full 16.2 playable mod lands (construction, power, combat, engineer, victory). No section 14 row owns it (:694 additional validation). Rec: follow-on scenario after M3. CEO tie-break (product priority).
K8 D02 desktop executable: CEO, carried.
