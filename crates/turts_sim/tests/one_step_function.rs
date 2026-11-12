//! The one-step-function guard (pre-MP1 slice, task A2a; design section 6, section 9 items 5, 13
//! and 22; plan refinement R12).
//!
//! Two tests, two kinds of proof:
//!
//! - `each_moving_unit_passes_each_shared_phase_once` runs a scenario on the authored `muster`
//!   package: a gather trip, a queued start, an attack approach, a follow, a resume, a single
//!   envelope move, a group move and a synthetic unit. The cargo feature `test-probes` counts
//!   the calls of each shared phase of the free-movement step (`probe::phase_calls`). Every
//!   moving unit enters the step once in each tick, and no phase runs twice. A call counter
//!   cannot see a second branch inside one function, so the counter counts phases, not calls
//!   of the function (design 9.5).
//! - `step_loop_and_step_call_occur_once_outside_plain` reads the production source of
//!   `crates/turts_sim/src/movement/` and counts the steering loop `for _ in 0..8` and the call
//!   `step(&MovementInput` (regex `\bstep\(\s*&MovementInput`). Comments and string literals
//!   are removed first (design 9.22). The files `plain.rs` (the plain-progress model that A2b2
//!   deletes) and `reference.rs` (the copy of the old kernel) and the folder `tests/` are not
//!   counted (R12). One occurrence of each means that one step function holds the loop.

use std::path::{Path, PathBuf};

/// The text of `source` with comments, string literals and char literals replaced by spaces
/// (newlines stay). Only code remains.
fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
        } else if c == 'r' && matches!(next, Some('"') | Some('#')) && raw_string_start(&chars, i) {
            let mut hashes = 0;
            let mut j = i + 1;
            while chars[j] == '#' {
                hashes += 1;
                j += 1;
            }
            // chars[j] is the opening quote.
            j += 1;
            let closing: Vec<char> = std::iter::once('"')
                .chain(std::iter::repeat_n('#', hashes))
                .collect();
            while j < chars.len() && chars[j..].get(..closing.len()) != Some(&closing[..]) {
                j += 1;
            }
            let end = (j + closing.len()).min(chars.len());
            for &c in &chars[i..end] {
                out.push(blank(c));
            }
            i = end;
        } else if c == '"' {
            out.push(' ');
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    out.push(' ');
                    i += 1;
                }
                if i < chars.len() {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
            out.push(' ');
            i += 1;
        } else if c == '\'' {
            // A char literal is `'x'` or `'\..'`; anything else is a lifetime.
            let is_char = next == Some('\\') || chars.get(i + 2) == Some(&'\'');
            if is_char {
                out.push(' ');
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    if chars[i] == '\\' {
                        out.push(' ');
                        i += 1;
                    }
                    if i < chars.len() {
                        out.push(' ');
                        i += 1;
                    }
                }
                out.push(' ');
                i += 1;
            } else {
                out.push(c);
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `chars[i]` is `r` and starts a raw string: `r"` or `r#...#"`, and the `r` is not the end of
/// an identifier.
fn raw_string_start(chars: &[char], i: usize) -> bool {
    if i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
        return false;
    }
    let mut j = i + 1;
    while chars.get(j) == Some(&'#') {
        j += 1;
    }
    chars.get(j) == Some(&'"')
}

/// The number of times `for _ in 0..8` occurs in code. Runs of white space count as one space.
fn count_steering_loops(code: &str) -> usize {
    let flat = code.split_whitespace().collect::<Vec<_>>().join(" ");
    flat.matches("for _ in 0..8").count()
}

/// The number of matches of the regex `\bstep\(\s*&MovementInput` in code.
fn count_step_calls(code: &str) -> usize {
    let chars: Vec<char> = code.chars().collect();
    let needle: Vec<char> = "step(".chars().collect();
    let target: Vec<char> = "&MovementInput".chars().collect();
    let mut count = 0;
    let mut i = 0;
    while i + needle.len() <= chars.len() {
        if chars[i..i + needle.len()] == needle[..] {
            let boundary = i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
            let mut j = i + needle.len();
            while chars.get(j).is_some_and(|c| c.is_whitespace()) {
                j += 1;
            }
            if boundary && chars.get(j..j + target.len()) == Some(&target[..]) {
                count += 1;
            }
        }
        i += 1;
    }
    count
}

