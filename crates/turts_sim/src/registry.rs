//! UnitRegistry implementation.

use bevy_ecs::prelude::Resource;
use turts_mod_api::{ContentId, Footprint, ModError, PackageId, UnitDef, UnitRegistry, UnitTypeId};

/// Upper bound of a unit speed, in world units per second.
const MAX_SPEED: f32 = 100.0;
/// Upper bound of a circle footprint radius, in world units.
const MAX_RADIUS: f32 = 50.0;

/// One registered unit type.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitTypeEntry {
    /// Full content id: `<package>:<local>`.
    pub content_id: String,
    pub speed: f32,
    pub radius: f32,
}

/// All registered unit types. A `UnitTypeId` is the index of an entry.
#[derive(Resource, Default, Debug)]
pub struct UnitTypes {
    entries: Vec<UnitTypeEntry>,
}

impl UnitTypes {
    pub fn get(&self, id: UnitTypeId) -> Option<&UnitTypeEntry> {
        self.entries.get(id.0 as usize)
    }

    pub fn find(&self, content_id: &str) -> Option<UnitTypeId> {
        self.entries
            .iter()
            .position(|entry| entry.content_id == content_id)
            .map(|index| UnitTypeId(index as u32))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// `value` is finite, above 0 and at most `max`.
fn in_range(value: f32, max: f32) -> bool {
    value.is_finite() && value > 0.0 && value <= max
}

impl UnitRegistry for UnitTypes {
    fn register_unit_type(
        &mut self,
        origin: &PackageId,
        def: UnitDef,
    ) -> Result<UnitTypeId, ModError> {
        let error = |field: &str, message: String| ModError::new(&origin.0, field, message);
        if def.id.contains(':') {
            return Err(error(
                "id",
                format!("local id `{}` must not contain `:`", def.id),
            ));
        }
        if !in_range(def.speed, MAX_SPEED) {
            return Err(error(
                "speed",
                format!(
                    "speed must be finite, above 0 and at most {MAX_SPEED}, got {}",
                    def.speed
                ),
            ));
        }
        let Footprint::Circle { radius } = def.footprint;
        if !in_range(radius, MAX_RADIUS) {
            return Err(error(
                "footprint.radius",
                format!("radius must be finite, above 0 and at most {MAX_RADIUS}, got {radius}"),
            ));
        }
        let content_id = ContentId::new(origin, &def.id).to_string();
        if self.find(&content_id).is_some() {
            return Err(error(
                "id",
                format!("unit type `{content_id}` is already registered"),
            ));
        }
        let id = UnitTypeId(self.entries.len() as u32);
        self.entries.push(UnitTypeEntry {
            content_id,
            speed: def.speed,
            radius,
        });
        Ok(id)
    }
}
