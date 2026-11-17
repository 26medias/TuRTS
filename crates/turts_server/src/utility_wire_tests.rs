//! M6 UT6: two-viewer privacy of underground utility rows on the real wire.
//!
//! Paired worlds load the public `utility_yard` package and receive identical commands,
//! except that slot 0 places lines only in the right world. Slot 0 then digs a cell under
//! a line in both worlds, so the right world also exposes and disconnects it. Viewer 1's
//! encoded snapshot and every encoded patch must stay byte-equal across the pair.
use bevy::prelude::*;
use std::path::PathBuf;
use turts_mod_api::{
    ContentId, TerrainCell, TerrainRect, UtilityEndpoint, UtilityRouteTarget, WorldPos,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    visibility_transaction::{StateTransactionFrame, encode_transaction},
    *,
};
use turts_sim::{CompletedWorld, SlotId as SimSlotId};

const ROUTE: &str = "utility_yard:order_route";
const LINE: &str = "utility_yard:power_line";

struct World2 {
    app: App,
}

impl World2 {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/utility_yard");
        let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
        let mut app = turts_sim::headless_app(20);
        turts_pack::load_package_with_terrain_seed(
            app.world_mut(),
            &archive,
            Some(turts_sim::terrain::PrivateTerrainSeed::new(77)),
        )
        .unwrap();
        turts_sim::visibility::initialize_context(
            app.world_mut(),
            9001,
            &(0..turts_mod_api::SLOT_COUNT).collect::<Vec<_>>(),
        )
        .unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        Self { app }
    }
    fn view(&self) -> &turts_sim::WorldView {
        &self.app.world().resource::<CompletedWorld>().0
    }
    fn baseline(&self, slot: u8) -> crate::export::PermittedBaseline {
        let permissions = crate::permissions::completed(self.app.world());
        crate::export::PermittedBaseline::capture(
            self.view(),
            SimSlotId(slot),
            permissions.0.as_ref(),
        )
    }
    fn unit(&self, slot: u8, content: &str) -> u32 {
        self.baseline(slot)
            .state
            .units
            .iter()
            .filter(|unit| unit.owner_slot.0 == slot && unit.content_id == content)
            .map(|unit| unit.unit_id.0)
            .min()
            .unwrap_or_else(|| panic!("no {content} of slot {slot}"))
    }
    fn order(
        &mut self,
        seq: u32,
        order: &str,
        behaviour: turts_mod_api::Behaviour,
        target_kind: turts_mod_api::TargetKind,
        units: Vec<u32>,
        target: turts_sim::orders::OrderTargetSim,
    ) -> bool {
        let command = turts_sim::Command {
            slot: SimSlotId(0),
            seq,
            kind: turts_sim::CommandKind::Order(Box::new(turts_sim::orders::OrderCommand {
                order: ContentId::parse(order).unwrap(),
                behaviour,
                target_kind,
                policy: None,
                units,
                target,
                queue: turts_sim::orders::QueueMode::Replace,
                group: 0,
                // The server fills this from the order file's `units` list.
                allowed_units: match behaviour {
                    turts_mod_api::Behaviour::UtilityRoute => vec!["utility_yard:depot".into()],
                    _ => vec![],
                },
                queue_depth: 16,
            })),
        };
        let result = turts_sim::apply_command(self.app.world_mut(), &command);
        if result.outcome.is_err() || !result.units.refused.is_empty() {
            eprintln!(
                "order {order} refused: {:?} {:?}",
                result.outcome, result.units.refused
            );
        }
        result.outcome.is_ok() && result.units.refused.is_empty()
    }
    fn route(&mut self, seq: u32, from: UtilityEndpoint, to: UtilityEndpoint) -> bool {
        let depot = self.unit(0, "utility_yard:depot");
        self.order(
            seq,
            ROUTE,
            turts_mod_api::Behaviour::UtilityRoute,
            turts_mod_api::TargetKind::UtilityRoute,
            vec![depot],
            turts_sim::orders::OrderTargetSim::UtilityRoute(UtilityRouteTarget {
                utility: LINE.into(),
                depth: 2,
                from,
                to,
            }),
        )
    }
    fn dig(&mut self, seq: u32, cell: TerrainCell, target_elevation: i32) -> bool {
        let porter = self.unit(0, "utility_yard:porter");
        self.order(
            seq,
            "utility_yard:order_work",
            turts_mod_api::Behaviour::TerrainWork,
            turts_mod_api::TargetKind::TerrainDesignation,
            vec![porter],
            turts_sim::orders::OrderTargetSim::TerrainWork {
                tool: ContentId::parse("utility_yard:scraper").unwrap(),
                cells: TerrainRect {
                    min: cell,
                    max: TerrainCell {
                        x: cell.x + 1,
                        y: cell.y + 1,
                    },
                },
                target_elevation,
            },
        )
    }
    fn stamp(&self, slot: u8, transaction_id: u64) -> StateTransactionStamp {
        StateTransactionStamp {
            basis: self.baseline(slot).knowledge.unwrap().basis,
            transaction_id,
            parent_transaction_id: transaction_id.checked_sub(1).filter(|id| *id > 0),
        }
    }
}

