//! Scratch probe (MP1 obstacle question). Not for commit.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use turts_mod_api::{
    ContentId, Footprint, MapDef, MapSize, MovementDef, NodePlacement, PackageId, Placement,
    SourceDef, UnitDef, UnitRegistry, WorldPos,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    Command, CommandKind, CommandQueue, OrderCommand, OrderTargetSim, Position, QueueMode,
    RejectReason, SlotId, Tick, UnitId, UnitOrders, UnitTypes, Vec2, headless_app,
    install_package, load_map, stage_package,
};

const HZ: u16 = 20;
const SPEED: f32 = 2.4;
const ORIGIN: &str = "probe";
const MAP: MapSize = MapSize { x: 112.0, y: 72.0 };

fn def(id: &str, radius: f32, speed: Option<f32>) -> UnitDef {
    UnitDef {
        id: id.into(),
        name: id.into(),
        movement: speed.map(|speed| MovementDef { speed }),
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: None,
        footprint: Footprint::Circle { radius },
        output_inventory: None,
        extractor: None,
        conveyor: None,
        inserter: None,
        machine: None,
        research: None,
        health: None,
        weapon: None,
        removal: None,
    }
}

struct Fixture {
    app: bevy_app::App,
    mobiles: Vec<(u32, Vec2, f32)>,
    statics: Vec<(Vec2, f32)>,
    seq: u32,
}

fn build(mobiles: &[(Vec2, f32)], statics: &[(Vec2, f32)]) -> Fixture {
    let mut app = headless_app(HZ);
    let origin = PackageId(ORIGIN.to_owned());
    let mut registered = std::collections::BTreeSet::new();
    let mut placements = Vec::new();
    for &(pos, radius) in mobiles {
        let id = format!("m{}", (radius * 100.0).round() as u32);
        if registered.insert(id.clone()) {
            app.world_mut()
                .resource_mut::<UnitTypes>()
                .register_unit_type(&origin, def(&id, radius, Some(SPEED)))
                .unwrap();
        }
        placements.push(Placement {
            unit: format!("{ORIGIN}:{id}"),
            owner_slot: 0,
            pos: WorldPos { x: pos.x, y: pos.y },
            contents: Vec::new(),
            facing: None,
        });
    }
    for &(pos, radius) in statics {
        let id = format!("s{}", (radius * 100.0).round() as u32);
        if registered.insert(id.clone()) {
            app.world_mut()
                .resource_mut::<UnitTypes>()
                .register_unit_type(&origin, def(&id, radius, None))
                .unwrap();
        }
        placements.push(Placement {
            unit: format!("{ORIGIN}:{id}"),
            owner_slot: 0,
            pos: WorldPos { x: pos.x, y: pos.y },
            contents: Vec::new(),
            facing: None,
        });
    }
    load_map(
        app.world_mut(),
        &origin,
        &MapDef {
            id: "probe".into(),
            size: MAP,
            placements,
            nodes: Vec::new(),
            slot_economy: Vec::new(),
            belts: Vec::new(),
        },
    )
    .unwrap();
    let positions = positions(&mut app);
    let mobiles: Vec<_> = mobiles
        .iter()
        .enumerate()
        .map(|(i, &(pos, radius))| {
            let id = (i + 1) as u32;
            assert_eq!(positions[&id], pos, "placement order gives unit ids");
            (id, pos, radius)
        })
        .collect();
    Fixture {
        app,
        mobiles,
        statics: statics.to_vec(),
        seq: 1,
    }
}

fn positions(app: &mut bevy_app::App) -> BTreeMap<u32, Vec2> {
    app.world_mut()
        .query::<(&UnitId, &Position)>()
        .iter(app.world())
        .map(|(id, position)| (id.0, position.0))
        .collect()
}

fn move_cmd(seq: u32, ids: Vec<u32>, target: Vec2, group: u32, queue: QueueMode) -> Command {
    Command {
        slot: SlotId(0),
        seq,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("probe:move").unwrap(),
            behaviour: turts_mod_api::Behaviour::Move,
            target_kind: turts_mod_api::TargetKind::PointOrUnit,
            policy: None,
            units: ids,
            target: OrderTargetSim::Point(target),
            queue,
            group,
            allowed_units: Vec::new(),
            queue_depth: 16,
        })),
    }
}

