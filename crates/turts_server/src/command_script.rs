//! Fixed-tick test commands applied without a network client.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::{App, FixedPreUpdate, Resource, World};
use serde::Deserialize;
use turts_net_protocol::{OrderEnvelope, PlayerCommand};
use turts_sim::{CommandQueue, SimSet, Tick};

#[derive(Debug, Clone, Deserialize)]
struct Entry {
    tick: u32,
    slot: u8,
    envelope: OrderEnvelope,
}

#[derive(Resource, Default)]
struct Script(BTreeMap<u32, Vec<Entry>>);

/// Load a JSON array of fixed-tick command envelopes.
pub fn install(app: &mut App, path: Option<&Path>) -> Result<(), String> {
    let Some(path) = path else {
        return Ok(());
    };
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read command script `{}`: {error}", path.display()))?;
    let entries: Vec<Entry> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("command script `{}`: {error}", path.display()))?;
    let mut by_tick = BTreeMap::<u32, Vec<Entry>>::new();
    for entry in entries {
        by_tick.entry(entry.tick).or_default().push(entry);
    }
    app.insert_resource(Script(by_tick));
    app.add_systems(
        FixedPreUpdate,
        apply_script.after(SimSet::Advance).before(SimSet::Commands),
    );
    Ok(())
}

fn apply_script(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let Some(entries) = world.resource_mut::<Script>().0.remove(&tick) else {
        return;
    };
    let catalog = world.resource::<crate::orders::ServerCatalog>().0.clone();
    let mut commands = Vec::new();
    for entry in entries {
        let command = PlayerCommand {
            seq: 0,
            order: entry.envelope,
        };
        match crate::orders::to_sim(turts_sim::SlotId(entry.slot), &command, &catalog) {
            Ok(mut resolved) => commands.append(&mut resolved),
            Err(result) => eprintln!(
                "turts_server: --command-script refused envelope at tick {}: {:?}",
                tick, result
            ),
        }
    }
    world.resource_mut::<CommandQueue>().0.extend(commands);
}
