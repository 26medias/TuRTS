//! UT2 to UT5 (core) scenarios. Every catalog is built in code; no package file is read.
//! The worlds run real ticks through `headless_app`.

use super::*;
use crate::economy::{ledger::Account, power, views};
use crate::{
    Command, CommandKind, CommandResult, RejectReason, SlotId, apply_command, headless_app,
    orders::{OrderCommand, OrderTargetSim, QueueMode},
};
use turts_mod_api::*;
use turts_rules::terrain::geometry::{Cell, Surface};

pub(super) fn cid(local: &str) -> ContentId {
    ContentId::parse(&format!("t:{local}")).unwrap()
}

pub(super) fn unit_def(local: &str) -> ResolvedUnit {
    ResolvedUnit {
        terrain_class: None,
        terrain_cargo: None,
        terrain_foundation: None,
        terrain_tools: Vec::new(),
        vision: None,
        repairer: None,
        id: cid(local),
        name: local.into(),
        footprint: Footprint::Circle { radius: 0.25 },
        movement: None,
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: None,
        constructor: None,
        placement: None,
        build_area: None,
        output_inventory: None,
        extractor: None,
        conveyor: None,
        inserter: None,
        machine: None,
        research: None,
        health: Some(HealthDef { max: 100 }),
        weapon: None,
        removal: None,
        provides: vec![],
        consumes: vec![],
        income: None,
        armour: None,
        acquire: None,
    }
}

fn supply(resource: &str, amount: u64) -> SupplyAmount {
    SupplyAmount {
        resource: format!("t:{resource}"),
        amount,
    }
}

pub(super) struct Opts {
    pub(super) policy: UtilityExposure,
    pub(super) units: Vec<(&'static str, u8, f32, f32)>,
    pub(super) vision: bool,
    pub(super) credits: u64,
    /// What one workshop consumes of the Network resource.
    pub(super) consume: u64,
    pub(super) with_enemy: bool,
    pub(super) size: u32,
}

impl Opts {
    pub(super) fn new(policy: UtilityExposure) -> Self {
        Self {
            policy,
            units: vec![
                ("builder", 0, 8.0, 8.0),
                ("generator", 0, 2.0, 8.0),
                ("workshop", 0, 14.0, 8.0),
                ("builder", 1, 8.0, 15.0),
            ],
            vision: false,
            credits: 100,
            consume: 6,
            with_enemy: false,
            size: SIZE,
        }
    }
}

pub(super) fn catalog(opts: &Opts) -> ContentCatalog {
    let mut builder = unit_def("builder");
    builder.movement = Some(MovementDef { speed: 2.0 });
    builder.terrain_tools = vec![cid("dig"), cid("fill")];
    builder.terrain_cargo = Some(TerrainCargoDef {
        unload_per_second: 20,
        interaction_range: 0.25,
    });
    builder.inventory = Some(ResolvedInventory {
        capacity: 1000,
        accepts: vec![cid("ore")],
    });
    let mut generator = unit_def("generator");
    generator.provides = vec![supply("power", 10), supply("pop", 5)];
    generator.removal = Some(RemovalDef {
        by_command: true,
        contents: ContentsPolicy::Destroy,
    });
    let mut workshop = unit_def("workshop");
    workshop.consumes = vec![supply("power", opts.consume), supply("pop", 3)];
    workshop.producer = Some(ResolvedProducer {
        recipes: vec![],
        queue_capacity: 1,
        spawn_offsets: vec![WorldPos { x: 1.0, y: 0.0 }],
        when_short: vec![WhenShort {
            resource: "t:power".into(),
            effect: ShortEffect::Rate(Ratio {
                numerator: 1,
                denominator: 4,
            }),
        }],
    });
    workshop.removal = generator.removal;
    // A weapon with a Network `when_short` entry: shortage disables it.
    workshop.weapon = Some(ResolvedWeapon {
        damage_type: None,
        bonus: vec![],
        range: 1.0,
        range_mode: WeaponRangeMode::Centre,
        cooldown_ms: 50,
        damage: 1,
        ammo: None,
        targets: TargetFilter {
            slots: SlotRelation::Other,
        },
        when_short: vec![WhenShort {
            resource: "t:power".into(),
            effect: ShortEffect::Disable,
        }],
    });
    let mut enemy = unit_def("enemy");
    enemy.weapon = Some(ResolvedWeapon {
        damage_type: None,
        bonus: vec![],
        range: 2.0,
        range_mode: WeaponRangeMode::Centre,
        cooldown_ms: 50,
        damage: 100,
        ammo: None,
        targets: TargetFilter {
            slots: SlotRelation::Other,
        },
        when_short: vec![],
    });
    let mut units = vec![builder, generator, workshop, enemy];
    if opts.vision {
        for unit in &mut units {
            if unit.id == cid("builder") {
                unit.vision = Some(VisionDef { radius: 3.0 });
            }
        }
    }
    let resource = |name: &str, kind| {
        (
            cid(name),
            ResourceDef {
                id: name.into(),
                name: name.into(),
                unit_name: String::new(),
                kind,
            },
        )
    };
    let network = ResourceKind::Supply {
        scope: SupplyScope::Network,
        counts: SupplyCounts::Completed,
    };
    let resources = [
        resource("ore", ResourceKind::Item),
        resource("credits", ResourceKind::Balance),
        resource("power", network),
        resource("flow", network),
        resource(
            "pop",
            ResourceKind::Supply {
                scope: SupplyScope::Slot,
                counts: SupplyCounts::Completed,
            },
        ),
    ]
    .into_iter()
    .collect();
    let appearance = UtilityAppearance {
        colour: "#ffcc00".into(),
        exposed_colour: "#ff0000".into(),
        junction: UtilitySymbol::Circle,
        labels: UtilityLabels {
            powered: "Powered".into(),
            short: "No power".into(),
            unconnected: "Unconnected".into(),
            exposed: "Exposed".into(),
        },
    };
    let line = ResolvedUtility {
        id: cid("power_line"),
        name: "Power line".into(),
        carries: cid("power"),
        ports: [
            (cid("generator"), WorldPos { x: 0.0, y: 0.0 }),
            (cid("workshop"), WorldPos { x: 0.0, y: 0.0 }),
        ]
        .into_iter()
        .collect(),
        depth: UtilityDepth {
            default: 2,
            min: 1,
            max: 4,
        },
        reach: 9.0,
        max_segment_length: 12.0,
        segment_cost: vec![(cid("credits"), 1)],
        exposure: opts.policy,
        appearance: appearance.clone(),
    };
    let pipe = ResolvedUtility {
        id: cid("pipe"),
        name: "Pipe".into(),
        carries: cid("flow"),
        ports: Default::default(),
        depth: UtilityDepth {
            default: 1,
            min: 1,
            max: 2,
        },
        reach: 9.0,
        max_segment_length: 12.0,
        segment_cost: vec![],
        exposure: UtilityExposure::Operate,
        appearance,
    };
    let density = Ratio {
        numerator: 1000,
        denominator: 1,
    };
    let mut catalog = ContentCatalog {
        utilities: [line, pipe]
            .into_iter()
            .map(|utility| (utility.id.clone(), utility))
            .collect(),
        terrain_materials: Default::default(),
        terrain_movement_classes: Default::default(),
        terrain_tools: Default::default(),
        bots: Default::default(),
        combat: None,
        resources,
        units: units
            .into_iter()
            .map(|unit| (unit.id.clone(), unit))
            .collect(),
        nodes: Default::default(),
        production: Default::default(),
        recipes: Default::default(),
        technologies: Default::default(),
        orders: Default::default(),
        order_queue_depths: Default::default(),
        source_files: Default::default(),
    };
    catalog.terrain_materials.insert(
        cid("stone"),
        ResolvedTerrainMaterial {
            id: cid("stone"),
            name: "Stone".into(),
            surface_color: "#888888".into(),
            hardness: 1,
            work_per_world_cube: density,
            natural_outputs: vec![ResolvedTerrainResourceRate {
                resource: cid("ore"),
                per_world_cube: density,
            }],
            fill_inputs: vec![
                ResolvedTerrainResourceRate {
                    resource: cid("ore"),
                    per_world_cube: density,
                },
                ResolvedTerrainResourceRate {
                    resource: cid("credits"),
                    per_world_cube: density,
                },
            ],
            placed_removal: TerrainPlacedRemoval::NoYield,
        },
    );
    for (name, operation) in [
        ("dig", ResolvedTerrainOperation::Excavate),
        (
            "fill",
            ResolvedTerrainOperation::Fill {
                material: cid("stone"),
            },
        ),
    ] {
        catalog.terrain_tools.insert(
            cid(name),
            ResolvedTerrainTool {
                id: cid(name),
                name: name.into(),
                operation,
                work_per_second: 20,
                reach: 6.0,
                allowed_materials: vec![cid("stone")],
                max_hardness: 1,
                output_destination: TerrainOutputDestination::Inventory,
            },
        );
    }
    catalog
}

pub(super) const SIZE: u32 = 16;
pub(super) const SURFACE: i32 = 20;

fn terrain_def(size: u32) -> TerrainDef {
    TerrainDef {
        length_scale: 1000,
        cell_width: 1000,
        elevation_step: 1,
        columns: size,
        rows: size,
        chunk_width: 4,
        initial_elevation: SURFACE,
        initial_heights: vec![],
        floor: 0,
        ceiling: SURFACE + 1000,
        generation: TerrainGenerationDef::Version1 {
            bands: vec![TerrainDatumBand {
                bottom: 0,
                top: SURFACE,
                choices: vec![TerrainWeightedMaterial {
                    material: cid("stone").to_string(),
                    weight: 1,
                }],
            }],
        },
    }
}

fn resolved_terrain(size: u32) -> ResolvedTerrainDef {
    ResolvedTerrainDef {
        length_scale: 1000,
        cell_width: 1000,
        elevation_step: 1,
        columns: size,
        rows: size,
        chunk_width: 4,
        initial_elevation: SURFACE,
        initial_heights: vec![],
        floor: 0,
        ceiling: SURFACE + 1000,
        generation: vec![ResolvedTerrainDatumBand {
            bottom: 0,
            top: SURFACE,
            choices: vec![ResolvedTerrainWeightedMaterial {
                material: cid("stone"),
                weight: 1,
            }],
        }],
    }
}

pub(super) fn map(opts: &Opts) -> MapDef {
    let mut placements: Vec<Placement> = opts
        .units
        .iter()
        .map(|(unit, owner, x, y)| Placement {
            unit: cid(unit).to_string(),
            owner_slot: *owner,
            pos: WorldPos { x: *x, y: *y },
            contents: vec![],
            facing: None,
            start_order: None,
        })
        .collect();
    if opts.with_enemy {
        // Far from every target: a test moves it into range when it is ready.
        placements.push(Placement {
            unit: cid("enemy").to_string(),
            owner_slot: 1,
            pos: WorldPos { x: 14.0, y: 1.0 },
            contents: vec![],
            facing: None,
            start_order: None,
        });
    }
    MapDef {
        terrain: Some(terrain_def(opts.size)),
        visibility: opts.vision.then_some(VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        }),
        environment: None,
        ground: None,
        id: "yard".into(),
        size: MapSize {
            x: opts.size as f32,
            y: opts.size as f32,
        },
        placements,
        nodes: vec![],
        slot_economy: (0..2)
            .map(|slot| SlotEconomyInit {
                slot,
                balances: vec![BalanceAmount {
                    balance: cid("credits").to_string(),
                    amount: opts.credits,
                }],
            })
            .collect(),
        belts: vec![],
    }
}