#[derive(Clone, Debug, Default)]
struct Track {
    start: Vec2,
    radius: f32,
    active: bool,
    end: Option<u32>,
    blocked: bool,
    legs_blocked: u32,
    last_fail_tick: Option<u32>,
    last_move: u32,
    prev: Vec2,
    final_pos: Vec2,
    min_static_gap: f32,
    deep: Option<String>,
}

struct Outcome {
    line: String,
}

/// Run one move scenario. `legs` are the points of the order chain (Replace, then Append).
fn run_moves(label: &str, mut fx: Fixture, legs: &[Vec2], group: u32) -> Outcome {
    let ids: Vec<u32> = fx.mobiles.iter().map(|(id, ..)| *id).collect();
    let target = *legs.last().unwrap();
    for (index, leg) in legs.iter().enumerate() {
        let queue = if index == 0 {
            QueueMode::Replace
        } else {
            QueueMode::Append
        };
        let seq = fx.seq;
        fx.seq += 1;
        fx.app
            .world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(move_cmd(seq, ids.clone(), *leg, group, queue));
    }
    let mut tracks: BTreeMap<u32, Track> = fx
        .mobiles
        .iter()
        .map(|&(id, pos, radius)| {
            (
                id,
                Track {
                    start: pos,
                    radius,
                    prev: pos,
                    final_pos: pos,
                    min_static_gap: f32::INFINITY,
                    ..Default::default()
                },
            )
        })
        .collect();
    // Path length through the legs from the group centroid: used for the time limit.
    let path_len = |start: Vec2| {
        let mut length = 0.0;
        let mut from = start;
        for leg in legs {
            length += from.distance(*leg);
            from = *leg;
        }
        length
    };
    let limit_tick = |start: Vec2| ((3.0 * path_len(start) / SPEED + 10.0) * HZ as f32).ceil() as u32 + 1;
    let max_limit = tracks.values().map(|t| limit_tick(t.start)).max().unwrap();
    let mut pools_seen = std::collections::BTreeSet::new();
    while fx.app.world().resource::<Tick>().0 < max_limit {
        fx.app.update();
        let tick = fx.app.world().resource::<Tick>().0;
        let rows: Vec<(u32, Vec2, Option<UnitOrders>, bool, Option<(u64, turts_sim::movement::MovementActivity)>, bool)> = fx
            .app
            .world_mut()
            .query::<(&UnitId, &Position, Option<&UnitOrders>, Option<&turts_sim::MoveTarget>, Option<&turts_sim::movement::MoveProgress>, Option<&turts_sim::movement::PhysicalEscape>)>()
            .iter(fx.app.world())
            .map(|(id, pos, orders, target, progress, escape)| (id.0, pos.0, orders.cloned(), target.is_some(), progress.map(|p| (p.pool_id, p.activity)), escape.is_some()))
            .collect();
        for (id, pos, orders, has_target, pool, escaping) in rows {
            let activity = pool.map(|p| p.1);
            let pool = pool.map(|p| p.0);
            let Some(track) = tracks.get_mut(&id) else { continue };
            if let Some(pool) = pool {
                pools_seen.insert(pool);
            }
            for &(spos, sr) in &fx.statics {
                let g = pos.distance(spos) - sr - track.radius;
                if g < track.min_static_gap {
                    track.min_static_gap = g;
                    if g < -0.05 {
                        track.deep = Some(format!("tick {tick} gap {g:.3} escape={escaping} activity={activity:?}"));
                    }
                }
            }
            if pos != track.prev {
                track.last_move = tick;
                track.prev = pos;
            }
            track.final_pos = pos;
            if track.end.is_some() {
                continue;
            }
            let orders = orders.unwrap_or_default();
            if orders.current.is_some() {
                track.active = true;
            }
            let blocked_failure = orders
                .last_failure
                .as_ref()
                .filter(|failure| failure.reason == RejectReason::Blocked)
                .map(|failure| failure.tick);
            if blocked_failure.is_some() && blocked_failure != track.last_fail_tick {
                track.legs_blocked += 1;
                track.last_fail_tick = blocked_failure;
            }
            if (track.active || blocked_failure.is_some() || track.last_move > 0)
                && orders.current.is_none()
                && orders.queue.is_empty()
                && !has_target
            {
                track.end = Some(tick);
                track.blocked = blocked_failure.is_some_and(|fail| fail + 2 >= tick);
            }
        }
    }
    let n = tracks.len();
    let mut rest_gap = f32::INFINITY;
    for track in tracks.values() {
        for &(spos, sr) in &fx.statics {
            rest_gap = rest_gap.min(track.final_pos.distance(spos) - sr - track.radius);
        }
    }
    let mut done = 0;
    let mut blocked = 0;
    let mut open = 0;
    let mut blocked_dist: Vec<f32> = Vec::new();
    let mut done_dist_max = 0.0_f32;
    let mut last_arrival = 0u32;
    let mut last_arrival_straight = 0.0_f32;
    let mut worst_ratio = 0.0_f32;
    let mut min_gap = f32::INFINITY;
    let mut legs_blocked = 0;
    for track in tracks.values() {
        min_gap = min_gap.min(track.min_static_gap);
        legs_blocked += track.legs_blocked;
        let limit = limit_tick(track.start);
        match track.end {
            Some(end) if end <= limit => {
                if track.blocked {
                    blocked += 1;
                    blocked_dist.push(track.final_pos.distance(target));
                } else {
                    done += 1;
                    done_dist_max = done_dist_max.max(track.final_pos.distance(target));
                    let straight = path_len(track.start)
                        .max(track.start.distance(track.final_pos))
                        / SPEED;
                    let straight_direct = track.start.distance(track.final_pos) / SPEED;
                    let _ = straight;
                    let arrival = track.last_move as f32 / HZ as f32;
                    if track.last_move >= last_arrival {
                        last_arrival = track.last_move;
                        last_arrival_straight = straight_direct;
                    }
                    if straight_direct > 0.0 {
                        worst_ratio = worst_ratio.max(arrival / straight_direct);
                    }
                }
            }
            _ => open += 1,
        }
    }
    blocked_dist.sort_by(f32::total_cmp);
    let bd = if blocked_dist.is_empty() {
        "-".to_string()
    } else {
        format!(
            "{:.2}..{:.2}",
            blocked_dist.first().unwrap(),
            blocked_dist.last().unwrap()
        )
    };
    let mut line = String::new();
    write!(
        line,
        "ROW {label} | n={n} | done={done} | blocked={blocked} (dist to target {bd}) | not_ended_by_limit={open} | legs_blocked={legs_blocked} | last_arrival={:.2}s vs its straight {:.2}s | worst arrival/straight={:.2} | done max dist to point={:.2} | min gap to statics={:.3} | gap at end={rest_gap:.3} | pools={}",
        last_arrival as f32 / HZ as f32,
        last_arrival_straight,
        worst_ratio,
        done_dist_max,
        min_gap,
        pools_seen.len()
    )
    .unwrap();
    for (id, track) in &tracks {
        if let Some(deep) = &track.deep {
            write!(line, "\n    DEEP {label} unit {id}: {deep}").unwrap();
        }
        if track.blocked || track.end.is_none() {
            write!(
                line,
                "\n    DETAIL {label} unit {id}: end={:?} blocked={} final=({:.2},{:.2}) dist={:.2}",
                track.end,
                track.blocked,
                track.final_pos.x,
                track.final_pos.y,
                track.final_pos.distance(target)
            )
            .unwrap();
        }
    }
    Outcome { line }
}

