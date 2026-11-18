// M4b privacy: remote hidden bodies must not select an early visible path.
// This is a regression probe, not permission to remove closest-arrival rules.
use super::{Fixture, id};
use crate::{
    Command, CommandKind, MoveTarget, OrderCommand, OrderTargetSim, Position, QueueMode, SlotId,
    Vec2,
};
use turts_mod_api::*;

const GOAL: Vec2 = Vec2::new(45.0, 24.0);
const CONTROL_BODY: Vec2 = Vec2::new(60.0, 50.0);
const REMOTE_CLEARANCE: f32 = 16.0;

struct PairedWorld {
    fixture: Fixture,
    enemy: u32,
    move_definition: OrderDef,
}
impl PairedWorld {
    fn new(occupied: bool) -> Self {
        let mut move_definition = None;
        let mut fixture = Fixture::configured(20, 100, 20, |catalog, map| {
            move_definition = Some(catalog.orders[&id("order_move")].clone());
            map.size = MapSize { x: 64.0, y: 64.0 };
            map.visibility = Some(VisibilityDef {
                cell_size: 1.0,
                sharing: VisionSharing::OwnSlot,
                memory_limit: None,
            });
            let terrain = map.terrain.as_mut().unwrap();
            terrain.columns = 64;
            terrain.rows = 64;
            map.placements[0].pos = WorldPos { x: 8.0, y: 8.0 };
            map.placements[1].pos = WorldPos { x: 4.0, y: 4.0 };
            let mover = catalog.units.get_mut(&id("harvester")).unwrap();
            mover.movement = Some(MovementDef { speed: 3.0 });
            mover.vision = Some(VisionDef { radius: 4.0 });
            catalog.units.get_mut(&id("command_center")).unwrap().vision =
                Some(VisionDef { radius: 2.0 });
            let mut holder = catalog.units[&id("command_center")].clone();
            holder.id = id("hidden_holder");
            holder.name = "Hidden stationary holder".into();
            holder.footprint = Footprint::Circle { radius: 2.0 };
            holder.movement = None;
            holder.vision = None;
            holder.weapon = None;
            holder.acquire = None;
            catalog.units.insert(holder.id.clone(), holder);
            let point = if occupied { GOAL } else { CONTROL_BODY };
            map.placements.push(Placement {
                unit: id("hidden_holder").to_string(),
                owner_slot: 1,
                pos: WorldPos {
                    x: point.x,
                    y: point.y,
                },
                contents: vec![],
                facing: None,
                start_order: None,
            });
        });
        crate::visibility::initialize_context(fixture.app.world_mut(), 77, &[0, 1]).unwrap();
        crate::initialize_completed_world(fixture.app.world_mut());
        let point = if occupied { GOAL } else { CONTROL_BODY };
        let enemy = fixture.unit_at(point.x, point.y);
        Self {
            fixture,
            enemy,
            move_definition: move_definition.unwrap(),
        }
    }
    fn move_to(&mut self, point: Vec2, sequence: u32) {
        let order = &self.move_definition;
        let result = crate::apply_command(
            self.fixture.app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: sequence,
                kind: CommandKind::Order(Box::new(OrderCommand {
                    order: id("order_move"),
                    behaviour: order.behaviour,
                    target_kind: order.target,
                    policy: None,
                    units: vec![self.fixture.worker],
                    target: OrderTargetSim::Point(point),
                    queue: QueueMode::Replace,
                    group: 0,
                    allowed_units: order.units.clone(),
                    queue_depth: 16,
                })),
            },
        );
        assert!(
            result.outcome.is_ok(),
            "ordinary public Move was refused: {result:?}"
        );
        assert_eq!(result.units.accepted, vec![self.fixture.worker]);
    }
    fn own_poses(&self) -> Vec<(u32, Vec2)> {
        [self.fixture.worker, self.fixture.depot]
            .into_iter()
            .map(|unit| {
                let entity = crate::unit_entity(self.fixture.app.world(), unit).unwrap();
                (
                    unit,
                    self.fixture.app.world().get::<Position>(entity).unwrap().0,
                )
            })
            .collect()
    }
    fn own_targets(&self) -> Vec<(u32, Option<Vec2>)> {
        [self.fixture.worker, self.fixture.depot]
            .into_iter()
            .map(|unit| {
                let entity = crate::unit_entity(self.fixture.app.world(), unit).unwrap();
                (
                    unit,
                    self.fixture
                        .app
                        .world()
                        .get::<MoveTarget>(entity)
                        .map(|target| target.0),
                )
            })
            .collect()
    }
    fn prove_remote_and_unobserved(&self, tick: u32) {
        let world = self.fixture.app.world();
        let basis = crate::visibility::basis(world, ObservationPhase::Completed).unwrap();
        assert!(
            !basis.unit_permitted(0, self.enemy),
            "enemy became observed before comparison tick {tick}"
        );
        let knowledge = basis.knowledge(0).unwrap();
        assert!(
            !knowledge
                .remembered_units
                .iter()
                .any(|unit| unit.unit == self.enemy),
            "enemy entered stale history before comparison tick {tick}"
        );
        let entity = crate::unit_entity(world, self.enemy).unwrap();
        let enemy = world.get::<Position>(entity).unwrap().0;
        assert!(!basis.point_observed(
            0,
            WorldPos {
                x: enemy.x,
                y: enemy.y
            }
        ));
        assert!(!basis.point_observed(
            0,
            WorldPos {
                x: GOAL.x,
                y: GOAL.y
            }
        ));
        for (_, own) in self.own_poses() {
            assert!(
                own.distance(enemy) > REMOTE_CLEARANCE,
                "remote-body premise failed at tick {tick}"
            );
            // The sum of the mover/depot and enemy radii is only 2.25.
            assert!(
                own.distance(enemy) >= 2.25,
                "hidden holder overlaps an owned body"
            );
        }
        let phase = super::phase(world, ObservationPhase::Completed).unwrap();
        assert!(
            phase
                .observe_cell(0, TerrainCell { x: 45, y: 24 })
                .is_none(),
            "goal has a current permitted terrain observation"
        );
        assert!(
            phase.surface(0, TerrainCell { x: 45, y: 24 }).is_none(),
            "goal has a current or stale geological surface record"
        );
    }
}
fn prove_same_permitted_geometry(left: &PairedWorld, right: &PairedWorld, tick: u32) {
    left.prove_remote_and_unobserved(tick);
    right.prove_remote_and_unobserved(tick);
    let a =
        crate::visibility::basis(left.fixture.app.world(), ObservationPhase::Completed).unwrap();
    let b =
        crate::visibility::basis(right.fixture.app.world(), ObservationPhase::Completed).unwrap();
    // Includes the actual fog masks, remembered identities, observation ages and retention flag.
    assert_eq!(
        a.knowledge(0),
        b.knowledge(0),
        "permitted fog/stale-history premise differs at tick {tick}"
    );
    let a = super::phase(left.fixture.app.world(), ObservationPhase::Completed).unwrap();
    let b = super::phase(right.fixture.app.world(), ObservationPhase::Completed).unwrap();
    for y in 0..64 {
        for x in 0..64 {
            let cell = TerrainCell { x, y };
            assert_eq!(
                a.initial_elevation(cell),
                b.initial_elevation(cell),
                "public initial geometry differs at ({x},{y}), tick {tick}"
            );
            assert_eq!(
                a.surface(0, cell),
                b.surface(0, cell),
                "permitted current/stale geometry differs at ({x},{y}), tick {tick}"
            );
        }
    }
}

