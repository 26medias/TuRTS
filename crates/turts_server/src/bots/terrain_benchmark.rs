//! TW7 frozen terrain workload. Diagnostic runs never replace the 1,000-sample gate.
#![cfg(test)]
use super::visibility_benchmark::{self as pipeline, Harness};
use std::collections::BTreeMap;
use turts_mod_api::*;
use turts_net_protocol::OrderTarget;
use turts_sim::{CommittedTick, EconomyEventKind, completed_world};

const WARMUP: usize = 100;
const REQUIRED: usize = 1000;
const PRIVATE_SEED: u64 = 0x5457_375f_4745_4f31;
const INITIAL_ITEM: u64 = 10_000;
fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("stockyard:{local}")).unwrap()
}
fn cell_for(index: usize, slot: u8) -> TerrainCell {
    TerrainCell {
        x: 9 + 2 * (index % 10) as u32,
        y: 44 + 64 * u32::from(slot) + 2 * (index / 10) as u32,
    }
}
fn authored(fog: bool) -> pipeline::Fixture {
    let mut fixture = pipeline::authored_fixture(fog, false);
    fixture.map.id = "tw7".into();
    let mut tool_worker = fixture.catalog.units[&id("worker")].clone();
    tool_worker.id = id("tw7_worker");
    tool_worker.name = "TW7 terrain worker".into();
    tool_worker.gatherer = None;
    tool_worker.weapon = None;
    tool_worker.acquire = None;
    tool_worker.vision = Some(VisionDef { radius: 6.0 });
    tool_worker.terrain_tools = vec![id("tw7_dig"), id("tw7_fill")];
    tool_worker.terrain_cargo = Some(TerrainCargoDef {
        unload_per_second: 20,
        interaction_range: 1.0,
    });
    tool_worker.inventory = Some(ResolvedInventory {
        capacity: 20_000,
        accepts: vec![id("wood")],
    });
    fixture
        .catalog
        .units
        .insert(tool_worker.id.clone(), tool_worker);
    let mut work_order = fixture.catalog.orders[&id("order_gather")].clone();
    work_order.id = "tw7_work".into();
    work_order.behaviour = Behaviour::TerrainWork;
    work_order.target = TargetKind::TerrainDesignation;
    work_order.units = vec![id("tw7_worker").to_string()];
    fixture.catalog.orders.insert(id("tw7_work"), work_order);
    fixture.catalog.terrain_materials.insert(
        id("tw7_loam"),
        ResolvedTerrainMaterial {
            id: id("tw7_loam"),
            name: "TW7 finite loam".into(),
            surface_color: "#826346".into(),
            hardness: 1,
            work_per_world_cube: Ratio {
                numerator: 40,
                denominator: 1,
            },
            natural_outputs: vec![ResolvedTerrainResourceRate {
                resource: id("wood"),
                per_world_cube: Ratio {
                    numerator: 4,
                    denominator: 1,
                },
            }],
            fill_inputs: vec![
                ResolvedTerrainResourceRate {
                    resource: id("wood"),
                    per_world_cube: Ratio {
                        numerator: 4,
                        denominator: 1,
                    },
                },
                ResolvedTerrainResourceRate {
                    resource: id("credits"),
                    per_world_cube: Ratio {
                        numerator: 4,
                        denominator: 1,
                    },
                },
            ],
            placed_removal: TerrainPlacedRemoval::NoYield,
        },
    );
    for (name, operation) in [
        ("tw7_dig", ResolvedTerrainOperation::Excavate),
        (
            "tw7_fill",
            ResolvedTerrainOperation::Fill {
                material: id("tw7_loam"),
            },
        ),
    ] {
        fixture.catalog.terrain_tools.insert(
            id(name),
            ResolvedTerrainTool {
                id: id(name),
                name: name.into(),
                operation,
                work_per_second: 20,
                reach: 1.1,
                allowed_materials: vec![id("tw7_loam")],
                max_hardness: 1,
                output_destination: TerrainOutputDestination::Inventory,
            },
        );
    }
    let mut retained = [0; 2];
    fixture.map.placements.retain(|placement| {
        if placement.unit != id("worker").to_string() {
            return true;
        }
        let count = &mut retained[usize::from(placement.owner_slot)];
        *count += 1;
        *count <= 50
    });
    for slot in 0..2 {
        for index in 0..50 {
            let cell = cell_for(index, slot);
            fixture.map.placements.push(Placement {
                unit: id("tw7_worker").to_string(),
                owner_slot: slot,
                pos: WorldPos {
                    x: cell.x as f32 - 0.5,
                    y: cell.y as f32 + 0.5,
                },
                contents: vec![ItemStack {
                    item: id("wood").to_string(),
                    amount: INITIAL_ITEM,
                }],
                facing: None,
                start_order: None,
            });
        }
    }
    fixture.map.terrain = Some(TerrainDef {
        length_scale: 1000,
        cell_width: 1000,
        elevation_step: 250,
        columns: 128,
        rows: 128,
        chunk_width: 16,
        initial_elevation: 0,
        initial_heights: vec![
            TerrainHeightRect {
                cells: TerrainRect {
                    min: TerrainCell { x: 104, y: 42 },
                    max: TerrainCell { x: 108, y: 56 },
                },
                elevation: 2,
            },
            TerrainHeightRect {
                cells: TerrainRect {
                    min: TerrainCell { x: 104, y: 106 },
                    max: TerrainCell { x: 108, y: 120 },
                },
                elevation: 2,
            },
        ],
        floor: -8,
        ceiling: 8,
        generation: TerrainGenerationDef::Version1 {
            bands: vec![TerrainDatumBand {
                bottom: -8,
                top: 2,
                choices: vec![TerrainWeightedMaterial {
                    material: id("tw7_loam").to_string(),
                    weight: 1,
                }],
            }],
        },
    });
    assert_eq!(fixture.map.placements.len(), 1000);
    for (index, a) in fixture.map.placements.iter().enumerate() {
        for b in &fixture.map.placements[index + 1..] {
            assert!(
                (a.pos.x - b.pos.x).hypot(a.pos.y - b.pos.y) >= 0.6,
                "overlapping TW7 placements"
            );
        }
    }
    let inherited = fixture.authored;
    fixture.authored = serde_json::json!({"version":1,"inherited_sdk16":inherited,"map":fixture.map,
        "mix_per_slot":{"moving_sight":100,"moving_combat":150,"stationary_combat":100,"gatherers":50,"terrain_workers":50,"producers":50},
        "terrain_worker":{"vision_radius":6,"body_radius":0.3,"rate_per_second":20,"work_per_prism":10,"step_period_ticks":10,"initial_item":INITIAL_ITEM,"capacity":20000,"fill_item_cost":1,"fill_balance_cost":1,"natural_first_excavation_yield":1,"placed_yield":0},
        "terrain_material":{"work_per_world_cube":[40,1],"natural_item_per_world_cube":[4,1],"fill_item_per_world_cube":[4,1],"fill_balance_per_world_cube":[4,1],"placed_removal":"NoYield"},
        "finite_duration_ticks":22000,"expected_fills_per_worker":1100,"expected_final_item_per_worker":8901,
        "seed_provenance":{"source":"fixed private benchmark input","value":PRIVATE_SEED},
        "activation_ack_rtt_ms":[50,100],"resync_interval":100,"routes":"unchanged SDK16 x32/x80 every10ticks; y32/y96; ridge/tool cells outside routes"});
    fixture
}

