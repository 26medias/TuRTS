//! White copies are shared by source material. Original team materials stay unchanged.
use super::*;
#[derive(Resource, Default)]
pub struct FlashMaterials {
    white: HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>,
    originals: HashMap<Entity, Handle<StandardMaterial>>,
}
#[derive(Resource, Clone, Copy)]
pub struct EffectSettings {
    pub flashes: bool,
    pub shake: Shake,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Shake {
    Off,
    #[default]
    Half,
    Full,
}
impl Shake {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Half => "Half",
            Self::Full => "Full",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Half,
            Self::Half => Self::Full,
            Self::Full => Self::Off,
        }
    }
    pub fn gain(self) -> f32 {
        match self {
            Self::Off => 0.,
            Self::Half => 0.5,
            Self::Full => 1.,
        }
    }
}
impl Default for EffectSettings {
    fn default() -> Self {
        Self {
            flashes: true,
            shake: Shake::Half,
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub fn flash_models(
    fx: Res<CombatPresentation>,
    settings: Res<EffectSettings>,
    roots: Query<(Entity, &crate::render::PresentedUnit)>,
    parents: Query<&ChildOf>,
    mut meshes: Query<(Entity, &mut MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<FlashMaterials>,
) {
    let roots: HashMap<_, _> = roots.iter().map(|(entity, id)| (entity, id.0)).collect();
    let mut live = HashSet::new();
    for (entity, mut material) in &mut meshes {
        live.insert(entity);
        let original = cache
            .originals
            .get(&entity)
            .cloned()
            .unwrap_or_else(|| material.0.clone());
        let mut cursor = entity;
        let mut unit = None;
        for _ in 0..130 {
            if let Some(id) = roots.get(&cursor) {
                unit = Some(*id);
                break;
            }
            let Ok(parent) = parents.get(cursor) else {
                break;
            };
            cursor = parent.parent();
        }
        let flashing = settings.flashes && unit.is_some_and(|id| fx.flashes.contains_key(&id));
        if flashing {
            let white = if let Some(handle) = cache.white.get(&original.id()) {
                handle.clone()
            } else {
                let Some(mut source) = materials.get(&original).cloned() else {
                    continue;
                };
                source.base_color = Color::WHITE;
                source.emissive = LinearRgba::rgb(3., 3., 3.);
                let handle = materials.add(source);
                cache.white.insert(original.id(), handle.clone());
                handle
            };
            cache.originals.insert(entity, original);
            if material.0 != white {
                material.0 = white;
            }
        } else if let Some(original) = cache.originals.remove(&entity) {
            material.0 = original;
        }
    }
    cache.originals.retain(|entity, _| live.contains(entity));
}
