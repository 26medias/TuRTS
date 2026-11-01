//! Deterministic unit order transition log (`--order-log PATH`).

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

use bevy::app::FixedLast;
use bevy::prelude::{App, Resource, World};
use turts_sim::Tick;
use turts_sim::economy::views::UnitOrderActivityView;

#[derive(Resource)]
struct OrderLog {
    out: BufWriter<File>,
    previous: BTreeMap<u32, (String, String)>,
}

/// Install the transition writer when `--order-log` is enabled.
pub fn install(app: &mut App, path: Option<&Path>) -> Result<(), String> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create --order-log directory: {error}"))?;
    }
    let file = File::create(path)
        .map_err(|error| format!("cannot create --order-log `{}`: {error}", path.display()))?;
    app.insert_resource(OrderLog {
        out: BufWriter::new(file),
        previous: BTreeMap::new(),
    });
    app.add_systems(FixedLast, record_changes);
    Ok(())
}

/// Write one state transition as `tick,unit_id,state,reason`.
pub fn write_transition(
    out: &mut impl Write,
    tick: u32,
    unit_id: u32,
    state: &str,
    reason: Option<&str>,
) -> io::Result<()> {
    writeln!(out, "{tick},{unit_id},{state},{}", reason.unwrap_or("-"))
}

pub fn record_changes(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let previous = world.resource::<OrderLog>().previous.clone();
    let rows: Vec<_> = turts_sim::completed_world(world)
        .unit_orders
        .iter()
        .filter_map(|row| {
            let status = match row.activity {
                UnitOrderActivityView::Pending => Some(("Pending", None)),
                UnitOrderActivityView::Moving
                | UnitOrderActivityView::Attacking
                | UnitOrderActivityView::Holding
                | UnitOrderActivityView::Gathering => Some(("Moving", None)),
                UnitOrderActivityView::Blocked => Some(("Blocked", Some("blocked"))),
                UnitOrderActivityView::Idle => match row
                    .last_failure
                    .as_ref()
                    .filter(|failure| failure.tick == tick)
                {
                    Some(failure) => Some(("Failed", Some(failure.reason.as_str()))),
                    None => previous
                        .get(&row.unit_id)
                        .filter(|(state, _)| state != "Done" && state != "Failed")
                        .map(|_| ("Done", None)),
                },
            };
            status.map(|(state, reason)| {
                (
                    row.unit_id,
                    state.to_string(),
                    reason.unwrap_or("-").to_string(),
                )
            })
        })
        .collect();

    let current: BTreeMap<_, _> = rows
        .into_iter()
        .map(|(unit, state, reason)| (unit, (state, reason)))
        .collect();
    let mut transitions = BTreeMap::new();
    for (unit, next) in &current {
        if previous.get(unit) != Some(next) {
            transitions.insert(*unit, next.clone());
        }
    }
    for (unit, (state, _)) in &previous {
        if !current.contains_key(unit) && state != "Done" && state != "Failed" {
            transitions.insert(*unit, ("Done".to_string(), "-".to_string()));
        }
    }

    let mut log = world.resource_mut::<OrderLog>();
    for (unit, (state, reason)) in transitions {
        write_transition(
            &mut log.out,
            tick,
            unit,
            &state,
            (reason != "-").then_some(reason.as_str()),
        )
        .unwrap_or_else(|error| panic!("turts_server: cannot write --order-log: {error}"));
    }
    log.previous.retain(|unit, _| current.contains_key(unit));
    log.previous.extend(current);
    log.out
        .flush()
        .unwrap_or_else(|error| panic!("turts_server: cannot flush --order-log: {error}"));
}