pub(super) struct Yard {
    pub(super) app: bevy_app::App,
    seq: u32,
}

pub(super) fn find_unit(app: &bevy_app::App, kind: &str, slot: u8, x: f32, y: f32) -> u32 {
    let world = app.world();
    let types = world.resource::<crate::UnitTypes>();
    world
        .iter_entities()
        .find_map(|entity| {
            let entry = types.get(entity.get::<crate::UnitType>()?.0)?;
            (entry.content_id == cid(kind).to_string()
                && entity.get::<crate::Owner>()?.0 == SlotId(slot)
                && entity.get::<crate::Position>()?.0 == crate::Vec2::new(x, y))
            .then(|| entity.get::<crate::UnitId>().unwrap().0)
        })
        .unwrap_or_else(|| panic!("no {kind} of slot {slot} at ({x}, {y})"))
}

impl Yard {
    pub(super) fn new(policy: UtilityExposure) -> Self {
        Self::build(Opts::new(policy))
    }

    pub(super) fn build(opts: Opts) -> Self {
        let mut app = headless_app(20);
        let staged = crate::bootstrap::stage_package_with_terrain(
            app.world(),
            catalog(&opts),
            SourceDef {
                file: "maps/yard.ron".into(),
                value: map(&opts),
            },
            20,
            cid("yard"),
            Some(resolved_terrain(opts.size)),
            Some(crate::terrain::PrivateTerrainSeed::new(77)),
        )
        .unwrap();
        crate::install_package(app.world_mut(), staged);
        if opts.vision {
            crate::visibility::initialize_context(app.world_mut(), 77, &[0, 1]).unwrap();
        }
        Self { app, seq: 0 }
    }

    pub(super) fn tick(&mut self) {
        self.app.update();
    }

    pub(super) fn ticks(&mut self, count: u32) {
        for _ in 0..count {
            self.tick();
        }
    }

    pub(super) fn now(&self) -> u32 {
        self.app.world().resource::<crate::Tick>().0
    }

    pub(super) fn builder(&self, slot: u8) -> u32 {
        match slot {
            0 => find_unit(&self.app, "builder", 0, 8.0, 8.0),
            _ => find_unit(&self.app, "builder", 1, 8.0, 15.0),
        }
    }

    pub(super) fn generator(&self) -> u32 {
        find_unit(&self.app, "generator", 0, 2.0, 8.0)
    }

    pub(super) fn workshop(&self) -> u32 {
        find_unit(&self.app, "workshop", 0, 14.0, 8.0)
    }