#[derive(Clone)]
struct Worker {
    unit: u32,
    slot: u8,
    cell: TerrainCell,
    excavated: u64,
    filled: u64,
}
struct Workload {
    workers: Vec<Worker>,
    interval_steps: BTreeMap<u32, u64>,
    steps: u64,
}
impl Workload {
    fn new(harness: &Harness) -> Self {
        let view = completed_world(harness.app.world());
        let mut workers = Vec::new();
        for slot in 0..2 {
            for index in 0..50 {
                let cell = cell_for(index, slot);
                let actor = view
                    .units
                    .iter()
                    .find(|unit| {
                        unit.owner_slot == slot
                            && unit.content_id == id("tw7_worker").to_string()
                            && unit.pos.x == cell.x as f32 - 0.5
                            && unit.pos.y == cell.y as f32 + 0.5
                    })
                    .unwrap();
                workers.push(Worker {
                    unit: actor.unit_id,
                    slot,
                    cell,
                    excavated: 0,
                    filled: 0,
                });
            }
        }
        Self {
            workers,
            interval_steps: BTreeMap::new(),
            steps: 0,
        }
    }
    fn tick(&mut self, harness: &mut Harness, tick: u32, after: bool) {
        if !after {
            if !(tick - 1).is_multiple_of(10) {
                return;
            }
            let dig = ((tick - 1) / 10).is_multiple_of(2);
            for worker in &self.workers {
                harness.command(
                    worker.slot,
                    "tw7_work",
                    vec![worker.unit],
                    OrderTarget::TerrainWork(TerrainDesignation {
                        tool: id(if dig { "tw7_dig" } else { "tw7_fill" }).to_string(),
                        cells: TerrainRect {
                            min: worker.cell,
                            max: TerrainCell {
                                x: worker.cell.x + 1,
                                y: worker.cell.y + 1,
                            },
                        },
                        target_elevation: if dig { -1 } else { 0 },
                    }),
                );
            }
            return;
        }
        for event in &harness.app.world().resource::<CommittedTick>().events {
            let EconomyEventKind::TerrainStepCommitted {
                unit,
                cell,
                before,
                after,
                placed,
                inputs,
                outputs,
                ..
            } = &event.kind
            else {
                continue;
            };
            let worker = self
                .workers
                .iter_mut()
                .find(|worker| worker.unit == *unit)
                .expect("all edits belong to authored workers");
            assert_eq!(*cell, worker.cell);
            assert_eq!((before - after).unsigned_abs(), 1);
            let sum = |rows: &[turts_sim::economy::events::CostLine], resource: &ContentId| {
                rows.iter()
                    .filter(|row| &row.resource == resource)
                    .map(|row| row.amount)
                    .sum::<u64>()
            };
            if after < before {
                assert_eq!((*before, *after), (0, -1));
                assert!(inputs.is_empty());
                assert_eq!(*placed, worker.excavated != 0);
                assert_eq!(sum(outputs, &id("wood")), u64::from(worker.excavated == 0));
                worker.excavated += 1;
            } else {
                assert_eq!((*before, *after), (-1, 0));
                assert!(*placed);
                assert_eq!(sum(inputs, &id("wood")), 1);
                assert_eq!(sum(inputs, &id("credits")), 1);
                assert!(outputs.is_empty());
                worker.filled += 1;
            }
            *self.interval_steps.entry(*unit).or_default() += 1;
            self.steps += 1;
        }
    }
    fn interval(&mut self, harness: &Harness) -> serde_json::Value {
        assert_eq!(
            self.interval_steps.len(),
            100,
            "all100 real workers must commit each interval"
        );
        assert!(self.interval_steps.values().all(|steps| *steps >= 1));
        let view = completed_world(harness.app.world());
        let phase = view.terrain.as_ref().unwrap();
        for worker in &self.workers {
            let current = phase.surface(worker.slot, worker.cell).unwrap();
            assert!(current.current);
            assert_eq!(current.elevation, 0);
            let inventory = turts_sim::inventory_view(
                harness.app.world(),
                turts_sim::SlotId(worker.slot),
                worker.unit,
            )
            .unwrap();
            let actual = inventory
                .items
                .iter()
                .filter(|row| row.resource == id("wood"))
                .map(|row| row.on_hand)
                .sum::<u64>();
            assert_eq!(
                actual,
                INITIAL_ITEM + u64::from(worker.excavated > 0) - worker.filled,
                "finite item conservation; no natural reset"
            );
        }
        for slot in 0..2 {
            let fills = self
                .workers
                .iter()
                .filter(|worker| worker.slot == slot)
                .map(|worker| worker.filled)
                .sum::<u64>();
            let charged = view
                .economy
                .queues
                .iter()
                .filter(|queue| queue.owner_slot == slot)
                .flat_map(|queue| &queue.jobs)
                .flat_map(|job| &job.charged)
                .filter(|cost| cost.resource == id("credits"))
                .map(|cost| cost.amount)
                .sum::<u64>();
            let balance = turts_sim::balance_view(
                harness.app.world(),
                turts_sim::SlotId(slot),
                &id("credits"),
            )
            .unwrap();
            assert_eq!(
                balance.on_hand,
                1_000_000_000 - fills - charged,
                "fill and actual COT debits conserve finite balance"
            );
            assert_eq!(balance.reserved, 0);
        }
        let nav = turts_sim::terrain::privileged_navigation_diagnostics(harness.app.world());
        let result = serde_json::json!({"steps":self.interval_steps,"cumulative_steps":self.steps,
            "local_navigation":{"searches":nav.searches,"cache_hits":nav.cache_hits,"invalidations":nav.invalidations,"solver_retries":nav.solver_retries,"budget_waits":nav.budget_waits}});
        self.interval_steps.clear();
        result
    }
}

