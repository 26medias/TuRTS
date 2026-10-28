//! Inserter arm state. The phase is added in Task 8.

use std::collections::BTreeMap;

use bevy_ecs::prelude::Resource;

/// Current arm phase.
#[allow(dead_code)] // used from Task 8
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArmStage {
    AtSource(Option<ArmWait>),
    ToDestination,
    AtDestination(ArmWait),
    Returning,
}

impl Default for ArmStage {
    fn default() -> Self {
        Self::AtSource(None)
    }
}

/// Why an arm is waiting at one end of its cycle.
#[allow(dead_code)] // used from Task 8
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ArmWait {
    #[default]
    NoSource,
    NoDestination,
    SourceEmpty,
    DestinationFull,
}

/// Resolved endpoints and progress for one inserter.
#[allow(dead_code)] // used from Task 8
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct InserterState {
    pub(crate) source: Option<u32>,
    pub(crate) destination: Option<u32>,
    pub(crate) stage: ArmStage,
    pub(crate) elapsed: u32,
}

/// Inserter state indexed by unit id.
#[allow(dead_code)] // used from Task 8
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Inserters(pub(crate) BTreeMap<u32, InserterState>);

/// Install the Inserters phase systems. Task 8 adds the phase behavior.
pub(crate) fn install(_app: &mut bevy_app::App) {}
