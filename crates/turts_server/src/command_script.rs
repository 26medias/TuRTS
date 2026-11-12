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
    for (index, entry) in entries.into_iter().enumerate() {
        if entry.tick == 0 {
            return Err(format!(
                "command script `{}` entry {index} has tick 0; fixed-tick entries start at tick 1",
                path.display()
            ));
        }
        by_tick.entry(entry.tick).or_default().push(entry);
    }
    app.insert_resource(Script(by_tick));
    app.add_systems(
        FixedPreUpdate,
        apply_script
            .after(SimSet::Construction)
            .before(SimSet::Commands),
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

#[cfg(test)]
mod tests {
    use super::*;
    use turts_net_protocol::{OrderTarget, Pos, QueueMode, UnitId};

    #[derive(serde::Serialize)]
    struct TestEntry<'a> {
        tick: u32,
        slot: u8,
        envelope: &'a OrderEnvelope,
    }

    #[test]
    fn the_script_system_is_ordered_against_every_set_before_the_commands() {
        let path = std::env::temp_dir().join(format!(
            "turts_command_script_order_{}_{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::write(&path, b"[]").unwrap();
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin)
            .add_plugins(turts_sim::SimPlugin { tick_hz: 20 })
            .insert_resource(turts_sim::MatchConfig::immediate(20));
        install(&mut app, Some(&path)).unwrap();
        crate::session::assert_fixed_pre_update_is_totally_ordered(app);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn loader_rejects_tick_zero_before_installing_any_script_entries() {
        let path = std::env::temp_dir().join(format!(
            "turts_command_script_zero_{}_{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_file(&path);
        let envelope = OrderEnvelope {
            order: "acme:order_move".into(),
            units: vec![UnitId(1)],
            target: OrderTarget::Point(Pos { x: 4.0, y: 5.0 }),
            queue: QueueMode::Replace,
            group: 0,
        };
        let entries = [
            TestEntry {
                tick: 1,
                slot: 0,
                envelope: &envelope,
            },
            TestEntry {
                tick: 0,
                slot: 0,
                envelope: &envelope,
            },
        ];
        fs::write(&path, serde_json::to_vec(&entries).unwrap()).unwrap();

        let mut app = App::new();
        let error = install(&mut app, Some(&path)).unwrap_err();
        assert!(error.contains(&path.display().to_string()), "{error}");
        assert!(error.contains("entry 1"), "{error}");
        assert!(error.contains("tick 0"), "{error}");
        assert!(
            !app.world().contains_resource::<Script>(),
            "valid earlier entries must not install when a later entry has tick zero"
        );
        fs::remove_file(path).unwrap();
    }
}