fn block(center: Vec2, count: usize, columns: usize, pitch: f32) -> Vec<Vec2> {
    let rows = count.div_ceil(columns);
    (0..count)
        .map(|i| {
            let c = (i % columns) as f32 - (columns as f32 - 1.0) / 2.0;
            let r = (i / columns) as f32 - (rows as f32 - 1.0) / 2.0;
            // Columns are along y (lateral), rows along x (depth).
            Vec2::new(center.x + r * pitch, center.y + c * pitch)
        })
        .collect()
}

fn print_all(rows: Vec<Outcome>) {
    let mut out = String::new();
    for row in rows {
        out.push_str(&row.line);
        out.push('\n');
    }
    print!("{out}");
}

#[test]
fn s1_single_unit_one_static() {
    let mut rows = Vec::new();
    for group in [7u32, 0] {
        for offset in [0.0_f32, 0.5, 1.5, 2.5, 3.0, -0.5, -1.5, -2.5, -3.0] {
            let fx = build(
                &[(Vec2::new(20.0, 36.0), 0.4)],
                &[(Vec2::new(40.0, 36.0 + offset), 2.5)],
            );
            rows.push(run_moves(
                &format!("S1 g{group} offset {offset:+.1}"),
                fx,
                &[Vec2::new(70.0, 36.0)],
                group,
            ));
        }
        // Control: no obstacle.
        let fx = build(&[(Vec2::new(20.0, 36.0), 0.4)], &[]);
        rows.push(run_moves(
            &format!("S1 g{group} control"),
            fx,
            &[Vec2::new(70.0, 36.0)],
            group,
        ));
    }
    print_all(rows);
}