    pub(super) fn command(
        &mut self,
        slot: u8,
        units: Vec<u32>,
        route: UtilityRouteTarget,
    ) -> CommandResult {
        self.seq += 1;
        let command = Command {
            slot: SlotId(slot),
            seq: self.seq,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: cid("route"),
                behaviour: Behaviour::UtilityRoute,
                target_kind: TargetKind::UtilityRoute,
                policy: None,
                units,
                target: OrderTargetSim::UtilityRoute(route),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![cid("builder").to_string()],
                queue_depth: 16,
            })),
        };
        apply_command(self.app.world_mut(), &command)
    }

    /// Slot 0 builder routes a power line at depth 2.
    pub(super) fn route(&mut self, from: UtilityEndpoint, to: UtilityEndpoint) -> CommandResult {
        self.route_at(0, "t:power_line", 2, from, to)
    }

    pub(super) fn route_at(
        &mut self,
        slot: u8,
        utility: &str,
        depth: u32,
        from: UtilityEndpoint,
        to: UtilityEndpoint,
    ) -> CommandResult {
        let builder = self.builder(slot);
        self.command(
            slot,
            vec![builder],
            UtilityRouteTarget {
                utility: utility.into(),
                depth,
                from,
                to,
            },
        )
    }

    pub(super) fn ok_route(&mut self, from: UtilityEndpoint, to: UtilityEndpoint) {
        let result = self.route(from, to);
        assert!(result.outcome.is_ok(), "{result:?}");
    }

    pub(super) fn terrain_work(&mut self, tool: &str, x: u32, y: u32, level: i32) {
        self.seq += 1;
        let builder = self.builder(0);
        let command = Command {
            slot: SlotId(0),
            seq: self.seq,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: cid("terrain"),
                behaviour: Behaviour::TerrainWork,
                target_kind: TargetKind::TerrainDesignation,
                policy: None,
                units: vec![builder],
                target: OrderTargetSim::TerrainWork {
                    tool: cid(tool),
                    cells: TerrainRect {
                        min: TerrainCell { x, y },
                        max: TerrainCell { x: x + 1, y: y + 1 },
                    },
                    target_elevation: level,
                },
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = apply_command(self.app.world_mut(), &command);
        assert!(result.outcome.is_ok(), "{result:?}");
    }

    pub(super) fn height(&self, x: u32, y: u32) -> i32 {
        self.app
            .world()
            .resource::<crate::terrain::TerrainState>()
            .elevation(Cell { x, y })
            .unwrap()
    }

    /// Run ordinary paid work until the cell reaches `level`.
    pub(super) fn work_to(&mut self, tool: &str, x: u32, y: u32, level: i32) {
        self.terrain_work(tool, x, y, level);
        for _ in 0..200 {
            if self.height(x, y) == level {
                return;
            }
            self.tick();
        }
        panic!(
            "the work did not reach level {level}: {}",
            self.height(x, y)
        );
    }

    pub(super) fn view(&mut self) -> WorldView {
        views::world_view(self.app.world_mut())
    }

    pub(super) fn owner(&mut self, slot: u8) -> UtilityOwnerView {
        owner_view(&self.view(), slot)
    }

    pub(super) fn changed(&mut self, slot: u8) -> Option<u32> {
        owner_view_changed_tick(&self.view(), slot)
    }

    pub(super) fn state(&self) -> &UtilityState {
        self.app.world().resource::<UtilityState>()
    }

    pub(super) fn credits(&self, slot: u8) -> u64 {
        self.app.world().resource::<crate::EconomyLedger>().balances[&(slot, cid("credits"))]
            .on_hand
    }

    pub(super) fn set_credits(&mut self, slot: u8, amount: u64) {
        self.app
            .world_mut()
            .resource_mut::<crate::EconomyLedger>()
            .balances
            .insert(
                (slot, cid("credits")),
                Account {
                    on_hand: amount,
                    reserved: 0,
                },
            );
    }

    /// The producer rate of a workshop, as the published view shows it.
    pub(super) fn producer_rate(&mut self, unit: u32) -> (u64, u64) {
        self.view()
            .operating
            .unwrap()
            .rates
            .iter()
            .find(|row| row.unit == unit)
            .unwrap()
            .producer
    }

    /// Everything that a refusal must leave unchanged: the audit state (balances, views,
    /// topology, rows) and the runtime indexes.
    pub(super) fn fingerprint(&mut self) -> Fingerprint {
        let audit = views::audit_state(self.app.world_mut());
        let state = self.state();
        Fingerprint {
            audit,
            snapshot: (*state.snap.0).clone(),
            next: state.next.clone(),
            index: state.index.clone(),
            dirty: state.dirty.clone(),
            pending: state.pending.clone(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct Fingerprint {
    audit: crate::AuditState,
    snapshot: UtilitySnapshot,
    next: BTreeMap<u8, Vec<NetworkRow>>,
    index: BTreeMap<u8, BTreeMap<(u32, u32), BTreeSet<u32>>>,
    dirty: BTreeSet<(u8, ContentId)>,
    pending: BTreeSet<(u8, u32)>,
}

pub(super) fn free(x: f32, y: f32) -> UtilityEndpoint {
    UtilityEndpoint::Free(WorldPos { x, y })
}

pub(super) fn split(segment: u32, x: f32, y: f32) -> UtilityEndpoint {
    UtilityEndpoint::Split {
        segment,
        point: WorldPos { x, y },
    }
}

pub(super) fn port(unit: u32) -> UtilityEndpoint {
    UtilityEndpoint::Port(unit)
}

fn node_pos(owner: &UtilityOwnerView, end: OwnerEnd) -> (f32, f32) {
    let OwnerEnd::Node(id) = end else {
        panic!("not a node end");
    };
    owner.nodes.iter().find(|node| node.id == id).unwrap().pos
}

/// The segment whose two ends are the junctions at `a` and `b`.
fn segment_between(owner: &UtilityOwnerView, a: (f32, f32), b: (f32, f32)) -> &OwnerSegment {
    owner
        .segments
        .iter()
        .find(|segment| {
            let ends = (node_pos(owner, segment.a), node_pos(owner, segment.b));
            ends == (a, b) || ends == (b, a)
        })
        .unwrap_or_else(|| panic!("no segment between {a:?} and {b:?}: {owner:?}"))
}

// ----- UT2 -----

/// The command is applied after the update of tick `k`, so the pass of tick `k + 1` sees it,
/// and the swap at the start of tick `k + 2` publishes its rows.
#[test]
fn ut2_crossing_does_not_connect_split_route_connects_and_supply_follows_next_tick() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let (generator, workshop) = (yard.generator(), yard.workshop());
    // Two lines that cross in plan view at (6, 4): generator to (8, 2), workshop to (4, 3).
    yard.ok_route(port(generator), free(8.0, 2.0));
    yard.ok_route(port(workshop), free(4.0, 3.0));
    assert_eq!(yard.credits(0), 98, "one credit for each new segment");
    yard.ticks(3);
    let owner = yard.owner(0);
    assert_eq!(owner.segments.len(), 2);
    // The lines cross, but only a shared endpoint connects: two networks, consumer short.
    assert_eq!(owner.networks.len(), 2);
    let consumer = owner
        .networks
        .iter()
        .find(|network| network.units.contains(&workshop))
        .unwrap();
    assert_eq!(
        (consumer.provided, consumer.consumed, consumer.short),
        (0, 6, true)
    );
    assert_eq!(yard.producer_rate(workshop), (1, 4));

    // The split route makes two explicit junctions and joins the two lines.
    let (first, second) = (owner.segments[0].id, owner.segments[1].id);
    yard.ok_route(split(first, 5.0, 5.0), split(second, 8.0, 5.0));
    let owner = yard.owner(0);
    assert_eq!(
        owner.nodes.len(),
        4,
        "two free ends and two split junctions"
    );
    assert_eq!(owner.segments.len(), 5, "each split adds a segment");
    // Tick k + 1: the topology is current, the network rows still lag by one tick.
    yard.tick();
    let lagging = yard.owner(0);
    assert!(
        lagging
            .networks
            .iter()
            .any(|network| network.units == vec![workshop] && network.short)
    );
    assert_eq!(yard.producer_rate(workshop), (1, 4));
    // Tick k + 2: `effective` is swapped in.
    yard.tick();
    let owner = yard.owner(0);
    let joined = owner
        .networks
        .iter()
        .find(|network| network.units.contains(&workshop))
        .unwrap();
    assert_eq!(joined.units, vec![generator, workshop]);
    assert_eq!(
        (joined.provided, joined.consumed, joined.short),
        (10, 6, false)
    );
    assert_eq!(yard.producer_rate(workshop), (1, 1));
}

#[test]
fn ut2_port_removal_gives_a_junction_and_keeps_the_geometry() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let (generator, workshop) = (yard.generator(), yard.workshop());
    yard.ok_route(port(generator), free(8.0, 2.0));
    yard.ok_route(port(workshop), UtilityEndpoint::Node(1));
    yard.ok_route(port(generator), port(workshop));
    yard.ticks(3);
    let before = yard.owner(0);
    assert_eq!(before.nodes.len(), 1);
    assert_eq!(before.segments.len(), 3);
    assert_eq!(before.networks.len(), 1);
    assert_eq!(before.networks[0].units, vec![generator, workshop]);
    assert!(!before.networks[0].short);
    yard.seq += 1;
    let command = Command {
        slot: SlotId(0),
        seq: yard.seq,
        kind: CommandKind::Remove { unit_id: generator },
    };
    let result = apply_command(yard.app.world_mut(), &command);
    assert!(result.outcome.is_ok(), "{result:?}");
    yard.tick();
    let after = yard.owner(0);
    let ports_of = |view: &UtilityOwnerView| {
        view.segments
            .iter()
            .flat_map(|segment| [segment.a, segment.b])
            .filter(|end| *end == OwnerEnd::Port(generator))
            .count()
    };
    assert_eq!(ports_of(&before), 2);
    assert_eq!(ports_of(&after), 0, "no segment keeps the removed port");
    // Segments 1 and 3 each had the port: each end became a junction, in ascending segment id.
    assert_eq!(after.nodes.len(), 3);
    assert_eq!(after.nodes[1].pos, (2.0, 8.0));
    assert_eq!(after.nodes[2].pos, (2.0, 8.0));
    assert_eq!(after.segments.len(), 3, "the segments keep their geometry");
    assert_eq!(after.segments[0].a, OwnerEnd::Node(after.nodes[1].id));
    assert_eq!(after.segments[2].a, OwnerEnd::Node(after.nodes[2].id));
    assert_eq!(after.segments[0].elevation, before.segments[0].elevation);
}

#[test]
fn ut2_port_removal_at_the_junction_limit_deletes_the_segment() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let generator = yard.generator();
    yard.ok_route(port(generator), free(8.0, 2.0));
    // Fill the slot with junctions: 511 more, so the slot holds 512.
    seed(&mut yard, 0, 511, 0);
    assert_eq!(yard.owner(0).nodes.len(), 512);
    yard.seq += 1;
    let command = Command {
        slot: SlotId(0),
        seq: yard.seq,
        kind: CommandKind::Remove { unit_id: generator },
    };
    assert!(
        apply_command(yard.app.world_mut(), &command)
            .outcome
            .is_ok()
    );
    yard.tick();
    let owner = yard.owner(0);
    assert!(
        owner.segments.is_empty(),
        "no room for the port junction: the segment is deleted"
    );
    assert_eq!(
        owner.nodes.len(),
        511,
        "its far junction is deleted with it"
    );
}

// ----- UT3 -----

/// Surface is 20 and depth 2: a line over row 4 sits at elevation 18.
fn buried_line(yard: &mut Yard) -> u32 {
    yard.ok_route(free(2.5, 4.5), free(6.5, 4.5));
    yard.tick();
    let owner = yard.owner(0);
    assert_eq!(owner.segments.len(), 1);
    assert_eq!(owner.segments[0].elevation, SURFACE - 2);
    assert_eq!(owner.segments[0].depth, 2);
    owner.segments[0].id
}

fn dig_to_exposure(yard: &mut Yard) {
    // Cover 0 at the cell: the surface is at the line.
    yard.work_to("dig", 4, 4, SURFACE - 2);
    yard.tick();
}

#[test]
fn ut3_fill_and_excavation_change_cover_but_never_the_elevation() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let id = buried_line(&mut yard);
    let cover = |yard: &Yard| i64::from(yard.height(4, 4)) - i64::from(SURFACE - 2);
    assert_eq!(cover(&yard), 2);
    yard.work_to("dig", 4, 4, SURFACE - 1);
    yard.tick();
    assert_eq!(cover(&yard), 1, "excavation lowers the cover");
    let owner = yard.owner(0);
    assert_eq!(owner.segments[0].elevation, SURFACE - 2);
    assert_eq!(owner.segments[0].state, OwnerSegmentState::Conducting);
    // Paid fill with dug ore raises the cover; the line does not move.
    yard.work_to("fill", 4, 4, SURFACE);
    yard.work_to("dig", 4, 3, SURFACE - 1);
    yard.work_to("fill", 4, 4, SURFACE + 1);
    yard.tick();
    assert_eq!(cover(&yard), 3, "fill raises the cover");
    let owner = yard.owner(0);
    assert_eq!(
        owner.segments.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![id]
    );
    assert_eq!(
        owner.segments[0].elevation,
        SURFACE - 2,
        "elevation is fixed"
    );
    assert_eq!(owner.segments[0].state, OwnerSegmentState::Conducting);
}