fn run(intervals: usize, acceptance: bool, warmup: usize) {
    if acceptance {
        assert!(!cfg!(debug_assertions));
        assert_eq!(intervals, REQUIRED);
        assert_eq!(warmup, WARMUP);
    }
    let mut rows = Vec::new();
    for fog in [true, false] {
        for rtt in [50, 100] {
            let fixture = authored(fog);
            turts_mod_runtime::terrain::validate_terrain_catalog(
                &PackageId("stockyard".into()),
                &fixture.catalog,
            )
            .unwrap();
            let resolved = turts_mod_runtime::terrain::validate_terrain_map(
                &PackageId("stockyard".into()),
                "maps/tw7.ron",
                &fixture.map,
                &fixture.catalog,
            )
            .unwrap()
            .unwrap();
            let frozen = fixture.authored.clone();
            let hash = turts_mod_runtime::hash::sha256_hex(&serde_json::to_vec(&frozen).unwrap());
            println!(
                "TW7_AUTHORED {}",
                serde_json::json!({"sha256":hash,"inputs":frozen})
            );
            let mut harness = Harness::with_fixture(
                fixture,
                fog,
                false,
                Some((
                    resolved,
                    turts_sim::terrain::PrivateTerrainSeed::new(PRIVATE_SEED),
                )),
            );
            harness.set_ack_rtt(rtt);
            harness.set_profiling(!acceptance);
            let mut workload = Workload::new(&harness);
            for index in 0..warmup {
                harness.interval_with(index, false, |h, t, after| workload.tick(h, t, after));
                workload.interval(&harness);
                if index % 10 == 0 {
                    println!("TW7_WARMUP fog={fog} rtt={rtt} interval={index}/{warmup}");
                }
            }
            let mut samples = Vec::new();
            let mut work = Vec::new();
            for index in 0..intervals {
                samples.push(
                    harness.interval_with(index, true, |h, t, after| workload.tick(h, t, after)),
                );
                work.push(workload.interval(&harness));
                if index % 25 == 0 {
                    println!("TW7_SAMPLE fog={fog} rtt={rtt} interval={index}/{intervals}");
                }
            }
            let server = samples
                .iter()
                .flat_map(|sample| sample.full_server_tick_ns.iter().copied())
                .collect::<Vec<_>>();
            let clients: [Vec<_>; 2] = std::array::from_fn(|slot| {
                samples
                    .iter()
                    .flat_map(|sample| sample.client_tick_ns[slot].iter().copied())
                    .collect()
            });
            let batches: [Vec<_>; 2] = std::array::from_fn(|slot| {
                samples
                    .iter()
                    .flat_map(|sample| sample.client_batches_ns[slot].iter().copied())
                    .collect()
            });
            let deadline = pipeline::deadline_summary(
                &server,
                [&clients[0], &clients[1]],
                [&batches[0], &batches[1]],
            );
            rows.push(serde_json::json!({"fog":fog,"activation_ack_rtt_ms":rtt,"frozen_authoring_hash":hash,"inputs":frozen,"deadline":deadline,"samples":samples,"work":work,"peak_resident_bytes":pipeline::peak_resident_bytes()}));
        }
    }
    let artifact = serde_json::json!({"acceptance":acceptance,"diagnostic_only":!acceptance,"warmup_intervals":warmup,"measured_intervals":intervals,"ticks_per_interval":20,"timer":"SDK16 direct full service; realwork order preparation charged; verification assertions excluded; all deferred ACK drains charged without extra periods","rows":rows});
    if let Some(path) = std::env::var_os("TURTS_TW7_OUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
    }
    println!("TW7_RESULT {}",artifact["rows"].as_array().unwrap().iter().map(|row|serde_json::json!({"fog":row["fog"],"rtt":row["activation_ack_rtt_ms"],"deadline":row["deadline"]})).collect::<serde_json::Value>());
    if acceptance {
        assert!(
            artifact["rows"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["deadline"]["pass"] == true),
            "TW7 failed frozen deadline budget"
        );
    }
}
#[test]
#[ignore = "release-only frozen1000 measured intervals on four recorded cores"]
fn tw7_frozen_1000_warmed_intervals() {
    run(REQUIRED, true, WARMUP);
}
#[test]
#[ignore = "short optimized diagnostic; not acceptance"]
fn diagnostic_tw7_10_intervals() {
    run(10, false, WARMUP);
}
#[test]
#[ignore = "workload, wire and conservation preflight; no performance claim"]
fn tw7_workload_preflight() {
    run(3, false, 0);
}