#[test]
fn s2_group_one_static() {
    let mut rows = Vec::new();
    for offset in [0.0_f32, 1.5, -1.5, 3.0] {
        let starts = block(Vec2::new(20.0, 36.0), 20, 5, 1.0);
        let mobiles: Vec<_> = starts.iter().map(|p| (*p, 0.4)).collect();
        let fx = build(&mobiles, &[(Vec2::new(40.0, 36.0 + offset), 2.5)]);
        rows.push(run_moves(
            &format!("S2 20xr0.4 offset {offset:+.1}"),
            fx,
            &[Vec2::new(70.0, 36.0)],
            7,
        ));
    }
    let starts = block(Vec2::new(20.0, 36.0), 20, 5, 1.0);
    let mobiles: Vec<_> = starts.iter().map(|p| (*p, 0.4)).collect();
    let fx = build(&mobiles, &[]);
    rows.push(run_moves("S2 control", fx, &[Vec2::new(70.0, 36.0)], 7));
    print_all(rows);
}

/// A compact base round a radius 2.5 centre at C: five satellites on an arc facing the
/// approach side, each at `gap` from the centre footprint.
fn base(c: Vec2, gap: f32, step: f32) -> Vec<(Vec2, f32)> {
    let mut statics = vec![(c, 2.5)];
    // (radius, angle in degrees); 180 faces the approach from the west.
    let satellites = [
        (1.5, 180.0 - 2.0 * step),
        (1.0, 180.0 - step),
        (1.5, 180.0),
        (1.0, 180.0 + step),
        (1.5, 180.0 + 2.0 * step),
    ];
    for (r, deg) in satellites {
        let d = 2.5 + r + gap;
        let a = deg.to_radians();
        statics.push((c + Vec2::new(a.cos(), a.sin()) * d, r));
    }
    statics
}

fn gaps(statics: &[(Vec2, f32)]) -> String {
    let mut out = String::new();
    let mut min = f32::INFINITY;
    for i in 0..statics.len() {
        for j in i + 1..statics.len() {
            let g = statics[i].0.distance(statics[j].0) - statics[i].1 - statics[j].1;
            min = min.min(g);
            if g < 3.0 {
                write!(out, "[{i}-{j}: {g:.2}] ").unwrap();
            }
        }
    }
    format!("min gap {min:.2}; gaps below 3.0: {out}")
}