#[test]
fn ut3_operate_marks_exposed_and_still_conducts() {
    let mut yard = Yard::new(UtilityExposure::Operate);
    let id = buried_line(&mut yard);
    dig_to_exposure(&mut yard);
    let owner = yard.owner(0);
    assert_eq!(owner.segments[0].id, id);
    assert_eq!(owner.segments[0].state, OwnerSegmentState::Exposed);
    assert_eq!(owner.segments[0].elevation, SURFACE - 2);
}

#[test]
fn ut3_operate_keeps_the_consumer_supplied() {
    let mut yard = Yard::new(UtilityExposure::Operate);
    let (generator, workshop) = (yard.generator(), yard.workshop());
    yard.ok_route(port(generator), port(workshop));
    yard.ticks(3);
    assert_eq!(yard.producer_rate(workshop), (1, 1));
    // The line runs along y = 8: dig under it at (5, 7).
    yard.work_to("dig", 5, 7, SURFACE - 2);
    yard.ticks(3);
    assert!(
        yard.owner(0)
            .segments
            .iter()
            .all(|s| s.state == OwnerSegmentState::Exposed)
    );
    assert_eq!(
        yard.producer_rate(workshop),
        (1, 1),
        "Operate still conducts"
    );
}

#[test]
fn ut3_remove_deletes_the_segment_and_its_orphan_junctions() {
    let mut yard = Yard::new(UtilityExposure::Remove);
    buried_line(&mut yard);
    assert_eq!(yard.owner(0).nodes.len(), 2);
    dig_to_exposure(&mut yard);
    let owner = yard.owner(0);
    assert!(owner.segments.is_empty());
    assert!(
        owner.nodes.is_empty(),
        "junctions with no segment are deleted"
    );
    assert_eq!(yard.credits(0), 99, "removal gives no refund");
    // The index holds no row of the deleted segment.
    assert!(yard.state().index.values().all(|index| index.is_empty()));
}

#[test]
fn ut3_disconnect_stops_supply_and_recovers_after_fill() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let (generator, workshop) = (yard.generator(), yard.workshop());
    yard.ok_route(port(generator), port(workshop));
    yard.ticks(3);
    assert_eq!(yard.producer_rate(workshop), (1, 1), "powered");
    let level = yard.height(5, 7);
    yard.work_to("dig", 5, 7, level - 2);
    yard.ticks(2);
    let owner = yard.owner(0);
    assert_eq!(owner.segments[0].state, OwnerSegmentState::Disconnected);
    assert_eq!(yard.producer_rate(workshop), (1, 4), "no power");
    yard.work_to("fill", 5, 7, level);
    yard.ticks(2);
    let owner = yard.owner(0);
    assert_eq!(owner.segments[0].state, OwnerSegmentState::Conducting);
    assert_eq!(yard.producer_rate(workshop), (1, 1), "restored");
}