/// The production files of `movement/` that count: every `.rs` file except `plain.rs`,
/// `reference.rs` and the folder `tests/`.
fn counted_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name != "tests" {
                counted_files(&path, out);
            }
        } else if name.ends_with(".rs") && name != "plain.rs" && name != "reference.rs" {
            out.push(path);
        }
    }
}

fn movement_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/movement")
}

/// Count `counter` over the counted files. The result lists each file with its count.
fn count_over_movement(counter: fn(&str) -> usize) -> (usize, Vec<(String, usize)>) {
    let mut files = Vec::new();
    counted_files(&movement_dir(), &mut files);
    assert!(
        files.iter().any(|f| f.ends_with("frame.rs")),
        "the scan must see frame.rs, got {files:?}"
    );
    let mut total = 0;
    let mut rows = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read a movement source file");
        let n = counter(&code_only(&text));
        total += n;
        if n > 0 {
            rows.push((file.display().to_string(), n));
        }
    }
    (total, rows)
}

#[test]
fn step_loop_and_step_call_occur_once_outside_plain() {
    let (loops, loop_rows) = count_over_movement(count_steering_loops);
    let (calls, call_rows) = count_over_movement(count_step_calls);
    assert_eq!(
        calls, 1,
        r"\bstep\(\s*&MovementInput: {calls} occurrences, expected 1 ({call_rows:?})"
    );
    assert_eq!(
        loops, 1,
        "for _ in 0..8: {loops} occurrences, expected 1 ({loop_rows:?})"
    );
}

/// The scanner is itself a test instrument: it must count code and nothing else.
#[test]
fn the_source_scanner_counts_code_only() {
    let text = r####"
        // for _ in 0..8 { step(&MovementInput { } }
        /* for _ in 0..8 { /* nested */ step(
            &MovementInput) } */
        let a = "for _ in 0..8 step(&MovementInput";
        let b = r##"step(&MovementInput "# for _ in 0..8"##;
        let c = '"';
        fn over<'a>(x: &'a str) {
            for   _   in
                0..8 {
                step(
                    &MovementInput { speed: 1.0 },
                );
            }
        }
        fn not_a_call(input: &MovementInput) { missteps(&MovementInput) }
    "####;
    let code = code_only(text);
    assert_eq!(count_steering_loops(&code), 1, "{code}");
    assert_eq!(count_step_calls(&code), 1, "{code}");
}

#[cfg(feature = "test-probes")]
mod scenario {
    use std::collections::BTreeMap;

    use turts_mod_api::{ContentId, OrderDef, PackageId, SourceDef};
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
    use turts_sim::movement::{PhysicalEscape, probe};
    use turts_sim::{
        Command, CommandKind, CommandQueue, NodeId, OrderCommand, OrderTargetSim, Position,
        QueueMode, SlotId, UnitId, Vec2, headless_app, install_package, stage_package,
    };

    struct Muster {
        app: bevy_app::App,
        catalog: turts_mod_api::ContentCatalog,
    }

    fn muster() -> Muster {
        let package_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
        let archive = PackageArchive::parse(&pack_dir(&package_dir).expect("Muster packs"))
            .expect("Muster archive parses");
        let catalog = archive.content().expect("Muster catalog resolves");
        let (map_file, map) = archive
            .maps()
            .expect("Muster maps parse")
            .into_iter()
            .find(|(_, map)| map.id == "field")
            .expect("Muster field map exists");
        let mut app = headless_app(20);
        let staged = stage_package(
            app.world(),
            catalog.clone(),
            SourceDef {
                file: map_file,
                value: map,
            },
            20,
        )
        .expect("the authored Muster map stages");
        install_package(app.world_mut(), staged);
        Muster { app, catalog }
    }

