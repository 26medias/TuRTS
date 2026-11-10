use std::path::PathBuf;

use glam::Vec2;
use turts_interaction::{
    Button, CameraState, Core, InputEvent, NodeView, UnitView, Viewport, WorldView,
};
use turts_mod_api::{Capability, ContentId, Footprint, InputMapDef, OrderDef, OrderRulesDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{NodeId, OrderTarget, Pos, SlotId, UnitId};

const VIEW: Viewport = Viewport {
    size: Vec2::new(1280.0, 720.0),
    scale_factor: 1.0,
};

struct MusterWorld {
    units: Vec<UnitView>,
    nodes: Vec<NodeView>,
    orders: Vec<OrderDef>,
    rules: OrderRulesDef,
    input: InputMapDef,
    priority: Vec<String>,
    size: Vec2,
}

impl MusterWorld {
    fn authored() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
        let archive = PackageArchive::parse(&pack_dir(&root).expect("pack authored Muster"))
            .expect("parse authored Muster");
        let catalog = archive.content().expect("resolve authored catalog");
        let (_, map) = archive
            .maps()
            .expect("read authored maps")
            .into_iter()
            .find(|(path, _)| path == "maps/field.ron")
            .expect("authored field map");
        let units = map
            .placements
            .iter()
            .enumerate()
            .map(|(index, placement)| {
                let content_id = placement.unit.to_string();
                let definition = catalog
                    .units
                    .get(&ContentId::parse(&content_id).expect("unit content id"))
                    .expect("authored unit definition");
                let mut capabilities = Vec::new();
                if definition.movement.is_some() {
                    capabilities.push(Capability::Movement);
                }
                if definition.weapon.is_some() {
                    capabilities.push(Capability::Weapon);
                }
                if definition.gatherer.is_some() {
                    capabilities.push(Capability::Gatherer);
                }
                if definition.producer.is_some() {
                    capabilities.push(Capability::Producer);
                }
                if definition.research.is_some() {
                    capabilities.push(Capability::Research);
                }
                let Footprint::Circle { radius } = definition.footprint;
                UnitView {
                    id: UnitId(index as u32 + 1),
                    content_id,
                    owner_slot: SlotId(placement.owner_slot),
                    position: Vec2::new(placement.pos.x, placement.pos.y),
                    footprint_radius: radius,
                    capabilities,
                    content_index: index as u32,
                }
            })
            .collect();
        let nodes = map
            .nodes
            .iter()
            .enumerate()
            .map(|(index, placement)| {
                let content_id = placement.node.to_string();
                let definition = catalog
                    .nodes
                    .get(&ContentId::parse(&content_id).expect("node content id"))
                    .expect("authored node definition");
                NodeView {
                    id: NodeId(index as u32 + 1),
                    content_id,
                    position: Vec2::new(placement.pos.x, placement.pos.y),
                    radius: definition.radius,
                    content_index: index as u32,
                }
            })
            .collect();
        let orders = catalog
            .orders
            .iter()
            .map(|(id, order)| {
                let mut order = order.clone();
                order.id = id.to_string();
                order
            })
            .collect();
        let rules = archive
            .order_rules()
            .expect("read authored smart rules")
            .into_iter()
            .next()
            .expect("authored smart rules")
            .1;
        let input = archive
            .input_maps()
            .expect("read authored input maps")
            .into_iter()
            .next()
            .expect("authored input map")
            .1;
        Self {
            units,
            nodes,
            orders,
            rules,
            input,
            priority: vec!["muster:trooper".into(), "muster:worker".into()],
            size: Vec2::new(map.size.x, map.size.y),
        }
    }
}

impl WorldView for MusterWorld {
    fn units(&self) -> &[UnitView] {
        &self.units
    }
    fn nodes(&self) -> &[NodeView] {
        &self.nodes
    }
    fn own_slot(&self) -> SlotId {
        SlotId(0)
    }
    fn orders(&self) -> &[OrderDef] {
        &self.orders
    }
    fn order_rules(&self) -> &OrderRulesDef {
        &self.rules
    }
    fn input_map(&self) -> &InputMapDef {
        &self.input
    }
    fn map_size(&self) -> Option<Vec2> {
        Some(self.size)
    }
    fn selection_priority(&self) -> &[String] {
        &self.priority
    }
    fn can_gather(&self, unit: &UnitView, node: &NodeView) -> bool {
        unit.content_id == "muster:worker" && node.content_id == "muster:ore_node"
    }
    fn can_weapon_target(&self, unit: &UnitView, target: &UnitView) -> bool {
        unit.content_id == "muster:trooper"
            && target.owner_slot != unit.owner_slot
            && unit.capabilities.contains(&Capability::Weapon)
    }
}