#[test]
fn distant_unseen_stationary_goal_holder_cannot_redirect_early_owned_movement() {
    let mut clear = PairedWorld::new(false);
    let mut occupied = PairedWorld::new(true);
    assert_eq!(clear.fixture.worker, occupied.fixture.worker);
    assert_eq!(clear.fixture.depot, occupied.fixture.depot);
    prove_same_permitted_geometry(&clear, &occupied, 0);
    assert_eq!(clear.own_poses(), occupied.own_poses());
    // Establish actual stale terrain history with the same legal prior movement.
    // The distant goal is still unknown in both worlds throughout this precondition.
    clear.move_to(Vec2::new(10.0, 8.0), 1);
    occupied.move_to(Vec2::new(10.0, 8.0), 1);
    for tick in 1..=20 {
        clear.fixture.tick();
        occupied.fixture.tick();
        prove_same_permitted_geometry(&clear, &occupied, tick);
        assert_eq!(
            clear.own_poses(),
            occupied.own_poses(),
            "paired public precondition differs at tick {tick}"
        );
    }
    let before = super::phase(clear.fixture.app.world(), ObservationPhase::Completed).unwrap();
    let stale = (0..64)
        .flat_map(|y| (0..64).map(move |x| TerrainCell { x, y }))
        .filter_map(|cell| before.surface(0, cell))
        .filter(|row| !row.current)
        .count();
    assert!(
        stale > 0,
        "the paired premise must include real stale surface observations"
    );
    clear.move_to(GOAL, 2);
    occupied.move_to(GOAL, 2);
    let initial = clear.own_poses();
    let mut advanced = false;
    for tick in 1..=30 {
        // Prove equal legal inputs before every compared fixed tick.
        prove_same_permitted_geometry(&clear, &occupied, 20 + tick - 1);
        clear.fixture.tick();
        occupied.fixture.tick();
        clear.prove_remote_and_unobserved(20 + tick);
        occupied.prove_remote_and_unobserved(20 + tick);
        assert_eq!(
            clear.own_poses(),
            occupied.own_poses(),
            "hidden remote goal occupancy redirected an actual owned pose at tick {tick}"
        );
        prove_same_permitted_geometry(&clear, &occupied, 20 + tick);
        assert_eq!(
            clear.own_targets(),
            occupied.own_targets(),
            "hidden remote goal occupancy changed the owned movement target at tick {tick}"
        );
        let own_orders = |pair: &PairedWorld| {
            crate::completed_world(pair.fixture.app.world())
                .unit_orders
                .iter()
                .filter(|row| row.owner_slot == 0)
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(
            own_orders(&clear),
            own_orders(&occupied),
            "hidden remote goal occupancy changed the owner's public target/activity at tick {tick}"
        );
        advanced |= clear.own_poses() != initial;
    }
    assert!(
        advanced,
        "paired worlds must execute real movement rather than compare idle actors"
    );
}