    /// An order of the Muster package by its local id.
    fn order(
        catalog: &turts_mod_api::ContentCatalog,
        local: &str,
        seq: u32,
        units: Vec<u32>,
        target: OrderTargetSim,
        queue: QueueMode,
        group: u32,
    ) -> Command {
        let (id, definition): (ContentId, &OrderDef) = catalog
            .orders
            .iter()
            .find(|(id, _)| id.package.0 == "muster" && id.local == local)
            .map(|(id, definition)| (id.clone(), definition))
            .unwrap_or_else(|| panic!("Muster order {local} exists"));
        Command {
            slot: SlotId(0),
            seq,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: id,
                behaviour: definition.behaviour,
                target_kind: definition.target,
                policy: None,
                units,
                target,
                queue,
                group,
                allowed_units: definition.units.clone(),
                queue_depth: catalog.order_queue_depths[&PackageId("muster".into())],
            })),
        }
    }

    /// One unit of each kind of mover that the design names (section 6, "One step function").
    /// The names are the labels of the coverage check.
    const WATCHED: [(u32, &str); 12] = [
        (1, "group move"),
        (9, "group move"),
        (17, "single envelope move"),
        (18, "queued start"),
        (19, "attack approach"),
        (21, "follow"),
        (22, "follow target"),
        (23, "resume (rush)"),
        (24, "resume (rush)"),
        (41, "gather trip"),
        (42, "gather trip"),
        (56, "synthetic unit"),
    ];

    /// The state of one unit at the start of a tick.
    struct Before {
        position: Vec2,
        eligible: bool,
    }

    /// The commands of the scenario, with the tick at which each one is queued: one unit of each
    /// kind of mover that the design names (section 6, "One step function").
    fn script(catalog: &turts_mod_api::ContentCatalog) -> Vec<(u32, Command)> {
        let catalog = catalog.clone();
        let point = |x: f32, y: f32| OrderTargetSim::Point(Vec2::new(x, y));
        let mut seq = 0;
        let mut script: Vec<(u32, Command)> = Vec::new();
        let mut add = |at: u32, mut command: Command| {
            seq += 1;
            command.seq = seq;
            script.push((at, command));
        };
        let replace = QueueMode::Replace;
        // A group move (a pool of 16 troopers).
        add(
            1,
            order(
                &catalog,
                "order_move",
                0,
                (1..=16).collect(),
                point(40.0, 30.0),
                replace,
                5,
            ),
        );
        // A single envelope move (group 0).
        add(
            1,
            order(
                &catalog,
                "order_move",
                0,
                vec![17],
                point(20.0, 40.0),
                replace,
                0,
            ),
        );
        // A queued start: the second move starts after the first one lands.
        add(
            1,
            order(
                &catalog,
                "order_move",
                0,
                vec![18],
                point(30.0, 30.0),
                replace,
                0,
            ),
        );
        add(
            2,
            order(
                &catalog,
                "order_move",
                0,
                vec![18],
                point(45.0, 40.0),
                QueueMode::Append,
                0,
            ),
        );
        // An attack approach: the target is an enemy dummy.
        add(
            1,
            order(
                &catalog,
                "order_attack",
                0,
                vec![19, 20],
                OrderTargetSim::Unit(45),
                replace,
                0,
            ),
        );
        // A follow: trooper 21 follows trooper 22, which moves away.
        add(
            1,
            order(
                &catalog,
                "order_move",
                0,
                vec![22],
                point(60.0, 10.0),
                replace,
                0,
            ),
        );
        add(
            1,
            order(
                &catalog,
                "order_move",
                0,
                vec![21],
                OrderTargetSim::Unit(22),
                replace,
                0,
            ),
        );
        // A rush (attack-point): the units stop to fire in range and resume the move.
        add(
            1,
            order(
                &catalog,
                "rush",
                0,
                (23..=30).collect(),
                point(55.0, 34.0),
                replace,
                0,
            ),
        );
        // A gather trip (no pool): two workers, the node and the depot of the map.
        add(
            1,
            order(
                &catalog,
                "order_gather",
                0,
                vec![41, 42],
                OrderTargetSim::Node(NodeId(1)),
                replace,
                0,
            ),
        );
        // A synthetic unit: a new move target every 40 ticks (`--synthetic-units`).
        for (round, tick) in (1..=14u32).map(|round| (round, round * 40 - 39)) {
            let x = 35.0 + ((round * 37) % 25) as f32;
            let y = 2.0 + ((round * 11) % 15) as f32;
            add(
                tick,
                Command {
                    slot: SlotId(0),
                    seq: 0,
                    kind: CommandKind::MoveTo {
                        unit_ids: vec![56],
                        target: Vec2::new(x, y),
                    },
                },
            );
        }
        script
    }

    /// Queue the commands of `script` that are due at `tick`.
    fn queue_due(app: &mut bevy_app::App, script: &[(u32, Command)], tick: u32) {
        for (at, command) in script {
            if *at == tick {
                app.world_mut()
                    .resource_mut::<CommandQueue>()
                    .0
                    .push_back(command.clone());
            }
        }
    }

    #[test]
    fn each_moving_unit_passes_each_shared_phase_once() {
        let mut fixture = muster();
        let script = script(&fixture.catalog);
        let app = &mut fixture.app;

        // The ticks where each label had a unit in the free-movement step.
        let mut seen: BTreeMap<&str, u32> = BTreeMap::new();
        let mut totals = [0u32; 5];
        for tick in 1..=560u32 {
            queue_due(app, &script, tick);
            let before: BTreeMap<u32, Before> = WATCHED
                .iter()
                .map(|&(id, _)| {
                    let world = app.world();
                    let position = position_of(world, id);
                    let escape = unit_has_escape(app, id);
                    let eligible = probe::unit_state(world, id) != "plain"
                        && probe::activity(world, id) != "Blocked"
                        && !escape;
                    (id, Before { position, eligible })
                })
                .collect();
            app.update();
            let world = app.world();
            for &(id, label) in &WATCHED {
                let calls = probe::phase_calls(world, id);
                let moved = position_of(world, id).to_array().map(f32::to_bits)
                    != before[&id].position.to_array().map(f32::to_bits);
                if moved && before[&id].eligible {
                    assert_eq!(
                        calls.arrival, 1,
                        "unit {id} tick {tick} phase arrival: {} calls ({label})",
                        calls.arrival
                    );
                    *seen.entry(label).or_default() += 1;
                }
                for (phase, n) in [
                    ("arrival", calls.arrival),
                    ("steering", calls.steering),
                    ("projection", calls.projection),
                    ("stall", calls.stall),
                    ("fallback", calls.fallback),
                ] {
                    assert!(
                        n <= 1,
                        "unit {id} tick {tick} phase {phase}: {n} calls ({label})"
                    );
                }
                assert_eq!(
                    calls.steering, calls.projection,
                    "unit {id} tick {tick} phase projection: {} calls against {} of steering",
                    calls.projection, calls.steering
                );
                assert!(
                    calls.steering <= calls.arrival,
                    "unit {id} tick {tick} phase steering: {} calls, arrival {}",
                    calls.steering,
                    calls.arrival
                );
                totals[0] += u32::from(calls.arrival);
                totals[1] += u32::from(calls.steering);
                totals[2] += u32::from(calls.projection);
                totals[3] += u32::from(calls.stall);
                totals[4] += u32::from(calls.fallback);
            }
        }
        // The scenario must reach each kind of mover, else the counts prove nothing.
        for label in [
            "group move",
            "single envelope move",
            "queued start",
            "attack approach",
            "follow",
            "follow target",
            "resume (rush)",
            "gather trip",
            "synthetic unit",
        ] {
            assert!(
                seen.get(label).copied().unwrap_or(0) > 0,
                "no unit of the kind {label:?} stepped: {seen:?}"
            );
        }
        for (phase, total) in ["arrival", "steering", "projection", "stall", "fallback"]
            .iter()
            .zip(totals)
        {
            assert!(
                total > 0,
                "phase {phase} never ran in the scenario: {totals:?}"
            );
        }
    }

    /// No unit of the scenario of `each_moving_unit_passes_each_shared_phase_once` holds a
    /// plain progress at the end of any tick (pre-MP1 slice, task A2b1; design 9.11): the obstacle
    /// fallback gives a pool of one, so no code creates a plain progress. The scenario must still
    /// reach the fallback: at least one unit holds a fallback pool at the end of some tick.
    #[test]
    fn no_plain_progress_is_created() {
        let mut fixture = muster();
        let script = script(&fixture.catalog);
        let app = &mut fixture.app;
        for tick in 1..=560u32 {
            queue_due(app, &script, tick);
            app.update();
            let world = app.world();
            let units: Vec<u32> = world
                .iter_entities()
                .filter_map(|entity| entity.get::<UnitId>().map(|id| id.0))
                .collect();
            for unit in units {
                assert_ne!(
                    probe::unit_state(world, unit),
                    "plain",
                    "unit {unit} tick {tick}: a plain progress was created"
                );
            }
        }
        assert!(
            !probe::fallback_trips(app.world()).is_empty(),
            "no unit of the scenario reached the obstacle fallback"
        );
    }

    fn position_of(world: &bevy_ecs::world::World, unit: u32) -> Vec2 {
        world
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit))
            .and_then(|entity| entity.get::<Position>().map(|position| position.0))
            .unwrap_or_else(|| panic!("unit {unit} has a position"))
    }

    fn unit_has_escape(app: &bevy_app::App, unit: u32) -> bool {
        app.world().iter_entities().any(|entity| {
            entity.get::<UnitId>().is_some_and(|id| id.0 == unit)
                && entity.contains::<PhysicalEscape>()
        })
    }
}
