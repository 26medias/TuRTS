//! Tick log (`--tick-log PATH`).
//!
//! After every tick, one line for each entity that `Query<(&UnitId, &Position)>` matches:
//! `tick,wall_ns,unit_id,x,y`. `wall_ns` is the time of day in nanoseconds since the UNIX epoch.
//! The lines of one tick are sorted by unit id. The file is flushed after every tick, so a
//! reader sees the ticks while the server runs. An entity that is spawned on a slot claim
//! shows in the log, which is how the process-level T3 sees it (design section 7).
//!
//! The log is off when `--synthetic-units` is above 0: one line per unit per tick is too much
//! for a load run.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::app::FixedLast;
use bevy::prelude::*;
use turts_sim::{Position, Tick, UnitId};

use crate::config::Cli;
use crate::stats::TickEnd;

/// The open log file.
#[derive(Resource)]
struct TickLog {
    out: BufWriter<File>,
}

/// Add the tick log to the app when `--tick-log` is set and `--synthetic-units` is 0.
///
/// # Panics
///
/// When the file cannot be created, or later when it cannot be written. The log is the
/// instrument of the process-level tests, and a log that stops without a word would let a
/// test read an empty file.
pub fn install(app: &mut App, cli: &Cli) {
    let Some(path) = &cli.tick_log else {
        return;
    };
    if cli.synthetic_units > 0 {
        eprintln!("turts_server: --tick-log is off when --synthetic-units is above 0");
        return;
    }
    let file = File::create(path).unwrap_or_else(|error| {
        panic!(
            "turts_server: cannot create --tick-log `{}`: {error}",
            path.display()
        )
    });
    app.insert_resource(TickLog {
        out: BufWriter::new(file),
    });
    // After the tick timer, so that the log adds nothing to a tick time in `--stats`.
    app.add_systems(FixedLast, write_tick.after(TickEnd));
}

/// FixedLast: log the position of every unit at the end of the tick.
fn write_tick(
    tick: Res<Tick>,
    mut log: ResMut<TickLog>,
    units: Query<(&UnitId, &Position)>,
    mut rows: Local<Vec<(u32, Vec2)>>,
) {
    let wall_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    rows.clear();
    rows.extend(units.iter().map(|(id, position)| (id.0, position.0)));
    rows.sort_unstable_by_key(|(id, _)| *id);
    write_rows(&mut log.out, tick.0, wall_ns, &rows)
        .unwrap_or_else(|error| panic!("turts_server: cannot write --tick-log: {error}"));
}

/// Write the lines of one tick and flush.
fn write_rows(
    out: &mut impl Write,
    tick: u32,
    wall_ns: u128,
    rows: &[(u32, Vec2)],
) -> io::Result<()> {
    for (id, position) in rows {
        writeln!(out, "{tick},{wall_ns},{id},{},{}", position.x, position.y)?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use turts_sim::{Position, UnitId, Vec2, headless_app};

    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("turts_ticklog_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cli(tick_log: &Path, synthetic_units: u32) -> Cli {
        Cli {
            config: PathBuf::from("server.ron"),
            tick_log: Some(tick_log.to_path_buf()),
            stats: None,
            economy_log: None,
            synthetic_units,
            run_ticks: None,
        }
    }

    fn now_ns() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    /// The lines of the log as `(tick, wall_ns, unit_id, x, y)` text fields.
    fn lines(path: &Path) -> Vec<Vec<String>> {
        fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|line| line.split(',').map(str::to_string).collect())
            .collect()
    }

    #[test]
    fn every_unit_gets_one_line_per_tick_and_the_file_is_flushed_each_tick() {
        let dir = test_dir("lines");
        let path = dir.join("ticks.log");
        let mut app = headless_app(20);
        install(&mut app, &cli(&path, 0));
        // Spawned out of order on purpose: the lines of one tick are sorted by unit id.
        app.world_mut()
            .spawn((UnitId(5), Position(Vec2::new(1.5, 2.0))));
        app.world_mut()
            .spawn((UnitId(2), Position(Vec2::new(3.0, 4.25))));
        // These two are not matched by `Query<(&UnitId, &Position)>`: no log line.
        app.world_mut().spawn(Position(Vec2::new(9.0, 9.0)));
        app.world_mut().spawn(UnitId(7));

        let before = now_ns();
        for _ in 0..3 {
            app.update();
        }
        let after = now_ns();
        // The app is still alive: the data is in the file, not in a buffer.
        let rows = lines(&path);
        assert_eq!(rows.len(), 6, "{rows:?}");
        let mut last_wall = before;
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), 5, "{row:?}");
            assert_eq!(row[0], (index / 2 + 1).to_string(), "tick of {row:?}");
            let wall: u128 = row[1].parse().unwrap();
            assert!(
                before <= wall && wall <= after,
                "wall_ns {wall} in {before}..{after}"
            );
            assert!(wall >= last_wall, "wall_ns never goes back");
            last_wall = wall;
            let (unit, x, y) = if index % 2 == 0 {
                ("2", "3", "4.25")
            } else {
                ("5", "1.5", "2")
            };
            assert_eq!(row[2..], [unit, x, y], "{row:?}");
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_unit_that_stands_still_is_logged_every_tick() {
        let dir = test_dir("still");
        let path = dir.join("ticks.log");
        let mut app = headless_app(20);
        install(&mut app, &cli(&path, 0));
        app.world_mut()
            .spawn((UnitId(1), Position(Vec2::new(8.0, 8.0))));
        for _ in 0..4 {
            app.update();
        }
        let ticks: Vec<String> = lines(&path).into_iter().map(|row| row[0].clone()).collect();
        assert_eq!(ticks, ["1", "2", "3", "4"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn synthetic_units_switch_the_log_off() {
        let dir = test_dir("synthetic");
        let path = dir.join("ticks.log");
        let mut app = headless_app(20);
        install(&mut app, &cli(&path, 1000));
        app.world_mut()
            .spawn((UnitId(1), Position(Vec2::new(8.0, 8.0))));
        app.update();
        assert!(!path.exists(), "no tick log with --synthetic-units");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn without_a_path_nothing_is_logged() {
        let mut app = headless_app(20);
        let mut options = cli(Path::new("unused"), 0);
        options.tick_log = None;
        install(&mut app, &options);
        app.update();
        assert!(!Path::new("unused").exists());
    }
}
