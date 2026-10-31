use glam::{Vec2, vec2};
use std::collections::BTreeMap;
use turts_interaction::{
    Core, InputEvent, InteractionSettings, NodeView, UnitView, Viewport, WorldView,
    camera::CameraOptions,
};
use turts_mod_api::{
    Behaviour, Binding, Button, Capability, InputMapDef, Mod, OrderDef, OrderRulesDef, SmartOrder,
    SmartTarget, TargetKind,
};
use turts_net_protocol::{NodeId, SlotId, UnitId};

pub const OWN: SlotId = SlotId(0);
pub const OTHER: SlotId = SlotId(1);
pub const VIEW: Viewport = Viewport {
    size: vec2(1280.0, 720.0),
    scale_factor: 1.0,
};

pub struct TestWorld {
    pub units: Vec<UnitView>,
    pub nodes: Vec<NodeView>,
    pub rules: OrderRulesDef,
    pub orders: Vec<OrderDef>,
    pub input: InputMapDef,
    pub selection_priority: Vec<String>,
    pub runtime_bindings: BTreeMap<String, Vec<Binding>>,
    pub interaction: InteractionSettings,
    pub camera_options: CameraOptions,
    pub map_size: Option<Vec2>,
}

impl Default for TestWorld {
    fn default() -> Self {
        Self {
            units: vec![
                unit(
                    1,
                    "forge:drone",
                    OWN,
                    vec2(0.0, 0.0),
                    &[Capability::Movement, Capability::Weapon],
                ),
                unit(
                    2,
                    "forge:drone",
                    OWN,
                    vec2(6.0, 0.0),
                    &[Capability::Movement, Capability::Weapon],
                ),
                unit(3, "forge:drill", OWN, vec2(0.0, 6.0), &[]),
                unit(
                    4,
                    "muster:trooper",
                    OTHER,
                    vec2(12.0, 0.0),
                    &[Capability::Movement, Capability::Weapon],
                ),
                unit(
                    5,
                    "forge:gatherer",
                    OWN,
                    vec2(18.0, 0.0),
                    &[Capability::Movement, Capability::Gatherer],
                ),
            ],
            nodes: vec![NodeView {
                id: NodeId(70),
                content_id: "forge:ore_node".into(),
                position: vec2(24.0, 0.0),
                radius: 2.0,
                content_index: 0,
            }],
            rules: OrderRulesDef {
                queue_depth: 16,
                smart: vec![
                    SmartOrder {
                        target: SmartTarget::ForeignUnitWeaponTarget,
                        needs: Capability::Weapon,
                        order: "forge:order_attack".into(),
                    },
                    SmartOrder {
                        target: SmartTarget::ResourceNodeAccepted,
                        needs: Capability::Gatherer,
                        order: "forge:order_gather".into(),
                    },
                    SmartOrder {
                        target: SmartTarget::Ground,
                        needs: Capability::Movement,
                        order: "forge:order_move".into(),
                    },
                    SmartOrder {
                        target: SmartTarget::OwnUnit,
                        needs: Capability::Movement,
                        order: "forge:order_move".into(),
                    },
                    SmartOrder {
                        target: SmartTarget::Node,
                        needs: Capability::Movement,
                        order: "forge:order_move".into(),
                    },
                    SmartOrder {
                        target: SmartTarget::ForeignUnit,
                        needs: Capability::Movement,
                        order: "forge:order_move".into(),
                    },
                ],
            },
            orders: vec![
                order("forge:order_move", Behaviour::Move, TargetKind::PointOrUnit),
                order("forge:order_attack", Behaviour::Attack, TargetKind::Unit),
                order("forge:order_gather", Behaviour::Gather, TargetKind::Node),
                order("forge:order_stop", Behaviour::Stop, TargetKind::None),
            ],
            input: InputMapDef {
                actions: vec![],
                engine_overrides: vec![],
            },
            selection_priority: vec![
                "forge:drone".into(),
                "forge:gatherer".into(),
                "forge:drill".into(),
            ],
            runtime_bindings: BTreeMap::new(),
            interaction: InteractionSettings::default(),
            camera_options: CameraOptions::default(),
            map_size: Some(vec2(64.0, 64.0)),
        }
    }
}

impl WorldView for TestWorld {
    fn units(&self) -> &[UnitView] {
        &self.units
    }
    fn nodes(&self) -> &[NodeView] {
        &self.nodes
    }
    fn own_slot(&self) -> SlotId {
        OWN
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
    fn runtime_binding_overrides(&self) -> &BTreeMap<String, Vec<Binding>> {
        &self.runtime_bindings
    }
    fn interaction_settings(&self) -> InteractionSettings {
        self.interaction
    }
    fn camera_options(&self) -> CameraOptions {
        self.camera_options.clone()
    }
    fn map_size(&self) -> Option<Vec2> {
        self.map_size
    }
    fn selection_priority(&self) -> &[String] {
        &self.selection_priority
    }
    fn can_gather(&self, unit: &UnitView, node: &NodeView) -> bool {
        unit.capabilities.contains(&Capability::Gatherer) && node.content_id == "forge:ore_node"
    }
    fn can_weapon_target(&self, unit: &UnitView, target: &UnitView) -> bool {
        unit.capabilities.contains(&Capability::Weapon) && target.owner_slot != unit.owner_slot
    }
}

pub fn unit(
    id: u32,
    content_id: &str,
    owner_slot: SlotId,
    position: Vec2,
    capabilities: &[Capability],
) -> UnitView {
    UnitView {
        id: UnitId(id),
        content_id: content_id.into(),
        owner_slot,
        position,
        footprint_radius: 0.5,
        capabilities: capabilities.to_vec(),
        content_index: 0,
    }
}

fn order(id: &str, behaviour: Behaviour, target: TargetKind) -> OrderDef {
    let (_, short_id) = id
        .split_once(':')
        .expect("fixture order ids have a package prefix");
    OrderDef {
        id: id.into(),
        name: short_id.into(),
        behaviour,
        target,
        fire: None,
        engage: None,
        icon: format!("icon_{short_id}"),
        hotkey: format!("key_{short_id}"),
        cursor: None,
        card: None,
        units: vec!["forge:drone".into(), "forge:gatherer".into()],
        description: String::new(),
    }
}

pub fn click(
    core: &mut Core,
    world: &TestWorld,
    button: Button,
    px: Vec2,
    _mods: Vec<Mod>,
    now_ms: u64,
) -> Vec<turts_interaction::Effect> {
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
        now_ms,
    )
}

#[allow(dead_code)] // Shared only by the integration-test crates that assert selection.
pub fn ids(core: &Core) -> Vec<UnitId> {
    let mut ids = core.selected_units().to_vec();
    ids.sort_by_key(|id| id.0);
    ids
}

#[allow(dead_code)] // Shared only by the integration-test crates that assert emitted orders.
pub fn sent(effects: &[turts_interaction::Effect]) -> Vec<&turts_net_protocol::OrderEnvelope> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            turts_interaction::Effect::Send(envelope) => Some(envelope),
            _ => None,
        })
        .collect()
}

pub fn select_world_point(core: &mut Core, world: &TestWorld, id: UnitId, now_ms: u64) {
    // Test input coordinates use the deterministic projection fixture required in contract.md.
    let px = core.project_world_to_screen(
        world
            .units
            .iter()
            .find(|unit| unit.id == id)
            .unwrap()
            .position,
        &VIEW,
    );
    click(core, world, Button::Left, px, vec![], now_ms);
}