fn frames(stamp: StateTransactionStamp, payload: &StateTransactionPayload) -> Vec<Vec<u8>> {
    let mut encoded = encode_transaction(stamp, payload, 512).unwrap();
    let mut frames = Vec::new();
    while let Some(frame) = encoded.next_frame() {
        let bytes = match frame {
            StateTransactionFrame::Begin(begin) => postcard::to_allocvec(&begin),
            StateTransactionFrame::Chunk(chunk) => postcard::to_allocvec(&chunk),
            StateTransactionFrame::End(end) => postcard::to_allocvec(&end),
        };
        frames.push(bytes.unwrap());
    }
    frames
}

fn snapshot_frames(world: &World2, slot: u8) -> Vec<Vec<u8>> {
    let baseline = world.baseline(slot);
    frames(
        world.stamp(slot, 1),
        &StateTransactionPayload::Snapshot {
            state: Box::new(baseline.state),
            knowledge: baseline.knowledge,
            event_units: vec![],
        },
    )
}

/// Generator port (18.2, 16) to a free point north-east, then the workshop port
/// (29.6, 16) to a free point north-west: the two segments cross in plan view.
fn crossing_pair(world: &mut World2, seq: u32) {
    let generator = world.unit(0, "utility_yard:generator");
    let workshop = world.unit(0, "utility_yard:workshop");
    assert!(world.route(
        seq,
        UtilityEndpoint::Port(generator),
        UtilityEndpoint::Free(WorldPos { x: 24.0, y: 8.0 }),
    ));
    assert!(world.route(
        seq + 1,
        UtilityEndpoint::Port(workshop),
        UtilityEndpoint::Free(WorldPos { x: 18.0, y: 8.0 }),
    ));
}

#[test]
fn foreign_lines_edits_and_exposure_never_change_viewer_one_bytes() {
    let mut left = World2::new();
    let mut right = World2::new();
    left.app.update();
    right.app.update();
    assert_eq!(snapshot_frames(&left, 1), snapshot_frames(&right, 1));
    let mut left_base = left.baseline(1);
    let mut right_base = right.baseline(1);
    crossing_pair(&mut right, 10);
    // An explicit junction between the two crossing segments (segment ids 1 and 2 of slot 0).
    assert!(right.route(
        12,
        UtilityEndpoint::Split {
            segment: 1,
            point: WorldPos { x: 22.55, y: 10.0 },
        },
        UtilityEndpoint::Split {
            segment: 2,
            point: WorldPos { x: 20.9, y: 10.0 },
        },
    ));
    // The same paid terrain work in both worlds: dig the flat cell (10, 6) under the
    // generator segment down to the line elevation (-2).
    let cell = TerrainCell { x: 10, y: 6 };
    assert!(left.dig(20, cell, -2));
    assert!(right.dig(20, cell, -2));
    let mut right_saw_lines = false;
    let mut right_saw_disconnected = false;
    for tick in 0..600u64 {
        left.app.update();
        right.app.update();
        let left_next = left.baseline(1);
        let right_next = right.baseline(1);
        let left_payload = left_base.patch(
            &left_next,
            left.app.world().resource::<turts_sim::CommittedTick>(),
            SimSlotId(1),
            &turts_sim::visibility::event_units(left.app.world(), 1),
        );
        let right_payload = right_base.patch(
            &right_next,
            right.app.world().resource::<turts_sim::CommittedTick>(),
            SimSlotId(1),
            &turts_sim::visibility::event_units(right.app.world(), 1),
        );
        assert_eq!(
            frames(left.stamp(1, tick + 2), &left_payload),
            frames(right.stamp(1, tick + 2), &right_payload),
            "viewer 1 patch bytes differ at tick {tick}"
        );
        left_base = left_next;
        right_base = right_next;
        let own = right.baseline(0).state.records.utilities.unwrap();
        right_saw_lines |= own.segments.len() >= 3;
        right_saw_disconnected |= own
            .segments
            .iter()
            .any(|segment| segment.state == UtilitySegmentState::Disconnected);
    }
    assert!(right_saw_lines, "slot 0 placed no lines in the right world");
    assert!(
        right_saw_disconnected,
        "the dig did not expose the slot 0 line; the pair does not test exposure"
    );
    assert_eq!(snapshot_frames(&left, 1), snapshot_frames(&right, 1));
    let foreign = right.baseline(1).state.records.utilities.unwrap();
    assert_eq!(foreign, UtilityView::default(), "viewer 1 got slot 0 rows");
}