#[test]
fn ut3_disconnect_recovers_when_the_hole_is_under_one_half_of_a_split_segment() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let id = buried_line(&mut yard);
    dig_to_exposure(&mut yard);
    assert_eq!(
        yard.owner(0).segments[0].state,
        OwnerSegmentState::Disconnected
    );
    // Split away from the hole at (4, 4): the west half covers it, the east half does not.
    yard.ok_route(split(id, 5.5, 4.5), free(5.5, 9.0));
    yard.tick();
    let owner = yard.owner(0);
    let west = segment_between(&owner, (2.5, 4.5), (5.5, 4.5));
    let east = segment_between(&owner, (5.5, 4.5), (6.5, 4.5));
    assert_eq!(west.state, OwnerSegmentState::Disconnected, "{owner:?}");
    assert_eq!(east.state, OwnerSegmentState::Conducting, "{owner:?}");
    assert_eq!(west.elevation, east.elevation);
    // Fill the hole: the half under it recovers.
    yard.work_to("fill", 4, 4, SURFACE);
    yard.tick();
    let owner = yard.owner(0);
    let west = segment_between(&owner, (2.5, 4.5), (5.5, 4.5));
    assert_eq!(west.state, OwnerSegmentState::Conducting, "{owner:?}");
}

#[test]
fn ut3_checked_segment_counter_proves_unchanged_cells_are_not_checked() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    // Line A over row 4, line B over row 9: they share no cell.
    yard.ok_route(free(2.5, 4.5), free(6.5, 4.5));
    yard.ok_route(free(2.5, 9.5), free(6.5, 9.5));
    yard.ticks(3);
    let before = privileged_checked_segments(yard.app.world());
    assert!(before >= 2, "a placement is checked once");
    yard.ticks(3);
    assert_eq!(privileged_checked_segments(yard.app.world()), before);
    // Edit one cell under A only. Each commit checks A and never B.
    let mut commits = 0_u64;
    yard.terrain_work("dig", 4, 4, SURFACE - 3);
    for _ in 0..200 {
        let level = yard.height(4, 4);
        yard.tick();
        if yard.height(4, 4) != level {
            commits += 1;
        }
        if yard.height(4, 4) == SURFACE - 3 {
            break;
        }
    }
    assert_eq!(commits, 3);
    let after = privileged_checked_segments(yard.app.world());
    assert_eq!(
        after - before,
        commits,
        "one segment for each commit: the segment under the changed cell"
    );
}

// ----- UT4 -----

fn refusal_world() -> Yard {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let generator = yard.generator();
    yard.ok_route(port(generator), free(8.0, 2.0));
    yard.ticks(3);
    yard
}

#[test]
fn ut4_every_refusal_leaves_state_and_balances_equal() {
    let mut yard = refusal_world();
    let generator = yard.generator();
    let workshop = yard.workshop();
    type Case = (
        &'static str,
        u8,
        &'static str,
        u32,
        UtilityEndpoint,
        UtilityEndpoint,
        RejectReason,
    );
    use RejectReason::*;
    let cases: Vec<Case> = vec![
        (
            "foreign node",
            1,
            "t:power_line",
            2,
            UtilityEndpoint::Node(1),
            free(8.0, 13.0),
            TargetInvalid,
        ),
        (
            "absent node",
            0,
            "t:power_line",
            2,
            UtilityEndpoint::Node(99),
            free(8.0, 5.0),
            TargetInvalid,
        ),
        (
            "foreign port",
            1,
            "t:power_line",
            2,
            port(generator),
            free(8.0, 13.0),
            TargetInvalid,
        ),
        (
            "absent port",
            0,
            "t:power_line",
            2,
            port(9999),
            free(8.0, 5.0),
            TargetInvalid,
        ),
        (
            "foreign split",
            1,
            "t:power_line",
            2,
            split(1, 5.0, 5.0),
            free(8.0, 13.0),
            TargetInvalid,
        ),
        (
            "absent split",
            0,
            "t:power_line",
            2,
            split(99, 5.0, 5.0),
            free(8.0, 5.0),
            TargetInvalid,
        ),
        (
            "wrong utility type on a node",
            0,
            "t:pipe",
            1,
            UtilityEndpoint::Node(1),
            free(8.0, 5.0),
            TargetInvalid,
        ),
        (
            "wrong utility type on a port",
            0,
            "t:pipe",
            1,
            port(workshop),
            free(8.0, 5.0),
            TargetInvalid,
        ),
        (
            "unknown utility",
            0,
            "t:nope",
            2,
            free(5.0, 5.0),
            free(8.0, 5.0),
            TargetInvalid,
        ),
        (
            "depth above the range",
            0,
            "t:power_line",
            5,
            free(5.0, 5.0),
            free(8.0, 5.0),
            TargetInvalid,
        ),
        (
            "depth zero",
            0,
            "t:power_line",
            0,
            free(5.0, 5.0),
            free(8.0, 5.0),
            TargetInvalid,
        ),
        (
            "zero length",
            0,
            "t:power_line",
            2,
            free(5.0, 5.0),
            free(5.0, 5.0),
            TargetInvalid,
        ),
        (
            "length 0.4",
            0,
            "t:power_line",
            2,
            free(5.0, 5.0),
            free(5.4, 5.0),
            TargetInvalid,
        ),
        (
            "same node twice",
            0,
            "t:power_line",
            2,
            UtilityEndpoint::Node(1),
            UtilityEndpoint::Node(1),
            TargetInvalid,
        ),
        (
            "duplicate pair",
            0,
            "t:power_line",
            2,
            port(generator),
            UtilityEndpoint::Node(1),
            TargetInvalid,
        ),
        (
            "reversed duplicate pair",
            0,
            "t:power_line",
            2,
            UtilityEndpoint::Node(1),
            port(generator),
            TargetInvalid,
        ),
        (
            "outside the map",
            0,
            "t:power_line",
            2,
            free(-1.0, 5.0),
            free(5.0, 5.0),
            TargetInvalid,
        ),
        (
            "outside the map east",
            0,
            "t:power_line",
            2,
            free(5.0, 5.0),
            free(17.0, 5.0),
            TargetInvalid,
        ),
        (
            "split too far from the line",
            0,
            "t:power_line",
            2,
            split(1, 5.0, 5.6),
            free(5.0, 9.0),
            TargetInvalid,
        ),
        (
            "split near an end",
            0,
            "t:power_line",
            2,
            split(1, 2.2, 7.6),
            free(5.0, 9.0),
            TargetInvalid,
        ),
        (
            "too long",
            0,
            "t:power_line",
            2,
            free(1.0, 1.0),
            free(15.0, 15.0),
            OutOfRange,
        ),
        (
            "out of reach",
            0,
            "t:power_line",
            2,
            free(1.0, 1.0),
            free(3.0, 1.0),
            OutOfRange,
        ),
    ];
    for (name, slot, utility, depth, from, to, reason) in cases {
        let before = yard.fingerprint();
        let result = yard.route_at(slot, utility, depth, from, to);
        assert_eq!(result.outcome, Err(reason), "{name}: {result:?}");
        assert_eq!(
            yard.fingerprint(),
            before,
            "{name}: state or balance changed"
        );
    }
}