fn pointer(
    core: &mut Core,
    world: &MusterWorld,
    button: Button,
    point: Vec2,
    now: u64,
) -> Vec<turts_interaction::Effect> {
    let px = core.project_world_to_screen(point, &VIEW);
    core.step(
        &[
            InputEvent::PointerDown {
                button,
                px,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button,
                px,
                over_widget: false,
            },
        ],
        world,
        &VIEW,
        now,
    )
}

fn select_mixed(core: &mut Core, world: &MusterWorld, worker: UnitId, trooper: UnitId) {
    let worker_pos = world
        .units
        .iter()
        .find(|unit| unit.id == worker)
        .unwrap()
        .position;
    let trooper_pos = world
        .units
        .iter()
        .find(|unit| unit.id == trooper)
        .unwrap()
        .position;
    for position in [worker_pos, trooper_pos] {
        let px = core.project_world_to_screen(position, &VIEW);
        assert!(
            px.x >= 0.0 && px.y >= 0.0 && px.x < VIEW.size.x && px.y < VIEW.size.y,
            "authored Muster unit projects inside the real viewport: {position:?} -> {px:?}"
        );
    }
    pointer(core, world, Button::Left, worker_pos, 10);
    core.step(
        &[InputEvent::KeyDown {
            code: "ShiftLeft".into(),
            mods: vec![turts_mod_api::Mod::Shift],
        }],
        world,
        &VIEW,
        20,
    );
    pointer(core, world, Button::Left, trooper_pos, 30);
    core.step(
        &[InputEvent::KeyUp {
            code: "ShiftLeft".into(),
        }],
        world,
        &VIEW,
        40,
    );
}

fn sent(effects: &[turts_interaction::Effect]) -> Vec<&turts_net_protocol::OrderEnvelope> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            turts_interaction::Effect::Send(order) => Some(order),
            _ => None,
        })
        .collect()
}

#[test]
fn authored_muster_mixed_selection_resolves_ground_node_and_foreign_unit_per_subject() {
    let world = MusterWorld::authored();
    let worker = world
        .units
        .iter()
        .find(|unit| unit.content_id == "muster:worker" && unit.owner_slot == SlotId(0))
        .unwrap()
        .id;
    let trooper = world
        .units
        .iter()
        .find(|unit| {
            unit.content_id == "muster:trooper"
                && unit.owner_slot == SlotId(0)
                && unit.position.x == 32.7
                && unit.position.y == 22.4
        })
        .unwrap()
        .id;
    let node = world.nodes.first().expect("authored ore node");
    let enemy = world
        .units
        .iter()
        .find(|unit| unit.content_id == "muster:dummy" && unit.owner_slot == SlotId(1))
        .unwrap();

    for (target, expected_worker, expected_trooper) in [
        (
            Vec2::new(18.0, 10.0),
            "muster:order_move",
            "muster:order_move",
        ),
        (node.position, "muster:order_gather", "muster:order_move"),
        (enemy.position, "muster:order_move", "muster:order_attack"),
    ] {
        let mut core = Core::new(CameraState {
            look_at: Vec2::new(28.85, 20.2),
            height: 38.0,
            pitch: 0.7,
            yaw: 0.0,
            vertical_fov: 60.0_f32.to_radians(),
        });
        select_mixed(&mut core, &world, worker, trooper);
        assert_eq!(
            core.selected_units(),
            &[worker, trooper],
            "the physical selection uses the authored worker and trooper"
        );
        let mut effects = Vec::new();
        core.smart_command_at_world_point(target, &world, &mut effects);
        let envelopes = sent(&effects);
        assert_eq!(
            envelopes.len(),
            if expected_worker == expected_trooper {
                1
            } else {
                2
            },
            "the authored gesture must group only subjects with the same resolved order and target"
        );
        let worker_order = envelopes
            .iter()
            .find(|order| order.units.contains(&worker))
            .expect("worker order");
        let trooper_order = envelopes
            .iter()
            .find(|order| order.units.contains(&trooper))
            .expect("trooper order");
        assert_eq!(worker_order.order, expected_worker);
        assert_eq!(trooper_order.order, expected_trooper);
        assert_eq!(
            worker_order.group, trooper_order.group,
            "per-subject orders from one smart gesture retain a common nonzero group"
        );
        if expected_worker != expected_trooper {
            assert_ne!(worker_order.group, 0);
        }
        match target {
            point if point == node.position => {
                assert_eq!(worker_order.target, OrderTarget::Node(node.id));
                assert_eq!(
                    trooper_order.target,
                    OrderTarget::Point(Pos {
                        x: node.position.x,
                        y: node.position.y
                    })
                );
            }
            point if point == enemy.position => {
                assert_eq!(worker_order.target, OrderTarget::Unit(enemy.id));
                assert_eq!(trooper_order.target, OrderTarget::Unit(enemy.id));
            }
            _ => assert!(
                envelopes
                    .iter()
                    .all(|order| matches!(order.target, OrderTarget::Point(_)))
            ),
        }
    }
}
