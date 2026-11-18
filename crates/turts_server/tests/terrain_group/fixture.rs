use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    process::Command as Process,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use super::{
    boot,
    config::HostConfig,
    orders,
    physical::{self, Body, SurfaceState},
};
use bevy::app::App;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use turts_mod_api::ContentCatalog;
use turts_mod_runtime::{archive::PackageArchive, hash::sha256_hex, pack::pack_dir};
use turts_net_protocol::{OrderEnvelope, PlayerCommand};
use turts_sim::{
    CommandQueue, CommandResults, Order, SlotId, Tick, UnitOrders, Vec2, completed_world,
    headless_app, initialize_completed_world, snapshot, unit_entity,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default)]
pub struct Variant {
    pub allocation: bool,
    pub members: bool,
}
pub const VARIANTS: [Variant; 4] = [
    Variant {
        allocation: false,
        members: false,
    },
    Variant {
        allocation: true,
        members: false,
    },
    Variant {
        allocation: false,
        members: true,
    },
    Variant {
        allocation: true,
        members: true,
    },
];
#[derive(Clone, Deserialize)]
pub struct Actor {
    pub label: String,
    pub unit: u32,
    pub content: String,
    pub slot: u8,
    pub kind: String,
    pub position: [f32; 2],
    pub profile: Profile,
}
#[derive(Clone, Deserialize)]
pub struct Profile {
    pub radius: f32,
    pub speed: f32,
    pub max_step: u32,
    pub max_grade: [u64; 2],
}
#[derive(Clone, Deserialize)]
pub struct Scheduled {
    pub tick: u32,
    pub slot: u8,
    pub envelope: OrderEnvelope,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicalRow {
    pub label: String,
    pub position: [u32; 2],
    pub current: Option<Order>,
    pub queued: Vec<Order>,
}
/// Actor handles are normalized only through frozen physical labels. Positions and order
/// endpoints retain their exact f32 bits. No movement or arrival difference is discarded.
pub type Frame = Vec<PhysicalRow>;
#[derive(Default)]
struct Timings {
    enabled: bool,
    app_ns: u128,
    physical_ns: u128,
    diagnostics_ns: u128,
    step_ns: u128,
}
pub struct Fixture {
    timings: Timings,
    pub app: App,
    pub actors: Vec<Actor>,
    pub commands: Vec<Scheduled>,
    pub scene: Value,
    pub catalog: ContentCatalog,
    pub tick: u32,
    pub scene_name: String,
    pub package: String,
    pub surface: SurfaceState,
    seq: [u32; 8],
    settled: BTreeMap<String, [u32; 2]>,
    pub trace: Vec<Frame>,
    dir: PathBuf,
    expected: BTreeMap<(u8, u32), Vec<u32>>,
    submitted: Vec<Value>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.write_debug_state("failure-state.json");
        }
        // Generated assets are hard links to existing public bytes. Keep only small provenance.
        let _ = fs::write(
            self.dir.join("submitted-commands.json"),
            serde_json::to_vec_pretty(&self.submitted).unwrap(),
        );
        let _ = fs::remove_dir_all(self.dir.join(&self.package));
    }
}
impl Fixture {
    pub fn new(package: &str, scene: &str, variant: Variant) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let evidence = std::env::var_os("TURTS_TERRAIN_GROUP_EVIDENCE")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("turts-terrain-group-acceptance"));
        fs::create_dir_all(&evidence).unwrap();
        let dir = evidence.join(format!(
            "{scene}-{package}-a{}-m{}-{}-{serial}",
            variant.allocation as u8,
            variant.members as u8,
            std::process::id()
        ));
        let mut generator = Process::new("python3");
        generator
            .arg(root.join("tests/browser/terrain-group/generate.py"))
            .arg("--root")
            .arg(&root)
            .arg("--package")
            .arg(package)
            .arg("--scene")
            .arg(scene)
            .arg("--output")
            .arg(&dir);
        if variant.allocation {
            generator.arg("--reverse-allocation");
        }
        if variant.members {
            generator.arg("--reverse-members");
        }
        let output = generator
            .output()
            .expect("run frozen public fixture generator");
        assert!(
            output.status.success(),
            "generator {scene}/{package}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let frozen: Value =
            serde_json::from_slice(&fs::read(dir.join("fixture.json")).unwrap()).unwrap();
        let command_bytes = fs::read(dir.join("commands.json")).unwrap();
        assert_eq!(frozen["commands_sha256"], generator_digest(&command_bytes));
        assert_eq!(
            frozen["source_scenes_sha256"],
            generator_digest(
                &fs::read(root.join("tests/browser/terrain-group/scenes.json")).unwrap()
            )
        );
        for (file, hash) in frozen["public_files_sha256"].as_object().unwrap() {
            assert_eq!(
                *hash,
                generator_digest(&fs::read(dir.join(package).join(file)).unwrap()),
                "public file {file}"
            );
        }
        let bytes = pack_dir(&dir.join(package)).expect("pack authored public package");
        let archive = PackageArchive::parse(&bytes).expect("parse public archive");
        turts_pack::check_package(&archive).expect("normal typed archive validation");
        let catalog = archive
            .content()
            .expect("normal final public content catalog");
        fs::write(dir.join("seed.txt"), "3457144321\n").unwrap();
        let config = HostConfig {
            terrain_seed_file: Some(dir.join("seed.txt").display().to_string()),
            terrain_provenance: Some(dir.join("private-provenance.json").display().to_string()),
            game_package: dir.join("package.tar").display().to_string(),
            http_listen: "127.0.0.1:0".into(),
            ws_listen: "127.0.0.1:0".into(),
            public_url: "http://127.0.0.1:7000".into(),
            tick_hz: 20,
            static_dir: None,
            start_policy: None,
            bot_limits: Default::default(),
            bots: vec![],
        };
        let mut app = headless_app(20);
        let booted =
            boot::boot(app.world_mut(), &config, &bytes).expect("normal seed-backed server boot");
        assert_eq!(booted.archive.hash, archive.hash);
        assert_eq!(booted.package_bytes, bytes);
        let lock = turts_mod_runtime::lock::read_lockfile(&booted.lockfile).unwrap();
        assert_eq!(lock.packages[0].hash, sha256_hex(&bytes));
        fs::write(
            dir.join("archive.sha256"),
            format!("{}\n", sha256_hex(&bytes)),
        )
        .unwrap();
        let viewers: Vec<_> = (0..turts_mod_api::SLOT_COUNT).collect();
        turts_sim::visibility::initialize_context(app.world_mut(), 0x474d0001, &viewers).unwrap();
        initialize_completed_world(app.world_mut());
        let actors: Vec<Actor> = serde_json::from_value(frozen["actors"].clone()).unwrap();
        let units = snapshot(app.world_mut());
        assert_eq!(
            units.len(),
            actors.len(),
            "no synthetic actors or hidden fixture helpers"
        );
        for actor in &actors {
            let actual = units.iter().find(|u| u.unit_id == actor.unit).unwrap();
            assert_eq!(actual.content_id, actor.content);
            assert_eq!(actual.owner_slot, actor.slot);
            assert_eq!(actual.pos, Vec2::from_array(actor.position));
        }
        let surface = SurfaceState::read(app.world());
        let mut fixture = Self {
            timings: Timings {
                enabled: std::env::var_os("TURTS_M4B_TIMERS").is_some(),
                ..Default::default()
            },
            app,
            actors,
            commands: serde_json::from_slice(&command_bytes).unwrap(),
            scene: frozen["scene"].clone(),
            catalog,
            tick: 0,
            scene_name: scene.into(),
            package: package.into(),
            surface,
            seq: [0; 8],
            settled: BTreeMap::new(),
            trace: Vec::new(),
            dir,
            expected: BTreeMap::new(),
            submitted: Vec::new(),
        };
        fixture.verify_initial();
        fixture
    }
    pub fn actor(&self, label: &str) -> &Actor {
        self.actors.iter().find(|a| a.label == label).unwrap()
    }
    pub fn position(&self, label: &str) -> Vec2 {
        let entity = unit_entity(self.app.world(), self.actor(label).unit).unwrap();
        self.app
            .world()
            .get::<turts_sim::Position>(entity)
            .unwrap()
            .0
    }
    pub fn order(&self, label: &str) -> Option<Order> {
        let entity = unit_entity(self.app.world(), self.actor(label).unit).unwrap();
        self.app
            .world()
            .get::<UnitOrders>(entity)
            .and_then(|o| o.current.clone())
    }
    pub fn complete(&self, label: &str) -> bool {
        let entity = unit_entity(self.app.world(), self.actor(label).unit).unwrap();
        let orders = self.app.world().get::<UnitOrders>(entity).unwrap();
        orders.current.is_none()
            && orders.queue.is_empty()
            && orders.last_failure.is_none()
            && self
                .app
                .world()
                .get::<turts_sim::MoveTarget>(entity)
                .is_none()
    }
    pub fn labels(&self, filter: impl Fn(&Actor) -> bool) -> Vec<String> {
        self.actors
            .iter()
            .filter(|a| filter(a))
            .map(|a| a.label.clone())
            .collect()
    }
    pub fn envelope(
        &self,
        order: &str,
        labels: &[String],
        target: turts_net_protocol::OrderTarget,
        queue: turts_net_protocol::QueueMode,
        group: u32,
    ) -> OrderEnvelope {
        OrderEnvelope {
            order: format!("{}:{order}", self.package),
            units: labels
                .iter()
                .map(|l| turts_net_protocol::UnitId(self.actor(l).unit))
                .collect(),
            target,
            queue,
            group,
        }
    }
    pub fn send(&mut self, slot: u8, envelope: OrderEnvelope) {
        for id in &envelope.units {
            if let Some(actor) = self.actors.iter().find(|a| a.unit == id.0) {
                self.settled.remove(&actor.label);
            }
        }
        self.seq[slot as usize] += 1;
        let command = PlayerCommand {
            seq: self.seq[slot as usize],
            order: envelope,
        };
        self.expected.insert(
            (slot, command.seq),
            command.order.units.iter().map(|id| id.0).collect(),
        );
        self.submitted
            .push(serde_json::json!({"tick":self.tick,"slot":slot,"command":command}));
        let resolved = orders::to_sim(SlotId(slot), &command, &self.catalog)
            .expect("ordinary server order resolution");
        assert_eq!(resolved.len(), 1);
        self.app
            .world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .extend(resolved);
    }
    fn bodies(&self) -> Vec<Body> {
        self.actors
            .iter()
            .map(|a| Body {
                label: a.label.clone(),
                position: self.position(&a.label),
                radius: a.profile.radius,
                max_step: a.profile.max_step,
                max_grade: a.profile.max_grade,
                speed: a.profile.speed,
            })
            .collect()
    }
    fn verify_initial(&mut self) {
        physical::verify(
            &self.scene_name,
            0,
            &self.bodies(),
            &self.bodies(),
            &self.surface,
        );
        assert_eq!(self.app.world().resource::<Tick>().0, 0);
    }
    pub fn frame(&self) -> Frame {
        let mut rows: Vec<_> = self
            .actors
            .iter()
            .map(|actor| {
                let e = unit_entity(self.app.world(), actor.unit).unwrap();
                let orders = self.app.world().get::<UnitOrders>(e).unwrap();
                PhysicalRow {
                    label: actor.label.clone(),
                    position: self.position(&actor.label).to_array().map(f32::to_bits),
                    current: orders.current.clone(),
                    queued: orders.queue.iter().cloned().collect(),
                }
            })
            .collect();
        rows.sort_by(|a, b| a.label.cmp(&b.label));
        rows
    }
    pub fn timer_start(&self) -> Option<Instant> {
        self.timings.enabled.then(Instant::now)
    }
    pub fn record_diagnostics_time(&mut self, start: Option<Instant>) {
        if let Some(start) = start {
            self.timings.diagnostics_ns += start.elapsed().as_nanos();
        }
        if self.timings.enabled && self.tick.is_multiple_of(64) {
            println!(
                "GM_TIMER {}",
                serde_json::json!({
                    "scope":"functional attribution; not capacity timing",
                    "package":self.package,"scene":self.scene_name,"fixture":self.dir,"ticks":self.tick,
                    "app_ns":self.timings.app_ns,"physical_ns":self.timings.physical_ns,
                    "diagnostics_ns":self.timings.diagnostics_ns,
                    "other_step_ns":self.timings.step_ns.saturating_sub(self.timings.app_ns).saturating_sub(self.timings.physical_ns),
                    "total_ns":self.timings.step_ns+self.timings.diagnostics_ns
                })
            );
        }
    }
    pub fn step(&mut self) {
        let total_start = self.timer_start();
        for command in self
            .commands
            .iter()
            .filter(|c| c.tick == self.tick)
            .cloned()
            .collect::<Vec<_>>()
        {
            self.send(command.slot, command.envelope);
        }
        let before = self.bodies();
        let app_start = self.timer_start();
        self.app.update();
        if let Some(start) = app_start {
            self.timings.app_ns += start.elapsed().as_nanos();
        }
        self.tick += 1;
        assert_eq!(
            self.app.world().resource::<Tick>().0,
            self.tick,
            "one actual fixed tick per update"
        );
        assert_eq!(completed_world(self.app.world()).tick, self.tick);
        for result in &self.app.world().resource::<CommandResults>().0 {
            let mut expected = self
                .expected
                .remove(&(result.slot.0, result.seq))
                .expect("one result for each submitted envelope");
            expected.sort_unstable();
            let mut accepted = result.units.accepted.clone();
            accepted.sort_unstable();
            assert_eq!(
                accepted, expected,
                "every selected actor must be admitted once"
            );
            assert!(
                result.outcome.is_ok(),
                "{} tick{} command refusal: {result:?}",
                self.scene_name,
                self.tick
            );
            assert!(
                result.units.refused.is_empty(),
                "{} tick{} per-member refusal: {result:?}",
                self.scene_name,
                self.tick
            );
            assert!(
                !result.units.accepted.is_empty(),
                "ordinary fixture order must accept its actual members"
            );
        }
        assert!(
            self.expected.is_empty(),
            "submitted command vanished before fixed-tick admission"
        );
        let after = self.bodies();
        // Movement precedes TerrainWork in the real fixed schedule. Certify against the
        // pre-work surface; a later same-tick edit cannot legalize an earlier wall crossing.
        let physical_start = self.timer_start();
        physical::verify(&self.scene_name, self.tick, &before, &after, &self.surface);
        if turts_sim::terrain::privileged_changed_chunk_count(self.app.world()) > 0 {
            self.surface = SurfaceState::read(self.app.world());
            physical::verify_support(&self.scene_name, self.tick, &after, &self.surface);
        }
        if let Some(start) = physical_start {
            self.timings.physical_ns += start.elapsed().as_nanos();
        }
        for actor in &self.actors {
            let point = self.position(&actor.label).to_array().map(f32::to_bits);
            if let Some(previous) = self.settled.get(&actor.label) {
                assert_eq!(
                    *previous, point,
                    "{} tick{} displaces settled {}",
                    self.scene_name, self.tick, actor.label
                );
            }
            // Only remember a real accepted/completed movement, not an uncommanded idle unit.
            if self.complete(&actor.label)
                && before
                    .iter()
                    .zip(&after)
                    .any(|(a, b)| a.label == actor.label && a.position != b.position)
            {
                self.settled.insert(actor.label.clone(), point);
            }
        }
        self.trace.push(self.frame());
        if self.tick.is_multiple_of(128) && std::env::var_os("TURTS_M4B_TRACE").is_some() {
            self.write_debug_state("debug-last.json");
        }
        if let Some(start) = total_start {
            self.timings.step_ns += start.elapsed().as_nanos();
        }
    }
    fn write_debug_state(&self, name: &str) {
        let world = self.app.world();
        let rows = world
            .iter_entities()
            .filter_map(|entity| {
                let id = entity.get::<turts_sim::UnitId>()?.0;
                let label = self
                    .actors
                    .iter()
                    .find(|actor| actor.unit == id)
                    .map(|actor| actor.label.as_str());
                Some(serde_json::json!({"unit":id,"label":label,
                    "position":entity.get::<turts_sim::Position>().map(|p|p.0.to_array()),
                    "target":entity.get::<turts_sim::world::MoveTarget>().map(|p|p.0.to_array()),
                    "group_progress":entity.get::<turts_sim::movement::MoveProgress>(),
                    "plain_progress":entity.get::<turts_sim::movement::PlainMoveProgress>(),
                    "physical_escape":entity.get::<turts_sim::movement::PhysicalEscape>(),
                }))
            })
            .collect::<Vec<_>>();
        let value = serde_json::json!({"diagnostic_only":true,"scene":self.scene_name,"tick":self.tick,
            "traffic":turts_sim::terrain::privileged_traffic_debug(world),"contacts":turts_sim::terrain::privileged_contact_debug(world),
            "units":rows,"owner_orders":turts_sim::completed_world(world).unit_orders,"navigation":format!("{:?}",turts_sim::terrain::privileged_navigation_audit(world)),
            "trace_tail":&self.trace[self.trace.len().saturating_sub(20)..]});
        if let Ok(bytes) = serde_json::to_vec_pretty(&value) {
            let _ = fs::write(self.dir.join(name), bytes);
        }
    }
    pub fn require_complete(&self, labels: &[String]) {
        for label in labels {
            assert!(
                self.complete(label),
                "{}/{} tick{}: {label} did not finish: {:?} at {:?}",
                self.package,
                self.scene_name,
                self.tick,
                self.order(label),
                self.position(label)
            );
        }
    }
    pub fn write_movement_capture(&self) -> PathBuf {
        let path = self.dir.join("movement-trace.json");
        fs::write(&path, serde_json::to_vec(&self.trace).unwrap()).unwrap();
        path
    }
    pub(super) fn evidence_dir(&self) -> &std::path::Path {
        &self.dir
    }
    pub fn write_result(&self, result: Value) {
        fs::write(self.dir.join("native-result.json"),serde_json::to_vec_pretty(&serde_json::json!({
            "status":"CHECKED", "scene":self.scene_name,"package":self.package,"ticks":self.tick,
            "final":self.trace.last().map(|_|self.actors.iter().map(|a|(a.label.clone(),self.position(&a.label).to_array().map(f32::to_bits))).collect::<BTreeMap<_,_>>()),
            "trajectory_sha256":sha256_hex(&serde_json::to_vec(&self.trace).unwrap()),
            "submitted_commands_sha256":sha256_hex(&serde_json::to_vec(&self.submitted).unwrap()),
            "checks":result
        })).unwrap()).unwrap();
    }
}

// Python's frozen fixture uses bare hex. Package-lock hashes keep their protocol prefix.
fn generator_digest(bytes: &[u8]) -> String {
    sha256_hex(bytes)
        .strip_prefix("sha256:")
        .expect("canonical SHA256 prefix")
        .to_owned()
}