#[test]
fn ut4_zero_balance_refuses_without_charge_or_change() {
    let mut yard = refusal_world();
    yard.set_credits(0, 0);
    let before = yard.fingerprint();
    let result = yard.route(free(5.0, 5.0), free(8.0, 5.0));
    assert_eq!(result.outcome, Err(RejectReason::InsufficientBalance));
    assert_eq!(yard.fingerprint(), before);
    assert_eq!(yard.credits(0), 0);
    // With one credit the same route is accepted and charges exactly one.
    yard.set_credits(0, 1);
    yard.ok_route(free(5.0, 5.0), free(8.0, 5.0));
    assert_eq!(yard.credits(0), 0);
}

#[test]
fn ut4_foreign_and_absent_ids_give_identical_results() {
    let mut yard = refusal_world();
    let generator = yard.generator();
    for (foreign, absent) in [
        (UtilityEndpoint::Node(1), UtilityEndpoint::Node(777)),
        (port(generator), port(777)),
        (split(1, 5.0, 5.0), split(777, 5.0, 5.0)),
    ] {
        let a = yard.route_at(1, "t:power_line", 2, foreign, free(8.0, 13.0));
        let b = yard.route_at(1, "t:power_line", 2, absent, free(8.0, 13.0));
        assert_eq!(
            (&a.outcome, &a.units),
            (&b.outcome, &b.units),
            "{foreign:?} against {absent:?}"
        );
        assert_eq!(a.outcome, Err(RejectReason::TargetInvalid));
    }
}

#[test]
fn ut4_non_builder_and_out_of_reach_units_are_refused_per_unit() {
    let mut yard = refusal_world();
    let (generator, builder) = (yard.generator(), yard.builder(0));
    let route = |from, to| UtilityRouteTarget {
        utility: "t:power_line".into(),
        depth: 2,
        from,
        to,
    };
    // The generator is an own unit, but it is not in the order's `units` list.
    let before = yard.fingerprint();
    let result = yard.command(0, vec![generator], route(free(5.0, 5.0), free(8.0, 5.0)));
    assert_eq!(result.outcome, Err(RejectReason::NotApplicable));
    assert_eq!(yard.fingerprint(), before);
    // A builder and a non-builder: the builder places the line, the other is refused.
    let result = yard.command(
        0,
        vec![generator, builder],
        route(free(5.0, 5.0), free(8.0, 5.0)),
    );
    assert!(result.outcome.is_ok(), "{result:?}");
    assert_eq!(result.units.accepted, vec![builder]);
    assert_eq!(
        result.units.refused,
        vec![(generator, RejectReason::NotApplicable)]
    );
    // A foreign builder never reaches the topology of slot 0.
    let foreign = yard.builder(1);
    let result = yard.command(0, vec![foreign], route(free(5.0, 6.0), free(8.0, 6.0)));
    assert_eq!(result.outcome, Err(RejectReason::NotOwner));
}

#[test]
fn ut4_out_of_reach_is_refused_per_unit() {
    let mut yard = refusal_world();
    // Move the builder out of reach of both endpoints.
    let near = yard.builder(0);
    let world = yard.app.world_mut();
    let entity = crate::unit_entity(world, near).unwrap();
    world.get_mut::<crate::Position>(entity).unwrap().0 = crate::Vec2::new(1.0, 1.0);
    let before = yard.fingerprint();
    let result = yard.command(
        0,
        vec![near],
        UtilityRouteTarget {
            utility: "t:power_line".into(),
            depth: 2,
            from: free(8.0, 12.0),
            to: free(10.0, 12.0),
        },
    );
    assert_eq!(result.outcome, Err(RejectReason::OutOfRange));
    assert_eq!(result.units.refused, vec![(near, RejectReason::OutOfRange)]);
    assert_eq!(yard.fingerprint(), before);
}

#[test]
fn ut4_unobserved_cells_are_refused_and_observed_cells_are_accepted() {
    let mut yard = Yard::build({
        let mut opts = Opts::new(UtilityExposure::Disconnect);
        opts.vision = true;
        opts
    });
    yard.ticks(3);
    let before = yard.fingerprint();
    // Within reach of the builder but outside its sight of 3 world units.
    let result = yard.route(free(8.0, 3.0), free(8.0, 5.0));
    assert_eq!(
        result.outcome,
        Err(RejectReason::UtilityNotObserved),
        "{result:?}"
    );
    assert_eq!(yard.fingerprint(), before);
    // Next to the builder the cells are observed.
    let result = yard.route(free(7.5, 8.5), free(9.5, 8.5));
    assert!(result.outcome.is_ok(), "{result:?}");
}

/// Fill a slot with junctions and segments in one write. Node 1 is at (7, 8) and node 2 at
/// (9, 8); a segment never joins them, so a test can place that one.
pub(super) fn seed(yard: &mut Yard, slot: u8, extra_nodes: usize, segments: usize) {
    let utility = cid("power_line");
    let world = yard.app.world_mut();
    let base = world.resource::<UtilityState>().counters(slot);
    let mut topology = world
        .resource::<UtilityState>()
        .topology(slot)
        .cloned()
        .unwrap_or_default();
    let total = extra_nodes;
    let mut ids = Vec::new();
    for index in 0..total {
        let id = topology.next_node;
        topology.next_node += 1;
        let pos = (0.5 + (index % 15) as f32, 0.5 + ((index / 15) % 15) as f32);
        topology.nodes.insert(
            id,
            Node {
                utility: utility.clone(),
                pos,
            },
        );
        ids.push(id);
    }
    let mut made = 0;
    'fill: for gap in 2..ids.len().max(2) {
        for index in 0..ids.len().saturating_sub(gap) {
            if made == segments {
                break 'fill;
            }
            let (a, b) = (ids[index], ids[index + gap]);
            let id = topology.next_segment;
            topology.next_segment += 1;
            topology.segments.insert(
                id,
                Segment {
                    utility: utility.clone(),
                    a: End::Node(a),
                    b: End::Node(b),
                    a_pos: topology.nodes[&a].pos,
                    b_pos: topology.nodes[&b].pos,
                    elevation: SURFACE - 2,
                    depth: 2,
                    state: SegmentState::Conducting,
                },
            );
            made += 1;
        }
    }
    assert_eq!(made, segments, "the seed could not make every segment");
    apply_write(
        world,
        UtilityWrite {
            slot,
            utility,
            base,
            topology,
            made: vec![],
        },
    );
}

#[test]
fn ut4_slot_limits_refuse_new_junctions_and_new_segments() {
    // Junction limit: 512 junctions; a free-free route needs two more, a node-node route none.
    let mut yard = refusal_world();
    seed(&mut yard, 0, 511, 0);
    assert_eq!(yard.owner(0).nodes.len(), 512);
    let before = yard.fingerprint();
    let result = yard.route(free(5.0, 5.0), free(8.0, 5.0));
    assert_eq!(result.outcome, Err(RejectReason::UtilityLimitReached));
    assert_eq!(yard.fingerprint(), before);
    let split_route = yard.route(split(1, 5.0, 5.0), free(8.0, 4.0));
    assert_eq!(split_route.outcome, Err(RejectReason::UtilityLimitReached));
    assert_eq!(yard.fingerprint(), before);
    // Slot 1 has its own limit: a full slot 0 does not refuse slot 1.
    let other = yard.route_at(1, "t:power_line", 2, free(5.0, 13.0), free(8.0, 13.0));
    assert!(other.outcome.is_ok(), "{other:?}");

    // Segment limit: 512 segments in the slot; one more is refused.
    let mut yard = refusal_world();
    seed(&mut yard, 0, 100, 511);
    assert_eq!(yard.owner(0).segments.len(), 512);
    let before = yard.fingerprint();
    let result = yard.route(free(5.0, 5.0), free(8.0, 5.0));
    assert_eq!(result.outcome, Err(RejectReason::UtilityLimitReached));
    assert_eq!(yard.fingerprint(), before);
}