#[test]
fn owner_view_has_only_own_rows_and_deltas_carry_it_only_on_change() {
    let mut world = World2::new();
    world.app.update();
    let empty = world.baseline(0).state.records.utilities;
    assert_eq!(empty, Some(UtilityView::default()));
    crossing_pair(&mut world, 10);
    world.app.update();
    let before = world.baseline(0);
    let rows = before.state.records.utilities.clone().unwrap();
    assert_eq!(rows.segments.len(), 2);
    assert_eq!(rows.nodes.len(), 2);
    let own_units = before
        .state
        .units
        .iter()
        .filter(|unit| unit.owner_slot.0 == 0)
        .map(|unit| unit.unit_id)
        .collect::<Vec<_>>();
    for segment in &rows.segments {
        for end in [segment.a, segment.b] {
            if let UtilityEndRecord::Port(unit) = end {
                assert!(own_units.contains(&unit));
            }
        }
    }
    // Plan-view crossing does not connect: the workshop network has no provider.
    let workshop = world.unit(0, "utility_yard:workshop");
    world.app.update();
    let rows = world.baseline(0).state.records.utilities.unwrap();
    let network = rows
        .networks
        .iter()
        .find(|row| row.units.contains(&UnitId(workshop)))
        .unwrap();
    assert!(network.short && network.provided == 0 && network.consumed == 6);
    // A quiet tick sends no utility change in the delta.
    let quiet = world.baseline(0);
    world.app.update();
    let next = world.baseline(0);
    let payload = quiet.patch(
        &next,
        world.app.world().resource::<turts_sim::CommittedTick>(),
        SimSlotId(0),
        &turts_sim::visibility::event_units(world.app.world(), 0),
    );
    let StateTransactionPayload::Patch { state, .. } = payload else {
        panic!("quiet tick did not patch");
    };
    assert_eq!(state.records.utilities, None);
}

fn loam(world: &World2) -> u64 {
    world
        .baseline(0)
        .state
        .economy
        .balances
        .iter()
        .find(|row| row.resource == "utility_yard:loam")
        .map(|row| row.on_hand)
        .unwrap()
}

fn workshop_short(world: &World2) -> bool {
    let workshop = UnitId(world.unit(0, "utility_yard:workshop"));
    world
        .baseline(0)
        .state
        .records
        .utilities
        .unwrap()
        .networks
        .iter()
        .find(|row| row.units.contains(&workshop))
        .unwrap()
        .short
}

fn run_until(world: &mut World2, ticks: u32, what: &str, done: impl Fn(&World2) -> bool) {
    for _ in 0..ticks {
        world.app.update();
        if done(world) {
            return;
        }
    }
    panic!("{what} did not happen in {ticks} ticks");
}

