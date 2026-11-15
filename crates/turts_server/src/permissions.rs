//! Shared public-observation decisions. Ownership remains a separate privacy check.
//!
//! Production still uses all-visible public geometry. This interface supplies no fog,
//! remembered sightings, death inference, or private enemy economy information.

use turts_mod_api::WorldPos;

/// Coverage describes permitted knowledge, not the presence of an unseen blocker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObservationArea {
    pub min: WorldPos,
    pub max: WorldPos,
}

/// A trusted host policy. Query implementations receive only its projected facts.
pub trait ObservationPermissions {
    fn unit_visible(&self, viewer: u8, unit: u32, owner: u8) -> bool;
    fn node_visible(&self, viewer: u8, node: u32) -> bool;
    fn area_observed(&self, viewer: u8, area: ObservationArea) -> bool;

    /// Immutable slot knowledge from this same phase, never a newer visibility lookup.
    fn knowledge(&self, _viewer: u8) -> Option<turts_mod_api::KnowledgeSnapshot> {
        None
    }

    /// Allows the unchanged production projection to avoid an extra filtered copy.
    fn all_public_geometry(&self, _viewer: u8) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AllVisible;

impl ObservationPermissions for AllVisible {
    fn unit_visible(&self, _: u8, _: u32, _: u8) -> bool {
        true
    }

    fn node_visible(&self, _: u8, _: u32) -> bool {
        true
    }

    fn area_observed(&self, _: u8, _: ObservationArea) -> bool {
        true
    }

    fn all_public_geometry(&self, _: u8) -> bool {
        true
    }
}

/// A participant always retains its own state. The policy filters foreign public facts.
pub fn unit_permitted(
    policy: &dyn ObservationPermissions,
    viewer: u8,
    unit: u32,
    owner: u8,
) -> bool {
    owner == viewer || policy.unit_visible(viewer, unit, owner)
}

/// The single trusted observation policy shared by human exports and bot observations.
#[derive(bevy::prelude::Resource, Clone)]
pub struct ObservationPolicy(pub std::sync::Arc<dyn ObservationPermissions + Send + Sync>);

impl Default for ObservationPolicy {
    fn default() -> Self {
        Self(std::sync::Arc::new(AllVisible))
    }
}

struct AuthoritativePermissions(std::sync::Arc<turts_sim::visibility::VisibilityBasis>);
impl ObservationPermissions for AuthoritativePermissions {
    fn unit_visible(&self, viewer: u8, unit: u32, _owner: u8) -> bool {
        self.0.unit_permitted(viewer, unit)
    }
    fn node_visible(&self, viewer: u8, node: u32) -> bool {
        self.0.node_permitted(viewer, node)
    }
    fn area_observed(&self, viewer: u8, area: ObservationArea) -> bool {
        self.0.area_observed(viewer, area.min, area.max)
    }
    fn knowledge(&self, viewer: u8) -> Option<turts_mod_api::KnowledgeSnapshot> {
        self.0.knowledge(viewer)
    }
}
/// One authoritative completed basis for both human publication and bot candidate projection.
pub fn completed(world: &bevy::prelude::World) -> ObservationPolicy {
    match turts_sim::visibility::basis(world, turts_mod_api::ObservationPhase::Completed) {
        Some(basis) => ObservationPolicy(std::sync::Arc::new(AuthoritativePermissions(basis))),
        None => world
            .get_resource::<ObservationPolicy>()
            .cloned()
            .unwrap_or_default(),
    }
}