// ----- UT5 -----

/// A world with several workshops and generators. `order` permutes both the creation order
/// of the units and the order of the route commands.
fn several(reverse: bool, consume: u64) -> Yard {
    let mut units = vec![
        ("builder", 0, 8.0, 8.0),
        ("builder", 1, 8.0, 15.0),
        // Network A: two generators, two workshops.
        ("generator", 0, 3.0, 6.0),
        ("generator", 0, 4.0, 9.0),
        ("workshop", 0, 3.0, 10.0),
        ("workshop", 0, 5.0, 7.0),
        // Network B: one generator, two workshops.
        ("generator", 0, 12.0, 6.0),
        ("workshop", 0, 12.0, 9.0),
        ("workshop", 0, 13.0, 8.0),
        // Alone: a workshop with no segment.
        ("workshop", 0, 8.0, 12.0),
    ];
    if reverse {
        units.reverse();
    }
    let mut opts = Opts::new(UtilityExposure::Disconnect);
    opts.units = units;
    opts.consume = consume;
    let mut yard = Yard::build(opts);
    let at = |yard: &Yard, kind: &str, x: f32, y: f32| find_unit(&yard.app, kind, 0, x, y);
    let mut routes = vec![
        (("generator", 3.0, 6.0), ("workshop", 5.0, 7.0)),
        (("workshop", 5.0, 7.0), ("generator", 4.0, 9.0)),
        (("generator", 4.0, 9.0), ("workshop", 3.0, 10.0)),
        (("generator", 12.0, 6.0), ("workshop", 12.0, 9.0)),
        (("workshop", 12.0, 9.0), ("workshop", 13.0, 8.0)),
    ];
    if reverse {
        routes.reverse();
    }
    for ((from_kind, fx, fy), (to_kind, tx, ty)) in routes {
        let (from, to) = (at(&yard, from_kind, fx, fy), at(&yard, to_kind, tx, ty));
        yard.ok_route(port(from), port(to));
    }
    yard.ticks(3);
    yard
}

/// The networks of a world by what the units are, never by id: for each network the sorted
/// (kind, position) of its units, then P, C and short.
type Normal = Vec<(Vec<(String, i32, i32)>, u64, u64, bool)>;

fn normalized(yard: &mut Yard) -> Normal {
    let owner = yard.owner(0);
    let world = yard.app.world();
    let types = world.resource::<crate::UnitTypes>();
    let describe = |unit: u32| {
        let entity = crate::unit_entity(world, unit).unwrap();
        let position = world.get::<crate::Position>(entity).unwrap().0;
        let kind = types
            .get(world.get::<crate::UnitType>(entity).unwrap().0)
            .unwrap()
            .content_id
            .clone();
        (kind, (position.x * 10.0) as i32, (position.y * 10.0) as i32)
    };
    let mut rows: Normal = owner
        .networks
        .iter()
        .map(|network| {
            let mut units: Vec<_> = network.units.iter().map(|unit| describe(*unit)).collect();
            units.sort();
            (units, network.provided, network.consumed, network.short)
        })
        .collect();
    rows.sort();
    rows
}

#[test]
fn ut5_totals_are_exact_for_chains_and_several_networks() {
    let mut yard = several(false, 6);
    let rows = normalized(&mut yard);
    let kind = |name: &str| cid(name).to_string();
    let net = |units: &[(&str, i32, i32)], p, c, short| {
        let mut units: Vec<_> = units
            .iter()
            .map(|(name, x, y)| (kind(name), *x, *y))
            .collect();
        units.sort();
        (units, p, c, short)
    };
    let mut expected = vec![
        // A chain: generator - workshop - generator - workshop. P = 20, C = 12.
        net(
            &[
                ("generator", 30, 60),
                ("workshop", 50, 70),
                ("generator", 40, 90),
                ("workshop", 30, 100),
            ],
            20,
            12,
            false,
        ),
        // B: P = 10, C = 12: short.
        net(
            &[
                ("generator", 120, 60),
                ("workshop", 120, 90),
                ("workshop", 130, 80),
            ],
            10,
            12,
            true,
        ),
        // A workshop with no segment is a network of one unit.
        net(&[("workshop", 80, 120)], 0, 6, true),
    ];
    expected.sort();
    assert_eq!(rows, expected);
    // Every consumer of one network gets the same result.
    let rate = |yard: &mut Yard, x: f32, y: f32| {
        let unit = find_unit(&yard.app, "workshop", 0, x, y);
        yard.producer_rate(unit)
    };
    assert_eq!(rate(&mut yard, 5.0, 7.0), (1, 1));
    assert_eq!(rate(&mut yard, 3.0, 10.0), (1, 1));
    assert_eq!(rate(&mut yard, 12.0, 9.0), (1, 4));
    assert_eq!(rate(&mut yard, 13.0, 8.0), (1, 4));
    assert_eq!(rate(&mut yard, 8.0, 12.0), (1, 4));
}

#[test]
fn ut5_a_weapon_with_a_network_when_short_follows_its_own_network() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let (generator, workshop) = (yard.generator(), yard.workshop());
    yard.ticks(3);
    // No segment: the workshop is a network of its own and is short.
    assert!(!power::weapon_enabled(yard.app.world(), workshop));
    yard.ok_route(port(generator), port(workshop));
    yard.ticks(3);
    assert!(power::weapon_enabled(yard.app.world(), workshop));
    // A weapon rule does not read a slot total: the generator has no weapon rule.
    assert!(power::weapon_enabled(yard.app.world(), generator));
}

#[test]
fn ut5_equality_is_sufficient_supply() {
    // Network A has P = 20 and two workshops. With consume 10 each, C = 20: not short.
    let mut yard = several(false, 10);
    let rows = normalized(&mut yard);
    let a = rows.iter().find(|row| row.0.len() == 4).unwrap();
    assert_eq!((a.1, a.2, a.3), (20, 20, false));
    let b = rows.iter().find(|row| row.0.len() == 3).unwrap();
    assert_eq!((b.1, b.2, b.3), (10, 20, true));
    // One unit more of consumption makes A short.
    let mut yard = several(false, 11);
    let rows = normalized(&mut yard);
    let a = rows.iter().find(|row| row.0.len() == 4).unwrap();
    assert_eq!((a.1, a.2, a.3), (20, 22, true));
}

#[test]
fn ut5_a_generator_killed_at_tick_t_supplies_nothing_in_tick_t_plus_1() {
    let mut opts = Opts::new(UtilityExposure::Disconnect);
    opts.with_enemy = true;
    let mut yard = Yard::build(opts);
    let (generator, workshop) = (yard.generator(), yard.workshop());
    yard.ok_route(port(generator), port(workshop));
    yard.ticks(4);
    assert_eq!(yard.producer_rate(workshop), (1, 1));
    // Move the enemy into range of the generator: ordinary combat kills it.
    let enemy = find_unit(&yard.app, "enemy", 1, 14.0, 1.0);
    let world = yard.app.world_mut();
    let entity = crate::unit_entity(world, enemy).unwrap();
    world.get_mut::<crate::Position>(entity).unwrap().0 = crate::Vec2::new(3.5, 8.0);
    let mut killed = None;
    for _ in 0..40 {
        yard.tick();
        if crate::unit_entity(yard.app.world(), generator).is_none() {
            killed = Some(yard.now());
            break;
        }
        assert_eq!(yard.producer_rate(workshop), (1, 1), "before the kill");
    }
    let killed = killed.expect("combat killed the generator");
    // Tick T: the rows that tick T publishes still come from the swap at its start.
    assert_eq!(yard.producer_rate(workshop), (1, 1), "tick T");
    yard.tick();
    assert_eq!(yard.now(), killed + 1);
    assert_eq!(
        yard.producer_rate(workshop),
        (1, 4),
        "tick T + 1: the dead generator supplies nothing"
    );
    let owner = yard.owner(0);
    let row = owner
        .networks
        .iter()
        .find(|network| network.units.contains(&workshop))
        .unwrap();
    assert_eq!((row.provided, row.consumed, row.short), (0, 6, true));
}

