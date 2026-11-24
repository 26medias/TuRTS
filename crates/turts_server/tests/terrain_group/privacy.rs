//! GM6 uses two mirrored pairs because the public contract has exactly two slots.
//! The editor owner intentionally receives different private commands. Whole-view equality
//! is required only for the protected viewer, whose authorized input is identical.
//! Both worlds use identical public archives and ordinary finite work orders. No column,
//! component, permission resource or navigation state is written after normal server boot.
//!
//! The guidance rows come from the flock audit (2026-11-23). This module creates no
//! network peers; `peer_sessions.rs` covers connection-order reversal.
//! The frozen GM1-GM5 inputs are unchanged. This dedicated GM6 map has two
//! zero-sight probes for the protected slot and one zero-sight route for the
//! editing owner, plus two sighted editors. Initial public input is identical.
use super::{
    boot,
    config::HostConfig,
    export,
    fixture::{VARIANTS, Variant},
    orders, permissions,
    physical::{self, Body, SurfaceState},
};
use bevy::app::App;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    process::Command as Process,
    sync::atomic::{AtomicU64, Ordering},
};
use turts_mod_api::{
    BalanceAmount, ContentCatalog, ItemStack, MapDef, Placement, SlotEconomyInit, TerrainCell,
    TerrainDesignation, TerrainRect, UnitDef, VisibilityDef, VisionDef, VisionSharing, WorldPos,
};
use turts_mod_runtime::{archive::PackageArchive, hash::sha256_hex, pack::pack_dir};
use turts_net_protocol::{OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode, UnitId};
use turts_sim::{
    CommandQueue, CommandResults, SlotId, Tick, Vec2, completed_world, headless_app,
    initialize_completed_world, unit_entity,
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const LOCAL_MIN: [u32; 2] = [30, 23];
const LOCAL_MAX: [u32; 2] = [31, 26];
const REMOTE_Y: u32 = 36;
const PROBE_START: u32 = 202;
const EXCAVATE_TICK: u32 = 360;
const END_TICK: u32 = 560;

pub(super) struct PrivacyWorld {
    pub(super) app: App,
    pub(super) catalog: ContentCatalog,
    package: String,
    pub(super) protected: u8,
    pub(super) tick: u32,
    pub(super) ids: BTreeMap<String, u32>,
    pub(super) surface: SurfaceState,
    seq: [u32; 2],
    expected: BTreeMap<(u8, u32), Vec<u32>>,
    pub(super) archive_hash: String,
    pub(super) dir: PathBuf,
    pub(super) wire_outbox: Option<Vec<(u8, PlayerCommand)>>,
    #[allow(
        dead_code,
        reason = "Used by the server unit-test real-session driver, not the direct integration target"
    )]
    pub(super) lockfile_hash: String,
    remote_x: u32,
    dig_tool: String,
    fill_tool: String,
}
impl Drop for PrivacyWorld {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.dir.join(&self.package));
    }
}
impl PrivacyWorld {
    fn new(package: &str, protected: u8, variant: Variant, remote_x: u32) -> Self {
        Self::with_app(package, protected, variant, remote_x, headless_app(20))
    }
    pub(super) fn with_app(
        package: &str,
        protected: u8,
        variant: Variant,
        remote_x: u32,
        mut app: App,
    ) -> Self {
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
            "GM6-{package}-v{protected}-r{remote_x}-a{}-m{}-{}-{serial}",
            variant.allocation as u8,
            variant.members as u8,
            std::process::id()
        ));
        // Reuse the unchanged public asset generator. The dedicated GM6 variant below only
        // authors a public map and vision definitions before ordinary validation/packing.
        let output = Process::new("python3")
            .arg(root.join("tests/browser/terrain-group/generate.py"))
            .arg("--root")
            .arg(&root)
            .arg("--package")
            .arg(package)
            .arg("--scene")
            .arg("GM1-wide")
            .arg("--output")
            .arg(&dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "GM6 public generator: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let frozen: Value =
            serde_json::from_slice(&fs::read(dir.join("fixture.json")).unwrap()).unwrap();
        let binding = &frozen["bindings"];
        let small = binding["small"].as_str().unwrap();
        let large = binding["large"].as_str().unwrap();
        let resource = format!("{package}:{}", binding["resource"].as_str().unwrap());
        let public = dir.join(package);
        for (name, vision) in [(small, Some(VisionDef { radius: 9.0 })), (large, None)] {
            let path = public.join(format!("units/{name}.ron"));
            let mut unit: UnitDef = ron::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            // None gives the probe zero sight. Some(radius:0) is invalid public authoring.
            unit.vision = vision;
            fs::write(path, ron::ser::to_string(&unit).unwrap()).unwrap();
        }
        let owner = 1 - protected;
        let mut placements = [
            ("protected_probe", large, protected, [24.0, 24.5], 0),
            ("protected_peer", large, protected, [4.0, 4.0], 0),
            ("other_probe", large, owner, [8.0, 40.0], 0),
            ("local_editor", small, owner, [28.5, 27.5], 12),
            ("remote_editor", small, owner, [53.5, 38.5], 4),
        ];
        if variant.allocation {
            placements.reverse();
        }
        let ids = placements
            .iter()
            .enumerate()
            .map(|(i, (label, _, _, _, _))| (label.to_string(), i as u32 + 1))
            .collect();
        let mut map: MapDef =
            ron::from_str(&fs::read_to_string(public.join("maps/field.ron")).unwrap()).unwrap();
        map.visibility = Some(VisibilityDef {
            cell_size: 1.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: Some(65536),
        });
        // Initial terrain, geology rules and catalog are equal across the pair. The two
        // remote X coordinates appear ONLY in later accepted private command envelopes.
        let inventory = binding["destination"] == "Inventory";
        map.placements = placements
            .iter()
            .map(|(_, kind, slot, position, amount)| Placement {
                unit: format!("{package}:{kind}"),
                owner_slot: *slot,
                pos: WorldPos {
                    x: position[0],
                    y: position[1],
                },
                contents: if inventory && *amount > 0 {
                    vec![ItemStack {
                        item: resource.clone(),
                        amount: *amount,
                    }]
                } else {
                    vec![]
                },
                facing: None,
                start_order: None,
            })
            .collect();
        map.slot_economy = if inventory {
            vec![]
        } else {
            vec![SlotEconomyInit {
                slot: owner,
                balances: vec![BalanceAmount {
                    balance: resource,
                    amount: 16,
                }],
            }]
        };
        fs::write(
            public.join("maps/field.ron"),
            ron::ser::to_string(&map).unwrap(),
        )
        .unwrap();
        let bytes = pack_dir(&public).expect("ordinary public GM6 archive pack");
        let archive = PackageArchive::parse(&bytes).unwrap();
        turts_pack::check_package(&archive).expect("ordinary public GM6 archive validation");
        let catalog = archive.content().unwrap();
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
        let booted = boot::boot(app.world_mut(), &config, &bytes)
            .expect("normal public GM6 seed-backed boot");
        turts_sim::visibility::initialize_context(app.world_mut(), 0x474d0006, &[0, 1]).unwrap();
        initialize_completed_world(app.world_mut());
        let surface = SurfaceState::read(app.world());
        let archive_hash = sha256_hex(&bytes);
        let files: BTreeMap<_, _> = frozen["public_files_sha256"]
            .as_object()
            .unwrap()
            .keys()
            .map(|name| {
                (
                    name.clone(),
                    sha256_hex(&fs::read(public.join(name)).unwrap()),
                )
            })
            .collect();
        fs::write(dir.join("privacy-fixture.json"),serde_json::to_vec_pretty(&json!({
            "status":"AUTHORED_UNRUN","package":package,"protected":protected,"remote_command_x":remote_x,
            "archive_sha256":archive_hash,"public_files_sha256":files,"ids":ids,
            "ticks":{"other_move":20,"local_depart":160,"protected_move":PROBE_START,"excavate":EXCAVATE_TICK,"end":END_TICK},
            "local_fill":{"min":LOCAL_MIN,"max":LOCAL_MAX,"height":4},
            "resources":{"local":12,"remote":4,"finite":true},"peer_connection_reversal":"not covered; later transport/browser",
        })).unwrap()).unwrap();
        Self {
            app,
            catalog,
            package: package.into(),
            protected,
            tick: 0,
            ids,
            surface,
            seq: [0; 2],
            expected: BTreeMap::new(),
            archive_hash,
            dir,
            remote_x,
            wire_outbox: None,
            lockfile_hash: booted.lockfile_hash,
            dig_tool: format!("{package}:{}", binding["dig"].as_str().unwrap()),
            fill_tool: format!("{package}:{}", binding["fill"].as_str().unwrap()),
        }
    }
    pub(super) fn position(&self, label: &str) -> Vec2 {
        let e = unit_entity(self.app.world(), self.ids[label]).unwrap();
        self.app.world().get::<turts_sim::Position>(e).unwrap().0
    }
    pub(super) fn bodies(&self) -> Vec<Body> {
        self.ids
            .iter()
            .map(|(label, id)| {
                let e = unit_entity(self.app.world(), *id).unwrap();
                let kind = self.app.world().get::<turts_sim::UnitType>(e).unwrap().0;
                let definition = self
                    .app
                    .world()
                    .resource::<turts_sim::UnitTypes>()
                    .get(kind)
                    .unwrap();
                Body {
                    label: label.clone(),
                    position: self.position(label),
                    radius: definition.radius,
                    speed: definition.movement.as_ref().unwrap().speed,
                    max_step: 0,
                    max_grade: [0, 1],
                    owner: self.app.world().get::<turts_sim::Owner>(e).unwrap().0.0,
                    idle: physical::idle(self.app.world(), *id),
                }
            })
            .collect()
    }
    fn send(
        &mut self,
        slot: u8,
        order: &str,
        labels: &[&str],
        target: OrderTarget,
        group: u32,
        reverse: bool,
    ) {
        let mut units: Vec<_> = labels
            .iter()
            .map(|label| UnitId(self.ids[*label]))
            .collect();
        if reverse {
            units.reverse();
        }
        self.seq[slot as usize] += 1;
        let command = PlayerCommand {
            seq: self.seq[slot as usize],
            order: OrderEnvelope {
                order: format!("{}:{order}", self.package),
                units,
                target,
                queue: QueueMode::Replace,
                group,
            },
        };
        self.expected.insert(
            (slot, command.seq),
            command.order.units.iter().map(|id| id.0).collect(),
        );
        // Exact ordinary inputs are retained for replay provenance. They are not sent to a player.
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("submitted-commands.jsonl"))
            .unwrap();
        use std::io::Write;
        writeln!(
            file,
            "{}",
            json!({"tick":self.tick,"slot":slot,"command":command})
        )
        .unwrap();
        if let Some(outbox) = self.wire_outbox.as_mut() {
            outbox.push((slot, command));
            return;
        }
        let resolved = orders::to_sim(SlotId(slot), &command, &self.catalog).unwrap();
        self.app
            .world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .extend(resolved);
    }
    fn work(
        &mut self,
        label: &str,
        fill: bool,
        min: [u32; 2],
        max: [u32; 2],
        height: i32,
        reverse: bool,
    ) {
        // The public fixture binds the exact tool. No catalog iteration picks an action.
        let tool = if fill {
            self.fill_tool.clone()
        } else {
            self.dig_tool.clone()
        };
        self.send(
            1 - self.protected,
            "order_work",
            &[label],
            OrderTarget::TerrainWork(TerrainDesignation {
                tool,
                cells: TerrainRect {
                    min: TerrainCell {
                        x: min[0],
                        y: min[1],
                    },
                    max: TerrainCell {
                        x: max[0],
                        y: max[1],
                    },
                },
                target_elevation: height,
            }),
            0,
            reverse,
        );
    }
    fn step(&mut self, reverse: bool) {
        self.submit_scheduled(reverse);
        let before = self.bodies();
        self.app.update();
        self.check_completed_step(&before);
    }
    pub(super) fn submit_scheduled(&mut self, reverse: bool) {
        match self.tick {
            0 => {
                self.work("local_editor", true, LOCAL_MIN, LOCAL_MAX, 4, reverse);
                self.work(
                    "remote_editor",
                    true,
                    [self.remote_x, REMOTE_Y],
                    [self.remote_x + 1, REMOTE_Y + 1],
                    4,
                    reverse,
                );
            }
            20 => self.send(
                1 - self.protected,
                "order_move",
                &["other_probe"],
                OrderTarget::Point(Pos { x: 50.0, y: 40.0 }),
                11,
                reverse,
            ),
            160 => self.send(
                1 - self.protected,
                "order_move",
                &["local_editor"],
                OrderTarget::Point(Pos { x: 26.0, y: 30.0 }),
                12,
                reverse,
            ),
            PROBE_START => self.send(
                self.protected,
                "order_move",
                &["protected_probe", "protected_peer"],
                OrderTarget::Point(Pos { x: 38.0, y: 24.5 }),
                13,
                reverse,
            ),
            EXCAVATE_TICK => self.work("local_editor", false, LOCAL_MIN, LOCAL_MAX, 0, reverse),
            _ => {}
        }
    }
    pub(super) fn check_completed_step(&mut self, before: &[Body]) {
        self.tick += 1;
        assert_eq!(self.app.world().resource::<Tick>().0, self.tick);
        for result in &self.app.world().resource::<CommandResults>().0 {
            let mut expected = self.expected.remove(&(result.slot.0, result.seq)).unwrap();
            expected.sort_unstable();
            let mut actual = result.units.accepted.clone();
            actual.sort_unstable();
            assert_eq!(
                actual, expected,
                "GM6 ordinary command member refusal: {result:?}"
            );
            assert!(result.outcome.is_ok() && result.units.refused.is_empty());
        }
        assert!(self.expected.is_empty());
        let after = self.bodies();
        physical::verify("GM6", self.tick, before, &after, &self.surface);
        if turts_sim::terrain::privileged_changed_chunk_count(self.app.world()) > 0 {
            self.surface = SurfaceState::read(self.app.world());
            physical::verify_support("GM6", self.tick, &after, &self.surface);
        }
    }
    pub(super) fn permitted(&self) -> Value {
        let policy = permissions::completed(self.app.world());
        let (snapshot, _) = export::snapshot_for_with_permissions(
            completed_world(self.app.world()),
            SlotId(self.protected),
            policy.0.as_ref(),
        );
        serde_json::to_value(snapshot).unwrap()
    }
    pub(super) fn member(&self, label: &str) -> Option<turts_sim::terrain::FlockMemberAudit> {
        turts_sim::terrain::privileged_flock_audit(self.app.world())
            .members
            .into_iter()
            .find(|m| m.unit == self.ids[label])
    }
    /// GM6: the flock guidance of the protected owner. Every field of the audit row is in
    /// this cross-world comparison: state, advice, neighbours and the field record.
    pub(super) fn protected_navigation(&self) -> Value {
        let audit = turts_sim::terrain::privileged_flock_audit(self.app.world());
        let rows: Vec<_> = ["protected_probe", "protected_peer"]
            .iter()
            .map(|label| {
                self.member(label)
                    .map(|m| {
                        json!({"label":label,"owner":m.owner,"state":m.state,
                "advice_bits":m.advice,"neighbours":m.neighbours,
                "field_class":m.field_class,"field_goal":m.field_goal,
                "field_revision":m.field_revision,"field_ready_cells":m.field_ready_cells})
                    })
                    .unwrap_or(Value::Null)
            })
            .collect();
        let work: Vec<_> = audit
            .owners
            .iter()
            .filter(|(owner, _, _)| *owner == self.protected)
            .collect();
        json!({"members":rows,"field_work":work})
    }
}