/// UT2 and UT3 on the real public package: a plan-view crossing stays short, an explicit
/// split route powers the consumer, a paid dig exposes and disconnects the line, a paid fill
/// restores it, and the line elevation never moves.
#[test]
fn real_package_crossing_connection_exposure_and_paid_fill_keep_elevation_fixed() {
    let mut world = World2::new();
    world.app.update();
    let start_loam = loam(&world);
    crossing_pair(&mut world, 10);
    world.app.update();
    world.app.update();
    assert!(workshop_short(&world), "a plan-view crossing connected");
    assert_eq!(loam(&world), start_loam - 2, "each segment costs one loam");
    assert!(world.route(
        12,
        UtilityEndpoint::Split {
            segment: 1,
            point: WorldPos { x: 22.55, y: 10.0 },
        },
        UtilityEndpoint::Split {
            segment: 2,
            point: WorldPos { x: 20.9, y: 10.0 },
        },
    ));
    run_until(&mut world, 3, "split connection", |world| {
        !workshop_short(world)
    });
    let segments = |world: &World2| world.baseline(0).state.records.utilities.unwrap().segments;
    let elevations = |world: &World2| {
        segments(world)
            .iter()
            .map(|segment| (segment.id, segment.elevation))
            .collect::<Vec<_>>()
    };
    let fixed = elevations(&world);
    assert_eq!(fixed.len(), 5, "two splits and one new segment");
    assert!(fixed.iter().all(|(_, elevation)| *elevation == -2));
    let cell = TerrainCell { x: 10, y: 6 };
    let before_dig = loam(&world);
    assert!(world.dig(20, cell, -2));
    run_until(&mut world, 2400, "exposure by digging", |world| {
        segments(world)
            .iter()
            .any(|segment| segment.state == UtilitySegmentState::Disconnected)
    });
    assert!(loam(&world) > before_dig, "excavation credited no loam");
    run_until(&mut world, 3, "shortage after exposure", workshop_short);
    assert_eq!(elevations(&world), fixed, "digging moved a line");
    let porter = world.unit(0, "utility_yard:porter");
    let before_fill = loam(&world);
    assert!(world.order(
        30,
        "utility_yard:order_work",
        turts_mod_api::Behaviour::TerrainWork,
        turts_mod_api::TargetKind::TerrainDesignation,
        vec![porter],
        turts_sim::orders::OrderTargetSim::TerrainWork {
            tool: ContentId::parse("utility_yard:leveler").unwrap(),
            cells: TerrainRect {
                min: cell,
                max: TerrainCell {
                    x: cell.x + 1,
                    y: cell.y + 1,
                },
            },
            target_elevation: 1,
        },
    ));
    run_until(&mut world, 2400, "recovery by filling", |world| {
        segments(world)
            .iter()
            .all(|segment| segment.state == UtilitySegmentState::Conducting)
    });
    assert!(loam(&world) < before_fill, "fill was not paid");
    run_until(&mut world, 3, "supply after recovery", |world| {
        !workshop_short(world)
    });
    run_until(&mut world, 2400, "fill above the old surface", |world| {
        world
            .baseline(0)
            .state
            .records
            .terrain
            .unwrap()
            .surfaces
            .iter()
            .any(|row| row.cell == TerrainCell { x: 10, y: 6 } && row.current && row.elevation == 1)
    });
    assert_eq!(elevations(&world), fixed, "filling moved a line");
}

/// The AllVisible and `export_observation` delta path (`export::delta_for`) carries the
/// owner view only on the tick where it changed, and never for the other viewer. The route
/// enters through the in-tick command queue, as live commands do.
#[test]
fn committed_tick_deltas_carry_the_owner_view_only_when_it_changes() {
    let mut world = World2::new();
    world.app.update();
    let generator = world.unit(0, "utility_yard:generator");
    let depot = world.unit(0, "utility_yard:depot");
    world
        .app
        .world_mut()
        .resource_mut::<turts_sim::CommandQueue>()
        .0
        .push_back(turts_sim::Command {
            slot: SimSlotId(0),
            seq: 1,
            kind: turts_sim::CommandKind::Order(Box::new(turts_sim::orders::OrderCommand {
                order: ContentId::parse(ROUTE).unwrap(),
                behaviour: turts_mod_api::Behaviour::UtilityRoute,
                target_kind: turts_mod_api::TargetKind::UtilityRoute,
                policy: None,
                units: vec![depot],
                target: turts_sim::orders::OrderTargetSim::UtilityRoute(UtilityRouteTarget {
                    utility: LINE.into(),
                    depth: 2,
                    from: UtilityEndpoint::Port(generator),
                    to: UtilityEndpoint::Free(WorldPos { x: 24.0, y: 8.0 }),
                }),
                queue: turts_sim::orders::QueueMode::Replace,
                group: 0,
                allowed_units: vec!["utility_yard:depot".into()],
                queue_depth: 16,
            })),
        });
    let deltas = |world: &World2| {
        let batch = world.app.world().resource::<turts_sim::CommittedTick>();
        [0, 1].map(|slot| {
            crate::export::delta_for(batch, world.view(), SimSlotId(slot))
                .records
                .utilities
        })
    };
    world.app.update();
    let [own, foreign] = deltas(&world);
    assert_eq!(
        own.expect("no owner delta on the route tick")
            .segments
            .len(),
        1
    );
    assert_eq!(foreign, None, "viewer 1 got a utility delta");
    let mut quiet = false;
    for _ in 0..4 {
        world.app.update();
        let [own, foreign] = deltas(&world);
        assert_eq!(foreign, None, "viewer 1 got a utility delta");
        quiet |= own.is_none();
    }
    assert!(quiet, "the owner view was resent on every quiet tick");
    world.app.update();
    assert_eq!(deltas(&world), [None, None], "a quiet tick resent the view");
}
