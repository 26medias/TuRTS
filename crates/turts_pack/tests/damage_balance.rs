//! Normal-stat battles. No player input, forced damage, or balance-specific engine path.
use serde::Serialize;
use std::path::Path;
use turts_mod_api::{ContentCatalog, MapDef, Placement, SourceDef, WorldPos};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

#[derive(Debug, Serialize)]
struct Battle {
    case: String,
    placement: usize,
    winner: Option<u8>,
    survivors: [usize; 2],
    surviving_cost: [u64; 2],
    ticks: u32,
    first_hit_tick: Option<u32>,
    timed_out: bool,
}
fn rotation(case: usize) -> f32 {
    [
        0.,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        -std::f32::consts::FRAC_PI_2,
        std::f32::consts::FRAC_PI_4,
    ][case]
}
fn point(case: usize, x: f32, y: f32) -> WorldPos {
    let (sin, cos) = rotation(case).sin_cos();
    WorldPos {
        x: 32. + x * cos - y * sin,
        y: 24. + x * sin + y * cos,
    }
}
fn cost(catalog: &ContentCatalog, kind: &str) -> u64 {
    catalog
        .production
        .values()
        .find(|recipe| recipe.output_unit.local == kind)
        .unwrap()
        .costs
        .iter()
        .map(|cost| cost.amount)
        .sum()
}
fn battle(
    catalog: &ContentCatalog,
    map: &MapDef,
    case: usize,
    label: &str,
    left: (&str, usize),
    right: (&str, usize),
) -> Battle {
    let mut map = map.clone();
    map.nodes.clear();
    map.slot_economy.clear();
    map.belts.clear();
    map.placements.clear();
    for (slot, (kind, count)) in [left, right].into_iter().enumerate() {
        for row in 0..count {
            let lateral = (row as f32 - (count - 1) as f32 / 2.) * 1.35;
            map.placements.push(Placement {
                unit: format!("skirmish:{kind}"),
                owner_slot: slot as u8,
                pos: point(case, if slot == 0 { -6. } else { 6. }, lateral),
                contents: vec![],
                facing: None,
                start_order: None,
            });
        }
    }
    let mut app = turts_sim::headless_app(20);
    let staged = turts_sim::stage_package(
        app.world(),
        catalog.clone(),
        SourceDef {
            file: format!("balance/{label}-{case}.ron"),
            value: map,
        },
        20,
    )
    .unwrap();
    turts_sim::install_package(app.world_mut(), staged);
    let target = (left.1 + 1) as u32;
    let initial_health = catalog
        .units
        .iter()
        .find(|(id, _)| id.local == right.0)
        .unwrap()
        .1
        .health
        .unwrap()
        .max;
    let mut first_hit = None;
    for tick in 1..=6000 {
        app.update();
        let view = turts_sim::completed_world(app.world());
        if first_hit.is_none()
            && (target..target + right.1 as u32).any(|id| {
                view.m3
                    .raw_healths
                    .iter()
                    .find(|health| health.unit == id)
                    .is_none_or(|health| health.current < initial_health)
            })
        {
            first_hit = Some(tick);
        }
        let survivors = [0, 1].map(|slot| {
            view.units
                .iter()
                .filter(|unit| unit.owner_slot == slot)
                .count()
        });
        if survivors.contains(&0) || tick == 6000 {
            return Battle {
                case: label.into(),
                placement: case,
                winner: match survivors {
                    [0, 0] => None,
                    [0, _] => Some(1),
                    [_, 0] => Some(0),
                    _ => None,
                },
                surviving_cost: [
                    survivors[0] as u64 * cost(catalog, left.0),
                    survivors[1] as u64 * cost(catalog, right.0),
                ],
                survivors,
                ticks: tick,
                first_hit_tick: first_hit,
                timed_out: !survivors.contains(&0),
            };
        }
    }
    unreachable!()
}

#[test]
fn damage_balance_five_placements_meet_the_documented_roles() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let catalog = archive.content().unwrap();
    let map = archive.maps().unwrap().remove(0).1;
    let mut rows = Vec::new();
    for case in 0..5 {
        for (name, left, right) in [
            (
                "six_crossbowmen_four_warriors",
                ("soldier", 6),
                ("warrior", 4),
            ),
            ("three_crossbowmen_one_tower", ("soldier", 3), ("tower", 1)),
            ("five_warriors_one_tower", ("warrior", 5), ("tower", 1)),
            (
                "eight_crossbowmen_barracks",
                ("soldier", 8),
                ("barracks", 1),
            ),
            ("eight_warriors_barracks", ("warrior", 8), ("barracks", 1)),
        ] {
            rows.push(battle(&catalog, &map, case, name, left, right));
        }
    }
    let mut failures = Vec::new();
    for side in 0..2 {
        if rows
            .iter()
            .filter(|row| row.case == "six_crossbowmen_four_warriors")
            .all(|row| row.winner == Some(side) && row.surviving_cost[side as usize] > 120)
        {
            failures.push(format!(
                "side {side} wins every equal-cost duel with more than half its cost"
            ));
        }
    }
    for row in &rows {
        if row.timed_out {
            failures.push(format!(
                "{} placement {} times out",
                row.case, row.placement
            ));
        }
        let expected = match row.case.as_str() {
            "three_crossbowmen_one_tower" => Some(1),
            "five_warriors_one_tower"
            | "eight_crossbowmen_barracks"
            | "eight_warriors_barracks" => Some(0),
            _ => None,
        };
        if expected.is_some() && row.winner != expected {
            failures.push(format!(
                "{} placement {} winner {:?}, expected {:?}",
                row.case, row.placement, row.winner, expected
            ));
        }
    }
    for case in 0..5 {
        let bow = rows
            .iter()
            .find(|row| row.placement == case && row.case == "eight_crossbowmen_barracks")
            .unwrap();
        let warrior = rows
            .iter()
            .find(|row| row.placement == case && row.case == "eight_warriors_barracks")
            .unwrap();
        // TTK starts at the first actual hit. Record full approach time separately.
        let fighting = |row: &Battle| row.first_hit_tick.map(|start| row.ticks - start + 1);
        if bow.winner != Some(0)
            || warrior.winner != Some(0)
            || fighting(bow)
                .zip(fighting(warrior))
                .is_none_or(|(bow, warrior)| bow < 2 * warrior)
        {
            failures.push(format!(
                "placement {case}: Barracks TTK bow {:?}, warrior {:?}",
                fighting(bow),
                fighting(warrior)
            ));
        }
    }
    let result = serde_json::json!({"tick_hz":20,"start_anchor_distance":12,"placement_rotations_degrees":[0,90,180,-90,45],"formation_spacing":1.35,"ttk_basis":"first damage tick through destruction, inclusive; full elapsed ticks also recorded","rows":rows,"failures":failures});
    if let Ok(path) = std::env::var("TURTS_DAMAGE_BALANCE_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&result).unwrap()).unwrap();
    }
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    assert!(failures.is_empty(), "{failures:?}");
}