#[test]
fn s3_base() {
    let mut rows = Vec::new();
    let c = Vec2::new(56.0, 36.0);
    for (gap, step) in [(1.0_f32, 40.0_f32), (2.0, 40.0), (0.5, 35.0), (2.0, 50.0), (1.0, 72.0)] {
        let statics = base(c, gap, step);
        let gap_label = format!("{gap}/{step}");
        let mut line = format!("BASE gap {gap_label}: ");
        for (p, r) in &statics {
            write!(line, "({:.2},{:.2}) r{r}; ", p.x, p.y).unwrap();
        }
        line.push_str(&gaps(&statics));
        rows.push(Outcome { line });
        for (label, radius, pitch, count) in [
            ("20xr0.4", 0.4_f32, 1.0_f32, 20usize),
            ("1xr0.9", 0.9, 2.0, 1),
            ("5xr0.9", 0.9, 2.0, 5),
            ("20xr0.9", 0.9, 2.0, 20),
        ] {
            let starts = if count == 1 {
                vec![Vec2::new(30.0, 36.0)]
            } else {
                block(Vec2::new(30.0, 36.0), count, 5.min(count), pitch)
            };
            let mobiles: Vec<_> = starts.iter().map(|p| (*p, radius)).collect();
            let fx = build(&mobiles, &statics);
            rows.push(run_moves(
                &format!("S3 gap{gap_label} {label}"),
                fx,
                &[Vec2::new(82.0, 36.0)],
                7,
            ));
            if gap == 1.0 && step == 40.0 {
                // S6: two waypoints round the base by hand (south side), then the point.
                let fx = build(&mobiles, &statics);
                rows.push(run_moves(
                    &format!("S6 base gap{gap} {label} via (46,24),(66,24)"),
                    fx,
                    &[Vec2::new(46.0, 24.0), Vec2::new(66.0, 24.0), Vec2::new(82.0, 36.0)],
                    7,
                ));
            }
        }
    }
    print_all(rows);
}

#[test]
fn s5_target_inside_static() {
    let mut rows = Vec::new();
    let c = Vec2::new(56.0, 36.0);
    for (label, count, group, point) in [
        ("1xr0.4 g7 centre", 1usize, 7u32, c),
        ("1xr0.4 g0 centre", 1, 0, c),
        ("1xr0.4 g7 inside +1.5", 1, 7, c + Vec2::new(-1.5, 0.0)),
        ("20xr0.4 g7 centre", 20, 7, c),
    ] {
        let starts = if count == 1 {
            vec![Vec2::new(30.0, 36.0)]
        } else {
            block(Vec2::new(30.0, 36.0), count, 5, 1.0)
        };
        let mobiles: Vec<_> = starts.iter().map(|p| (*p, 0.4)).collect();
        let fx = build(&mobiles, &[(c, 2.5)]);
        let mut row = run_moves(&format!("S5 {label}"), fx, &[point], group);
        // Report the ring: distance of each final position from the centre.
        row.line.push_str(" (dist to point here = distance from the click point)");
        rows.push(row);
    }
    print_all(rows);
}

#[test]
fn s6_waypoints_single_static() {
    let mut rows = Vec::new();
    // Worst cases from S1 and S2 are re-run with waypoints round the south side.
    for offset in [0.0_f32, -1.5, 1.5] {
        let fx = build(
            &[(Vec2::new(20.0, 36.0), 0.4)],
            &[(Vec2::new(40.0, 36.0 + offset), 2.5)],
        );
        rows.push(run_moves(
            &format!("S6 1xr0.4 offset {offset:+.1} via (36,31),(44,31)"),
            fx,
            &[Vec2::new(36.0, 31.0), Vec2::new(44.0, 31.0), Vec2::new(70.0, 36.0)],
            7,
        ));
        let starts = block(Vec2::new(20.0, 36.0), 20, 5, 1.0);
        let mobiles: Vec<_> = starts.iter().map(|p| (*p, 0.4)).collect();
        let fx = build(&mobiles, &[(Vec2::new(40.0, 36.0 + offset), 2.5)]);
        rows.push(run_moves(
            &format!("S6 20xr0.4 offset {offset:+.1} via (34,28),(46,28)"),
            fx,
            &[Vec2::new(34.0, 28.0), Vec2::new(46.0, 28.0), Vec2::new(70.0, 36.0)],
            7,
        ));
    }
    print_all(rows);
}

