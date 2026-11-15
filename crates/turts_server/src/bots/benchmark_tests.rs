#![cfg(test)]
//! BF8: real simulation, public controllers, and the complete bounded host/admission path.
use super::*;
use std::time::{Duration, Instant};
use turts_mod_api::{
    BalanceAmount, CostSource, GroundRally, ItemStack, MapDef, MapSize, NodePlacement, Placement,
    ResolvedCost, SlotEconomyInit, SourceDef, StartOrder, StartTarget,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{economy::events::EconomyEventKind, install_package, stage_package};

const PHASES: usize = 1000;
const WARMUP: usize = 100;
const BASELINE: usize = 100;
const HEALTH: u64 = 1_000_000_000_000;
const INVENTORY: u64 = 10_000_000;
const STOCK: u64 = 500_000;
const DURATION_MS: u64 = 2_000_000;
const COST: u64 = 100_000;

#[derive(Default, Clone, serde::Serialize)]
struct Sample {
    host_admission_ns: u64,
    publication_ns: u64,
    participant_results_ns: u64,
    full_tick_ns: u64,
    ticks: u32,
}
impl Sample {
    fn phase_ns(&self) -> u64 {
        self.host_admission_ns + self.publication_ns + self.participant_results_ns
    }
}
#[derive(Resource)]
struct Meter {
    pre: Option<Instant>,
    publication: Option<Instant>,
    results: Option<Instant>,
    whole: Option<Instant>,
    current: Sample,
    samples: Vec<Sample>,
    seen_decisions: u64,
    groups: usize,
    warmup: usize,
    target: usize,
    baseline: bool,
    extracted: u64,
    measured_extracted: u64,
    measured_shots: u64,
    production_ticks: u64,
    shots: u64,
    commands: u64,
    min_units: usize,
    max_units: usize,
}
impl Meter {
    fn new(warmup: usize, target: usize, baseline: bool) -> Self {
        Self {
            pre: None,
            publication: None,
            results: None,
            whole: None,
            current: Sample::default(),
            samples: Vec::new(),
            seen_decisions: 0,
            groups: 0,
            warmup,
            target,
            baseline,
            extracted: 0,
            measured_extracted: 0,
            measured_shots: 0,
            production_ticks: 0,
            shots: 0,
            commands: 0,
            min_units: usize::MAX,
            max_units: 0,
        }
    }
    fn complete(&self) -> bool {
        self.samples.len() >= self.target
    }
}
fn nanos(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
fn start_tick(mut meter: ResMut<Meter>) {
    meter.whole = Some(Instant::now())
}
fn start_pre(mut meter: ResMut<Meter>) {
    meter.pre = Some(Instant::now())
}
fn finish_pre(world: &mut World) {
    let decision = world
        .get_resource::<BotHost>()
        .map_or(0, |host| host.decisions);
    let tick = world.resource::<Tick>().0;
    let mut meter = world.resource_mut::<Meter>();
    let elapsed = nanos(meter.pre.take().unwrap());
    let new_group = if meter.baseline {
        (tick - 1) % 20 == 0
    } else {
        decision != meter.seen_decisions
    };
    meter.seen_decisions = decision;
    if new_group {
        if meter.groups > meter.warmup && !meter.complete() {
            let finished = std::mem::take(&mut meter.current);
            meter.samples.push(finished);
        } else {
            meter.current = Sample::default()
        }
        meter.groups += 1;
    }
    meter.current.host_admission_ns += elapsed;
}
fn start_publication(mut meter: ResMut<Meter>) {
    meter.publication = Some(Instant::now())
}
fn finish_publication(mut meter: ResMut<Meter>) {
    let start = meter.publication.take().unwrap();
    meter.current.publication_ns += nanos(start)
}
fn start_results(mut meter: ResMut<Meter>) {
    meter.results = Some(Instant::now())
}
fn finish_tick(world: &mut World) {
    let finished = Instant::now();
    let count = turts_sim::completed_world(world).units.len();
    let events = turts_sim::committed_tick(world).events.iter().fold(
        (0u64, 0u64),
        |(gathered, shots), event| match &event.kind {
            EconomyEventKind::Extracted { amount, .. } => (gathered + amount, shots),
            EconomyEventKind::Fired { .. } => (gathered, shots + 1),
            _ => (gathered, shots),
        },
    );
    let production = turts_sim::completed_world(world)
        .operating
        .as_ref()
        .map_or(0, |operating| {
            operating
                .jobs
                .iter()
                .filter(|job| job.funding.is_some() && job.done > 0)
                .count()
        }) as u64;
    let commands = world
        .get_resource::<BotExecution>()
        .map_or(0, |execution| execution.generated.len()) as u64;
    let mut meter = world.resource_mut::<Meter>();
    let start = meter.results.take().unwrap();
    meter.current.participant_results_ns +=
        u64::try_from(finished.duration_since(start).as_nanos()).unwrap();
    let start = meter.whole.take().unwrap();
    meter.current.full_tick_ns += u64::try_from(finished.duration_since(start).as_nanos()).unwrap();
    meter.current.ticks += 1;
    if meter.groups > meter.warmup && !meter.complete() {
        meter.measured_extracted += events.0;
        meter.measured_shots += events.1;
    }
    meter.extracted += events.0;
    meter.shots += events.1;
    meter.production_ticks += production;
    meter.commands += commands;
    meter.min_units = meter.min_units.min(count);
    meter.max_units = meter.max_units.max(count);
}
fn install_meter(app: &mut App, baseline: bool) {
    app.insert_resource(Meter::new(
        if baseline { 0 } else { WARMUP },
        if baseline { BASELINE } else { PHASES },
        baseline,
    ));
    app.add_systems(FixedFirst, start_tick);
    app.add_systems(
        FixedPreUpdate,
        start_pre
            .after(SimSet::Construction)
            .before(super::drive)
            .before(SimSet::Commands)
            .before(session::admit_player_commands),
    );
    app.add_systems(
        FixedPreUpdate,
        finish_pre.after(SimSet::Commands).after(start_pre),
    );
    app.add_systems(
        FixedPostUpdate,
        start_publication
            .after(SimSet::Collect)
            .before(actors::publish_completed),
    );
    app.add_systems(
        FixedPostUpdate,
        finish_publication
            .after(actors::publish_completed)
            .after(start_publication),
    );
    app.add_systems(
        FixedLast,
        start_results.before(session::ParticipantTraceComplete),
    );
    app.add_systems(
        FixedLast,
        finish_tick
            .after(session::ParticipantTraceComplete)
            .after(start_results),
    );
}
fn content(local: &str) -> ContentId {
    ContentId::parse(&format!("stockyard:{local}")).unwrap()
}
fn placement(local: &str, slot: u8, x: f32, y: f32, stock: bool) -> Placement {
    Placement {
        unit: format!("stockyard:{local}"),
        owner_slot: slot,
        pos: WorldPos {
            x: x + f32::from(slot) * 256.,
            y,
        },
        contents: if stock {
            vec![ItemStack {
                item: "stockyard:wood".into(),
                amount: STOCK,
            }]
        } else {
            vec![]
        },
        facing: None,
        start_order: None,
    }
}
fn fixture(bots: bool) -> (App, serde_json::Value) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/stockyard");
    let bytes = pack_dir(&path).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut catalog = archive.content().unwrap();
    for definition in catalog.units.values_mut() {
        if let Some(health) = definition.health.as_mut() {
            health.max = HEALTH;
        }
        if definition.producer.is_some() || definition.constructor.is_some() {
            definition.inventory.as_mut().unwrap().capacity = INVENTORY;
        }
    }
    catalog.units.get_mut(&content("tower")).unwrap().provides[0].amount = 32;
    for recipe in catalog.production.values_mut() {
        recipe.duration_ms = DURATION_MS;
        recipe.payment = turts_mod_api::PaymentPolicy::ChargeOverTime;
        recipe.costs = vec![
            ResolvedCost {
                resource: content("wood"),
                source: CostSource::ProducerInventory,
                amount: COST,
            },
            ResolvedCost {
                resource: content("credits"),
                source: CostSource::SlotBalance,
                amount: COST,
            },
        ];
    }
    let profile = catalog.bots.get_mut(&content("opponent")).unwrap();
    profile.start_when = BotStartWhen::MatchRunning;
    profile
        .knowledge
        .roles
        .iter_mut()
        .find(|role| role.key == "home")
        .unwrap()
        .units = vec!["stockyard:tower".into()];
    profile
        .knowledge
        .production
        .iter_mut()
        .find(|target| target.role == "ranged")
        .unwrap()
        .count = 150;
    for squad in &mut profile.knowledge.squads {
        squad.target_roles = vec!["home".into()];
    }
    let profile_hash = turts_audit::actor_profile_hash(profile).unwrap();
    let mut map = MapDef {
        id: "bot_workload".into(),
        environment: None,
        ground: None,
        size: MapSize { x: 560., y: 224. },
        placements: vec![],
        nodes: vec![],
        slot_economy: vec![],
        belts: vec![],
    };
    for slot in 0..2 {
        for n in 0..100 {
            let x = 32. + (n % 10) as f32 * 6.;
            let y = 32. + (n / 10) as f32 * 6.;
            let mut worker = placement("worker", slot, x, y, false);
            worker.start_order = Some(StartOrder {
                order: "stockyard:order_gather".into(),
                target: StartTarget::NearestNode,
                ground_rally: GroundRally::Ignore,
            });
            map.placements.push(worker);
            map.nodes.push(NodePlacement {
                node: "stockyard:trees".into(),
                pos: WorldPos {
                    x: x + f32::from(slot) * 256. + 2.,
                    y: y + 2.,
                },
                remaining: 1_000_000_000,
            });
        }
        for n in 0..300 {
            map.placements.push(placement(
                ["soldier", "swordsman", "warrior"][n % 3],
                slot,
                120. + (n % 20) as f32 * 3.,
                110. + (n / 20) as f32 * 3.,
                false,
            ));
        }
        for n in 0..50 {
            map.placements.push(placement(
                "barracks",
                slot,
                32. + (n % 10) as f32 * 8.,
                110. + (n / 10) as f32 * 8.,
                true,
            ));
        }
        for n in 0..25 {
            map.placements.push(placement(
                "builder",
                slot,
                140. + (n % 5) as f32 * 8.,
                32. + (n / 5) as f32 * 8.,
                true,
            ));
        }
        for n in 0..25 {
            map.placements.push(placement(
                "tower",
                slot,
                200. + (n % 5) as f32 * 8.,
                32. + (n / 5) as f32 * 8.,
                false,
            ));
        }
        map.slot_economy.push(SlotEconomyInit {
            slot,
            balances: vec![BalanceAmount {
                balance: "stockyard:credits".into(),
                amount: 1_000_000_000,
            }],
        });
    }
    let authored = serde_json::json!({"fixture_version":1,"source_archive_sha256":turts_mod_runtime::hash::sha256_hex(&bytes),"profile_hash":profile_hash,"map":map,"health_max":HEALTH,"receiver_capacity":INVENTORY,"initial_inventory":STOCK,"production_duration_ms":DURATION_MS,"production_cost_per_endpoint":COST,"provider_amount":32});
    let fixture_hash = turts_mod_runtime::hash::sha256_hex(&serde_json::to_vec(&authored).unwrap());
    let mut app = turts_sim::headless_app(20);
    let staged = stage_package(
        app.world(),
        catalog.clone(),
        SourceDef {
            file: "benchmark/public-bot-workload.ron".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    install_package(app.world_mut(), staged);
    app.insert_resource(ServerCatalog(catalog));
    app.init_resource::<crate::stats::Stats>();
    app.insert_resource(crate::stats::MatchId(1));
    session::install(
        &mut app,
        session::Advertised {
            lockfile_hash: fixture_hash.clone(),
            tick_hz: 20,
        },
    );
    turts_sim::initialize_completed_world(app.world_mut());
    if bots {
        super::install(
            &mut app,
            &[
                BotConfig {
                    slot: 0,
                    profile: "stockyard:opponent".into(),
                    seed: 42,
                },
                BotConfig {
                    slot: 1,
                    profile: "stockyard:opponent".into(),
                    seed: 43,
                },
            ],
        )
        .unwrap();
    }
    install_meter(&mut app, !bots);
    (
        app,
        serde_json::json!({"fixture_sha256":fixture_hash,"profiles":[{"slot":0,"id":"stockyard:opponent","seed":42,"sha256":profile_hash},{"slot":1,"id":"stockyard:opponent","seed":43,"sha256":profile_hash}],"units_per_bot":{"gatherers":100,"mobile_combatants":300,"producers":50,"constructors":25,"static_providers":25},"limits":artifact_limits(BotLimitsConfig::default()).unwrap(),"authored":authored}),
    )
}
fn percentiles(values: impl Iterator<Item = u64>) -> serde_json::Value {
    let mut values = values.collect::<Vec<_>>();
    values.sort_unstable();
    let percentile =
        |numerator: usize| values[(values.len() * numerator).div_ceil(100).saturating_sub(1)];
    serde_json::json!({"samples":values.len(),"p50_ns":percentile(50),"p95_ns":percentile(95),"p99_ns":percentile(99),"max_ns":values.last().unwrap()})
}
fn run(mut app: App) -> (serde_json::Value, u64) {
    let deadline = Instant::now() + Duration::from_secs(1800);
    while !app.world().resource::<Meter>().complete() {
        assert!(
            Instant::now() < deadline,
            "benchmark did not complete within 30 minutes"
        );
        app.update();
        assert!(
            app.world()
                .get_resource::<BotExecution>()
                .is_none_or(|execution| execution.error.is_none()),
            "host invariant failed"
        );
    }
    let meter = app.world().resource::<Meter>();
    assert_eq!(
        (meter.min_units, meter.max_units),
        (1000, 1000),
        "authored work stays pending and no unit dies during measurement"
    );
    assert!(
        meter.samples.iter().all(|sample| sample.ticks == 20),
        "each sample must include all 20 ticks, including deferred work"
    );
    let total = percentiles(meter.samples.iter().map(Sample::phase_ns));
    let p99 = total["p99_ns"].as_u64().unwrap();
    let result = serde_json::json!({"complete_bot_phase":total,"host_plus_common_admission_and_command_apply":percentiles(meter.samples.iter().map(|s|s.host_admission_ns)),"all_deferred_receipt_publication":percentiles(meter.samples.iter().map(|s|s.publication_ns)),"participant_results":percentiles(meter.samples.iter().map(|s|s.participant_results_ns)),"full_simulation_intervals":percentiles(meter.samples.iter().map(|s|s.full_tick_ns)),"counters":{"ticks":app.world().resource::<Tick>().0,"groups":meter.groups,"gathered_items":meter.extracted,"measured_gathered_items":meter.measured_extracted,"measured_shots":meter.measured_shots,"shots":meter.shots,"cot_job_ticks":meter.production_ticks,"commands":meter.commands,"min_units":meter.min_units,"max_units":meter.max_units,"decisions":app.world().get_resource::<BotHost>().map_or(0,|host|host.decisions),"private_projection_rows":app.world().get_resource::<BotHost>().map_or(0,|host|host.private_projection_scans)}});
    if !meter.baseline {
        assert!(
            meter.measured_extracted > 0
                && meter.measured_shots > 0
                && meter.production_ticks > 0
                && meter.commands > 0
                && meter.shots > 0,
            "must execute collection, COT work, and ordinary bot commands"
        );
        let active_cot = turts_sim::completed_world(app.world())
            .operating
            .as_ref()
            .unwrap()
            .jobs
            .iter()
            .filter(|job| job.funding.is_some() && job.done > 0 && job.done < job.total)
            .count();
        assert_eq!(
            active_cot, 100,
            "all fifty producers per bot must retain real mixed-endpoint COT work"
        );
        assert!(
            app.world()
                .resource::<BotHost>()
                .bots
                .iter()
                .all(|bot| bot.clock.active > 0),
            "both bots advance without a human"
        );
    }
    (result, p99)
}
#[test]
#[ignore = "BF8 pinned-core release measurement; run with taskset on four cores and --release --ignored --exact"]
fn bf8_two_bots_1000_units_complete_phase_budget() {
    assert!(!cfg!(debug_assertions), "BF8 must use the release profile");
    let status = std::fs::read_to_string("/proc/self/status")
        .expect("BF8 requires Linux CPU affinity evidence");
    let affinity = status
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:\t"))
        .expect("Linux allowed CPU list");
    let cores = affinity
        .split(',')
        .map(|piece| {
            let (low, high) = piece.split_once('-').unwrap_or((piece, piece));
            high.parse::<usize>().unwrap() - low.parse::<usize>().unwrap() + 1
        })
        .sum::<usize>();
    assert_eq!(
        cores, 4,
        "run BF8 with taskset on exactly four selected logical CPUs"
    );
    let (baseline, _) = fixture(false);
    let (baseline, _) = run(baseline);
    let (bots, definition) = fixture(true);
    let (measured, p99) = run(bots);
    let report = serde_json::json!({"acceptance":"BF8","selected_logical_cpus":affinity,"tick_hz":20,"decision_ms":1000,"active_phases":PHASES,"warmup_phases":WARMUP,"baseline_intervals":BASELINE,"budget_ns":5_000_000,"phase_definition":"sum host, shared admission, ordinary command apply, all per-tick receipt publication and participant result work across each 20-tick interval; no baseline subtraction","baseline_definition":"same authored 1000-unit boot and gathering start orders, no bot host or automatic production","baseline_without_bots":baseline,"bots":measured,"workload":definition});
    println!(
        "BF8_MEASUREMENT {}",
        serde_json::to_string(&report).unwrap()
    );
    if let Some(directory) = std::env::var_os("TURTS_BOT_EVIDENCE_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::PathBuf::from(directory).join("bf8-measurement.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    assert!(
        p99 <= 5_000_000,
        "BF8 complete active phase p99 {p99}ns exceeds 5ms"
    );
}
