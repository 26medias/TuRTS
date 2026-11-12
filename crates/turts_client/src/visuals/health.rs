//! Health bars: the player-facing display mode, the visibility rule, and the bar geometry.

use super::materials::world_units_for_pixels;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthMode {
    Selected,
    DamagedOrSelected,
    Always,
}

/// Player-facing health-bar visibility. Task16 settings can write this resource directly.
#[derive(Resource, Debug, Clone, Copy)]
pub struct HealthDisplay(pub HealthMode);

impl Default for HealthDisplay {
    fn default() -> Self {
        Self(HealthMode::DamagedOrSelected)
    }
}

pub(super) fn health_bar_visible(mode: HealthMode, selected: bool, current: u64, max: u64) -> bool {
    match mode {
        HealthMode::Selected => selected,
        HealthMode::DamagedOrSelected => selected || current < max,
        HealthMode::Always => true,
    }
}

pub(super) fn health_fraction(current: u64, max: u64) -> Option<f32> {
    (max > 0).then(|| (current as f32 / max as f32).clamp(0.0, 1.0))
}

pub(super) fn update_health_fill_transform(
    transform: &mut Mut<'_, Transform>,
    fraction: f32,
) -> bool {
    // Read through `Deref` first; the first mutable field access below marks the Bevy component
    // changed, so it must only happen after the geometry comparison succeeds.
    if (transform.scale.x - fraction).abs() <= f32::EPSILON {
        return false;
    }
    transform.scale.x = fraction;
    transform.translation.x = -(1.0 - fraction) * 0.45;
    true
}

pub(super) fn sync_health_bars(
    input: HealthBarSync<'_>,
    desired: &mut HashMap<VisualId, (Handle<Mesh>, VisualMaterial, Transform)>,
    fractions: &mut HashMap<u32, f32>,
) {
    let HealthBarSync {
        healths,
        presented,
        catalog,
        selected,
        mode,
        assets,
        camera,
        camera_transform,
        generation,
    } = input;
    let (Some(mesh), Some(background), Some(fill)) =
        (&assets.bar, &assets.bar_background, &assets.bar_health)
    else {
        return;
    };
    for health in healths {
        let Some(unit) = presented.get(&health.unit.0) else {
            continue;
        };
        let Some(def) = ContentId::parse(&unit.content_id)
            .ok()
            .and_then(|id| catalog.units.get(&id))
        else {
            continue;
        };
        let Some(max) = def.health.map(|health| health.max).filter(|max| *max > 0) else {
            continue;
        };
        let is_selected = selected.contains(&health.unit.0);
        if !health_bar_visible(mode, is_selected, health.current, max) {
            continue;
        }
        let position = world_to_scene(unit.pos) + Vec3::Y * 2.2;
        let size =
            world_units_for_pixels(camera, camera_transform, position, assets.health_bar_size);
        let id = VisualId {
            kind: VisualKind::HealthBackground,
            unit: health.unit.0,
            ordinal: 0,
            generation,
            identity: 0,
        };
        desired.insert(
            id,
            (
                mesh.clone(),
                background.clone().into(),
                Transform {
                    translation: position,
                    scale: Vec3::new(size.x, size.y, 0.1),
                    ..default()
                },
            ),
        );
        let Some(fraction) = health_fraction(health.current, max) else {
            continue;
        };
        let _ = fill;
        fractions.insert(health.unit.0, fraction);
    }
}