// ---------- S4: the real Gather behaviour on the `base` package ----------

fn gather_fixture(pylon: Option<Vec2>) -> (bevy_app::App, turts_mod_api::ContentCatalog) {
    let package_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let archive =
        PackageArchive::parse(&pack_dir(&package_dir).expect("base packs")).expect("base parses");
    let mut catalog = archive.content().expect("base catalog");
    let cc = ContentId::parse("base:command_center").unwrap();
    let harvester = ContentId::parse("base:harvester").unwrap();
    let rifleman = ContentId::parse("base:rifleman").unwrap();
    catalog.units.get_mut(&cc).unwrap().footprint = Footprint::Circle { radius: 2.5 };
    {
        let h = catalog.units.get_mut(&harvester).unwrap();
        h.footprint = Footprint::Circle { radius: 0.9 };
        h.movement = Some(MovementDef { speed: SPEED });
    }
    let mut p = catalog.units[&rifleman].clone();
    let pylon_id = ContentId::parse("base:pylon").unwrap();
    p.id = pylon_id.clone();
    p.name = "Pylon".into();
    p.movement = None;
    p.footprint = Footprint::Circle { radius: 1.5 };
    catalog.units.insert(pylon_id, p);
    let mut placements = vec![
        Placement {
            unit: "base:harvester".into(),
            owner_slot: 0,
            pos: WorldPos { x: 31.0, y: 36.0 },
            contents: Vec::new(),
            facing: None,
        },
        Placement {
            unit: "base:command_center".into(),
            owner_slot: 0,
            pos: WorldPos { x: 20.0, y: 36.0 },
            contents: Vec::new(),
            facing: None,
        },
    ];
    if let Some(at) = pylon {
        placements.push(Placement {
            unit: "base:pylon".into(),
            owner_slot: 0,
            pos: WorldPos { x: at.x, y: at.y },
            contents: Vec::new(),
            facing: None,
        });
    }
    let map = MapDef {
        id: "probe".into(),
        size: MAP,
        placements,
        nodes: vec![NodePlacement {
            node: "base:ore_deposit".into(),
            pos: WorldPos { x: 35.0, y: 36.0 },
            remaining: 10_000_000,
        }],
        slot_economy: vec![turts_mod_api::SlotEconomyInit {
            slot: 0,
            balances: vec![turts_mod_api::BalanceAmount {
                balance: "base:credits".into(),
                amount: 0,
            }],
        }],
        belts: Vec::new(),
    };
    let mut app = headless_app(HZ);
    let staged = stage_package(
        app.world(),
        catalog.clone(),
        SourceDef {
            file: "maps/probe.ron".into(),
            value: map,
        },
        HZ,
    )
    .expect("probe map stages");
    install_package(app.world_mut(), staged);
    (app, catalog)
}