#[test]
fn ut5_reversed_creation_and_command_order_give_equal_networks_and_effects() {
    let mut forward = several(false, 6);
    let mut backward = several(true, 6);
    // The instrument can fail: the two worlds really differ in unit ids and topology ids.
    assert_ne!(
        find_unit(&forward.app, "generator", 0, 3.0, 6.0),
        find_unit(&backward.app, "generator", 0, 3.0, 6.0)
    );
    assert_ne!(forward.fingerprint(), backward.fingerprint());
    assert_eq!(normalized(&mut forward), normalized(&mut backward));
    let effects = |yard: &mut Yard| {
        let view = yard.view();
        let world = yard.app.world();
        let mut rows: Vec<_> = view
            .operating
            .unwrap()
            .rates
            .iter()
            .map(|row| {
                let entity = crate::unit_entity(world, row.unit).unwrap();
                let position = world.get::<crate::Position>(entity).unwrap().0;
                (
                    (position.x * 10.0) as i32,
                    (position.y * 10.0) as i32,
                    row.producer,
                )
            })
            .collect();
        rows.sort();
        rows
    };
    assert_eq!(effects(&mut forward), effects(&mut backward));
}

#[test]
fn ut5_same_build_replay_gives_the_same_state() {
    let mut first = several(false, 6);
    let mut second = several(false, 6);
    first.ticks(5);
    second.ticks(5);
    assert_eq!(first.fingerprint(), second.fingerprint());
    // The instrument can fail: another creation order is a different state.
    let mut other = several(true, 6);
    other.ticks(5);
    assert_ne!(first.fingerprint(), other.fingerprint());
}

#[test]
fn ut5_a_network_resource_never_enters_slot_supply_and_slot_supply_is_unchanged() {
    let mut yard = several(false, 6);
    let world = yard.app.world();
    let rows = power::supply_rows(world);
    assert!(
        rows.iter().all(|row| row.resource == cid("pop")),
        "only the Slot resource has a row: {rows:?}"
    );
    // Slot scope: three generators (5 each) and five workshops (3 each).
    let slot0 = rows.iter().find(|row| row.slot == 0).unwrap();
    assert_eq!((slot0.provided, slot0.consumed), (15, 15));
    let view = yard.view();
    let operating = view.operating.unwrap();
    assert!(
        operating
            .supply
            .iter()
            .all(|row| row.resource == cid("pop"))
    );
    let audit = views::audit_state(yard.app.world_mut())
        .raw_operating
        .unwrap();
    assert!(
        audit
            .live_supply
            .iter()
            .all(|row| row.resource == cid("pop"))
    );
    assert!(audit.effects.iter().all(|row| row.resource == cid("pop")));
}

#[test]
fn owner_view_changed_tick_moves_exactly_on_changes_and_never_on_a_quiet_tick() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let (generator, workshop) = (yard.generator(), yard.workshop());
    yard.ticks(3);
    let quiet = yard.changed(0).unwrap();
    let other = yard.changed(1).unwrap();
    yard.ticks(2);
    assert_eq!(yard.changed(0), Some(quiet), "a quiet tick changes nothing");
    // A placement changes the topology rows at its own tick.
    let k = yard.now();
    yard.ok_route(port(generator), port(workshop));
    assert_eq!(yard.changed(0), Some(k));
    // Tick k + 1: the pass computes new rows, the swap comes next tick.
    yard.tick();
    assert_eq!(yard.changed(0), Some(k));
    yard.tick();
    assert_eq!(
        yard.changed(0),
        Some(k + 2),
        "the network row changed after the swap"
    );
    yard.tick();
    assert_eq!(yard.changed(0), Some(k + 2));
    // Excavation that leaves cover above 0 writes no change.
    let level = yard.height(5, 7);
    yard.work_to("dig", 5, 7, level - 1);
    yard.ticks(2);
    assert_eq!(yard.changed(0), Some(k + 2), "cover 1 is still buried");
    // Excavation to the line: the exposure state changes at its tick.
    let before = yard.now();
    yard.terrain_work("dig", 5, 7, level - 2);
    let mut flipped = None;
    for _ in 0..40 {
        yard.tick();
        if yard.owner(0).segments[0].state == OwnerSegmentState::Disconnected {
            flipped = Some(yard.now());
            break;
        }
    }
    let flipped = flipped.expect("the segment is disconnected");
    assert!(flipped > before);
    assert_eq!(yard.changed(0), Some(flipped));
    yard.tick();
    assert_eq!(
        yard.changed(0),
        Some(flipped + 1),
        "the rows split after the swap"
    );
    yard.tick();
    assert_eq!(yard.changed(0), Some(flipped + 1));
    // Fill restores the state, then the rows.
    yard.work_to("fill", 5, 7, level);
    yard.ticks(3);
    let restored = yard.changed(0).unwrap();
    assert!(restored > flipped + 1);
    yard.ticks(3);
    assert_eq!(yard.changed(0), Some(restored));
    // Port removal changes the topology (a junction) at the pass tick.
    yard.seq += 1;
    let command = Command {
        slot: SlotId(0),
        seq: yard.seq,
        kind: CommandKind::Remove { unit_id: generator },
    };
    assert!(
        apply_command(yard.app.world_mut(), &command)
            .outcome
            .is_ok()
    );
    yard.tick();
    let removed = yard.now();
    assert_eq!(yard.changed(0), Some(removed));
    yard.tick();
    assert_eq!(yard.changed(0), Some(removed + 1));
    yard.ticks(2);
    assert_eq!(yard.changed(0), Some(removed + 1));
    // Slot 1 never changed, and a world with no utility type has no tick.
    assert_eq!(yard.changed(1), Some(other));
    assert_eq!(owner_view_changed_tick(&WorldView::default(), 0), None);
}

#[test]
fn the_world_view_snapshot_round_trips_through_json_and_compares_by_value() {
    let mut yard = Yard::new(UtilityExposure::Disconnect);
    let (generator, workshop) = (yard.generator(), yard.workshop());
    yard.ok_route(port(generator), port(workshop));
    yard.ticks(3);
    let view = yard.view();
    let json = serde_json::to_string(&view).unwrap();
    let back: WorldView = serde_json::from_str(&json).unwrap();
    assert_eq!(back.utilities, view.utilities);
    assert_eq!(owner_view(&back, 0), owner_view(&view, 0));
    assert!(owner_view(&view, 0).segments.len() == 1);
    // A change of one segment state makes the views unequal: the field is part of equality.
    let mut changed = back.clone();
    let mut snapshot = (*changed.utilities.as_ref().unwrap().0).clone();
    snapshot
        .slots
        .get_mut(&0)
        .unwrap()
        .segments
        .values_mut()
        .for_each(|segment| segment.state = SegmentState::Disconnected);
    changed.utilities = Some(SharedSnapshot(std::sync::Arc::new(snapshot)));
    assert_ne!(changed, view);
}