#[test]
fn gm6_mirrored_hidden_edits_preserve_protected_view_and_do_not_poison_other_viewer_route() {
    for package in ["quarry", "terrain_stockyard"] {
        for protected in [0, 1] {
            for variant in VARIANTS {
                let mut a = PrivacyWorld::new(package, protected, variant, 52);
                let mut b = PrivacyWorld::new(package, protected, variant, 54);
                assert_eq!(
                    a.archive_hash, b.archive_hash,
                    "paired worlds must have byte-identical public input"
                );
                let mut other_leases = None;
                let mut frontier = false;
                let mut after_frontier = 0;
                let mut rows = Vec::new();
                while a.tick < END_TICK {
                    a.step(variant.members);
                    b.step(variant.members);
                    assert_eq!(a.tick, b.tick);
                    assert_eq!(
                        a.permitted(),
                        b.permitted(),
                        "{package}/viewer{protected} tick{}: hidden edit leaked into protected complete projection",
                        a.tick
                    );
                    assert_eq!(
                        a.protected_navigation(),
                        b.protected_navigation(),
                        "{package}/viewer{protected} tick{}: hidden edit changed protected guidance",
                        a.tick
                    );
                    assert_eq!(
                        a.position("protected_probe").to_array().map(f32::to_bits),
                        b.position("protected_probe").to_array().map(f32::to_bits),
                        "hidden remote edit changed physical movement"
                    );
                    if a.tick == 200 {
                        assert!(a.surface.rectangle_is(LOCAL_MIN, LOCAL_MAX, 4));
                        assert!(b.surface.rectangle_is(LOCAL_MIN, LOCAL_MAX, 4));
                        assert!(
                            a.surface
                                .rectangle_is([52, REMOTE_Y], [53, REMOTE_Y + 1], 4)
                        );
                        assert!(
                            b.surface
                                .rectangle_is([52, REMOTE_Y], [53, REMOTE_Y + 1], 0)
                        );
                        assert!(
                            b.surface
                                .rectangle_is([54, REMOTE_Y], [55, REMOTE_Y + 1], 4)
                        );
                        assert!(
                            a.surface
                                .rectangle_is([54, REMOTE_Y], [55, REMOTE_Y + 1], 0)
                        );
                        other_leases = Some((
                            a.member("other_probe").expect("other viewer active route"),
                            b.member("other_probe").expect("other viewer active route"),
                        ));
                    }
                    if (PROBE_START + 1..=300).contains(&a.tick) {
                        let (left, right) = other_leases.as_ref().unwrap();
                        // GM6: the move of the protected owner does not change the field of
                        // the other owner.
                        super::diagnostics::same_field(
                            left,
                            &a.member("other_probe")
                                .expect("other viewer membership retained"),
                        );
                        super::diagnostics::same_field(
                            right,
                            &b.member("other_probe")
                                .expect("other viewer membership retained"),
                        );
                    }
                    let p = a.position("protected_probe");
                    // The committed body reaches the hidden barrier's local frontier.
                    // No contact overlay is invented: safe Blocked/Pending may persist.
                    if a.tick > PROBE_START
                        && a.tick < EXCAVATE_TICK
                        && p.x >= 28.7
                        && p.x < 30.0
                        && (22.0..27.0).contains(&p.y)
                    {
                        frontier = true;
                    }
                    if frontier && a.tick < EXCAVATE_TICK {
                        after_frontier += 1;
                    }
                    rows.push(json!({"tick":a.tick,"position_bits":p.to_array().map(f32::to_bits),"permitted_sha256":sha256_hex(&serde_json::to_vec(&a.permitted()).unwrap()),"navigation":a.protected_navigation()}));
                }
                assert!(
                    frontier && after_frontier >= 20,
                    "paired worlds never exercised sustained same local barrier contact"
                );
                assert!(a.surface.rectangle_is(LOCAL_MIN, LOCAL_MAX, 0));
                assert!(b.surface.rectangle_is(LOCAL_MIN, LOCAL_MAX, 0));
                for world in [&a, &b] {
                    fs::write(world.dir.join("privacy-result.json"),serde_json::to_vec_pretty(&json!({"status":"CHECKED","protected":protected,"ticks":world.tick,"after_frontier_ticks":after_frontier,
                    "raw_protected_rows":rows,"other_viewer_field_and_revision_stable":true,"peer_connection_reversal":"UNRUN"})).unwrap()).unwrap();
                }
            }
        }
    }
}
