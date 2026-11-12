//! The read-only world view that the client gives to the interaction core.

use super::*;

/// A unit record that is safe for the renderer-independent interaction core.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitView {
    pub id: UnitId,
    pub content_id: String,
    pub owner_slot: SlotId,
    pub position: Vec2,
    pub footprint_radius: f32,
    pub capabilities: Vec<Capability>,
    /// Dense catalog index cached by the client adapter to avoid per-frame id parsing.
    pub content_index: u32,
}

/// A resource-node record visible to the interaction core.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub id: NodeId,
    pub content_id: String,
    pub position: Vec2,
    pub radius: f32,
    /// Dense catalog index cached by the client adapter.
    pub content_index: u32,
}

/// Read-only view of the replicated world and package input/order definitions.
pub trait WorldView {
    fn units(&self) -> &[UnitView];
    fn nodes(&self) -> &[NodeView];
    fn own_slot(&self) -> SlotId;
    fn orders(&self) -> &[OrderDef];
    fn order_rules(&self) -> &OrderRulesDef;
    fn input_map(&self) -> &InputMapDef;
    fn runtime_binding_overrides(&self) -> &BTreeMap<String, Vec<turts_mod_api::Binding>> {
        static EMPTY: std::sync::LazyLock<BTreeMap<String, Vec<turts_mod_api::Binding>>> =
            std::sync::LazyLock::new(BTreeMap::new);
        &EMPTY
    }
    fn interaction_settings(&self) -> InteractionSettings {
        InteractionSettings::default()
    }
    fn camera_options(&self) -> camera::CameraOptions {
        camera::CameraOptions::default()
    }
    fn map_size(&self) -> Option<Vec2> {
        None
    }
    /// Authored content priority used to choose the preferred class in mixed boxes.
    fn selection_priority(&self) -> &[String];
    fn selection_rank(&self, content_id: &str) -> usize {
        self.selection_priority()
            .iter()
            .position(|priority| priority == content_id)
            .unwrap_or(usize::MAX)
    }
    /// Returns whether this unit can gather the item represented by the node.
    fn can_gather(&self, unit: &UnitView, node: &NodeView) -> bool;
    /// Whether the replicated/catalog view authorizes this unit to produce the requested content.
    fn can_produce(&self, _unit: &UnitView, _content_id: &str) -> bool {
        false
    }
    /// Whether this unit currently owns the requested queued job.
    fn has_job(&self, _unit: &UnitView, _job: JobId) -> bool {
        false
    }
    /// Returns whether this unit's weapon can target this foreign unit.
    fn can_weapon_target(&self, unit: &UnitView, target: &UnitView) -> bool;
    /// The world data of the placement rule (`UI-PLACE` fills `placement::Facts`).
    fn placement_facts(&self) -> &placement::Facts {
        static EMPTY: std::sync::LazyLock<placement::Facts> =
            std::sync::LazyLock::new(placement::Facts::default);
        &EMPTY
    }
    /// The world data of the hint conditions (`UI-HINT` fills `hints::Facts`).
    fn hint_facts(&self) -> &hints::Facts {
        static EMPTY: std::sync::LazyLock<hints::Facts> =
            std::sync::LazyLock::new(hints::Facts::default);
        &EMPTY
    }
}