fn run_gather(label: &str, pylon: Option<Vec2>, group: u32, free_cycle: Option<f32>) -> (String, Option<f32>) {
    let (mut app, catalog) = gather_fixture(pylon);
    let view = turts_sim::world_view(app.world_mut());
    let node = view.nodes[0].id;
    let harvester_id = view
        .units
        .iter()
        .find(|u| u.pos == Vec2::new(31.0, 36.0))
        .map(|u| u.unit_id)
        .expect("harvester");
    let (order, definition) = catalog
        .orders
        .iter()
        .find(|(id, _)| id.local == "order_gather")
        .map(|(id, d)| (id.clone(), d.clone()))
        .unwrap();
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order,
                behaviour: definition.behaviour,
                target_kind: definition.target,
                policy: None,
                units: vec![harvester_id],
                target: OrderTargetSim::Node(node),
                queue: QueueMode::Replace,
                group,
                allowed_units: definition.units.clone(),
                queue_depth: 16,
            })),
        });
    let credits = ContentId::parse("base:credits").unwrap();
    let trips_wanted = 20u64;
    // Limit: 20 x (3 x free cycle + 10 s); with no free cycle yet, 20 x 60 s.
    let limit_secs = free_cycle.map_or(20.0 * 60.0, |c| 20.0 * (3.0 * c + 10.0));
    let limit = (limit_secs * HZ as f32) as u32;
    let mut trip_ticks = Vec::new();
    let mut blocked_at = None;
    let mut last = 0u64;
    let mut min_gap = f32::INFINITY;
    while app.world().resource::<Tick>().0 < limit {
        app.update();
        let tick = app.world().resource::<Tick>().0;
        let on_hand = turts_sim::balance_view(app.world(), SlotId(0), &credits)
            .map(|b| b.on_hand)
            .unwrap_or(0);
        if on_hand / 100 > last / 100 {
            trip_ticks.push(tick);
        }
        last = on_hand;
        let rows: Vec<(u32, Vec2, Option<UnitOrders>)> = app
            .world_mut()
            .query::<(&UnitId, &Position, Option<&UnitOrders>)>()
            .iter(app.world())
            .map(|(id, pos, orders)| (id.0, pos.0, orders.cloned()))
            .collect();
        for (id, pos, orders) in rows {
            if id != harvester_id {
                continue;
            }
            for (spos, sr) in [(Vec2::new(20.0, 36.0), 2.5)]
                .into_iter()
                .chain(pylon.map(|p| (p, 1.5)))
            {
                min_gap = min_gap.min(pos.distance(spos) - sr - 0.9);
            }
            if blocked_at.is_none()
                && orders
                    .and_then(|o| o.last_failure)
                    .is_some_and(|f| f.reason == RejectReason::Blocked)
            {
                blocked_at = Some((tick, pos));
            }
        }
        if trip_ticks.len() as u64 >= trips_wanted {
            break;
        }
    }
    let cycles: Vec<f32> = trip_ticks
        .windows(2)
        .map(|w| (w[1] - w[0]) as f32 / HZ as f32)
        .collect();
    let mean = if cycles.is_empty() {
        None
    } else {
        Some(cycles.iter().sum::<f32>() / cycles.len() as f32)
    };
    let max = cycles.iter().copied().fold(0.0_f32, f32::max);
    let line = format!(
        "ROW S4 {label} g{group} | trips={} of {trips_wanted} within {:.0}s | first trip at {:.2}s | mean cycle={} | max cycle={max:.2}s | ratio to free={} | blocked={:?} | min gap to statics={min_gap:.3}",
        trip_ticks.len(),
        limit_secs,
        trip_ticks.first().map_or(f32::NAN, |t| *t as f32 / HZ as f32),
        mean.map_or("-".into(), |m| format!("{m:.2}s")),
        match (mean, free_cycle) {
            (Some(m), Some(f)) => format!("{:.2}", m / f),
            _ => "-".into(),
        },
        blocked_at.map(|(t, p)| (t as f32 / HZ as f32, p.x, p.y)),
    );
    (line, mean)
}

#[test]
fn s4_harvester_trip() {
    let mut out = String::new();
    for group in [5u32, 0] {
        let (line, free) = run_gather("free (no pylon)", None, group, None);
        out.push_str(&line);
        out.push('\n');
        for offset in [0.0_f32, 1.0, 2.0, -1.0, -2.0, 2.5] {
            let (line, _) = run_gather(
                &format!("pylon r1.5 mid-path offset {offset:+.1}"),
                Some(Vec2::new(28.25, 36.0 + offset)),
                group,
                free,
            );
            out.push_str(&line);
            out.push('\n');
        }
        for (gap, offset) in [(1.0_f32, 0.0_f32), (1.0, 2.0), (1.0, -2.0), (1.0, 3.0), (2.0, 0.0), (2.5, 0.0), (3.0, 0.0), (4.0, 0.0), (1.0, 6.0)] {
            let (line, _) = run_gather(
                &format!("pylon r1.5 at gap {gap} from CC, offset {offset:+.1}"),
                Some(Vec2::new(20.0 + 2.5 + gap + 1.5, 36.0 + offset)),
                group,
                free,
            );
            out.push_str(&line);
            out.push('\n');
        }
    }
    print!("{out}");
}
