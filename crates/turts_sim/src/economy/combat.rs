//! Weapon and health state. The combat phase is added in Task 11.

use std::collections::BTreeMap;

use bevy_ecs::prelude::Resource;

/// Current weapon condition.
#[allow(dead_code)] // used from Task 11
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum WeaponStatus {
    #[default]
    Ready,
    Cooling,
    NoTarget,
    OutOfAmmo,
}

/// Cooldown and most recent targeting information for a weapon.
#[allow(dead_code)] // used from Task 11
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WeaponState {
    pub(crate) cooldown: u32,
    pub(crate) status: WeaponStatus,
    pub(crate) last_target: Option<u32>,
}

/// Weapon states indexed by unit id.
#[allow(dead_code)] // used from Task 11
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Weapons(pub(crate) BTreeMap<u32, WeaponState>);

/// Current hit points indexed by unit id.
#[allow(dead_code)] // used from Task 11
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Healths(pub(crate) BTreeMap<u32, u64>);

/// Install the Combat phase systems. Task 11 adds the phase behavior.
pub(crate) fn install(_app: &mut bevy_app::App) {}
