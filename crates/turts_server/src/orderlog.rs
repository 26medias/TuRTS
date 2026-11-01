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

fn record_changes(world: &mut World) {
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
                UnitOrderActivityView::Blocked => Some((
                    "Blocked",
                    row.last_failure
                        .as_ref()
                        .map(|failure| failure.reason.as_str()),
                )),
                UnitOrderActivityView::Idle => match row.last_failure.as_ref() {
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

    let mut current: BTreeMap<_, _> = rows
        .into_iter()
        .map(|(unit, state, reason)| (unit, (state, reason)))
        .collect();
    let mut log = world.resource_mut::<OrderLog>();
    for (unit, (state, reason)) in &current {
        let next = (state.clone(), reason.clone());
        if log.previous.get(unit) != Some(&next) {
            write_transition(
                &mut log.out,
                tick,
                *unit,
                state,
                (reason != "-").then_some(reason),
            )
            .unwrap_or_else(|error| panic!("turts_server: cannot write --order-log: {error}"));
        }
    }
    let finished: Vec<_> = log
        .previous
        .keys()
        .filter(|unit| !current.contains_key(unit))
        .copied()
        .collect();
    for unit in finished {
        if log
            .previous
            .get(&unit)
            .is_some_and(|(state, _)| state != "Done" && state != "Failed")
        {
            write_transition(&mut log.out, tick, unit, "Done", None)
                .unwrap_or_else(|error| panic!("turts_server: cannot write --order-log: {error}"));
        }
        log.previous.remove(&unit);
    }
    log.previous.append(&mut current);
    log.out
        .flush()
        .unwrap_or_else(|error| panic!("turts_server: cannot flush --order-log: {error}"));
}
