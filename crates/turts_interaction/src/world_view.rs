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

/// A hit on the permitted plateau model. Wall hits retain their owning top level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceHit {
    pub position: Vec2,
    pub height: f32,
    pub elevation: Option<i32>,
    pub cell: Option<turts_mod_api::TerrainCell>,
    pub parameter: f32,
    pub current: bool,
}

/// Read-only view of the replicated world and package input/order definitions.
pub trait WorldView {
    fn units(&self) -> &[UnitView];
    fn nodes(&self) -> &[NodeView];
    /// The visible body and its scene yaw. Missing presentation keeps footprint picking.
    fn unit_pick_shape(&self, _unit: &UnitView) -> Option<(&turts_mod_api::Shape, f32)> {
        None
    }
    fn node_pick_shape(&self, _node: &NodeView) -> Option<&turts_mod_api::Shape> {
        None
    }
    /// Returns a flat, fully observed support plane or an explicit public refusal.
    fn terrain_foundation(&self, _point: Vec2, _radius: f32) -> Result<f32, &'static str> {
        Ok(0.)
    }
    fn terrain_enabled(&self) -> bool {
        false
    }
    fn terrain_active_order(
        &self,
        _unit: UnitId,
    ) -> Option<(&str, &turts_mod_api::TerrainDesignation)> {
        None
    }
    fn can_unload(&self, _unit: &UnitView, _target: &UnitView) -> bool {
        false
    }
    fn terrain_identity(&self) -> Option<(u64, turts_mod_api::ContentId)> {
        None
    }
    fn terrain_grid(&self) -> Option<turts_rules::terrain::geometry::Grid> {
        None
    }
    fn terrain_current(&self, _cell: turts_mod_api::TerrainCell) -> bool {
        false
    }
    fn terrain_elevation(&self, _cell: turts_mod_api::TerrainCell) -> Option<i32> {
        None
    }
    fn terrain_tool(
        &self,
        _unit: &UnitView,
        _tool: &str,
    ) -> Option<&turts_mod_api::ResolvedTerrainTool> {
        None
    }
    fn camera_ground_height(&self, _point: Vec2) -> f32 {
        0.
    }
    fn unit_ground_height(&self, _unit: &UnitView) -> Option<f32> {
        Some(0.)
    }
    fn node_ground_height(&self, _node: &NodeView) -> Option<f32> {
        Some(0.)
    }
    /// None is an unobstructed ray outside the map; Err is an uncertified query.
    #[allow(
        clippy::result_unit_err,
        reason = "This geometric predicate returns hit, clear or uncertified without an error payload"
    )]
    fn surface_occlusion(
        &self,
        origin: glam::Vec3,
        direction: glam::Vec3,
    ) -> Result<Option<f32>, ()> {
        Ok(self.surface_hit(origin, direction).map(|hit| hit.parameter))
    }
    fn surface_hit(&self, origin: glam::Vec3, direction: glam::Vec3) -> Option<SurfaceHit> {
        if !origin.is_finite() || !direction.is_finite() || direction.y.abs() < 1e-6 {
            return None;
        }
        let parameter = -origin.y / direction.y;
        if !parameter.is_finite() || parameter < 0. {
            return None;
        }
        let point = origin + direction * parameter;
        Some(SurfaceHit {
            position: Vec2::new(point.x, point.z),
            height: 0.,
            elevation: None,
            cell: None,
            parameter,
            current: true,
        })
    }
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
    /// Whether the actor can repair this complete, positive-health damaged target.
    /// The client adapter applies authored relation and armour-class filters.
    fn can_repair(&self, _unit: &UnitView, _target: &UnitView) -> bool {
        false
    }
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
