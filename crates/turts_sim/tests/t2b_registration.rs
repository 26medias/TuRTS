//! T2b registration rules (design 7): the same invalid input is rejected in every package
//! namespace, with a `ModError` that names the field and leaks no schema type name.

use turts_mod_api::{
    Footprint, MapDef, MapSize, ModError, PackageId, Placement, UnitDef, UnitRegistry, WorldPos,
};
use turts_sim::{UnitTypes, gameplay_entity_count, headless_app, load_map};

const NAMESPACES: [&str; 2] = ["base", "acme"];
const TYPE_NAMES: [&str; 4] = ["UnitDef", "MapDef", "Footprint", "Placement"];

fn pkg(name: &str) -> PackageId {
    PackageId(name.to_string())
}

/// The namespace that is not `origin`.
fn other_of(origin: &str) -> &'static str {
    NAMESPACES.iter().copied().find(|n| *n != origin).unwrap()
}

fn unit_def(id: &str, speed: f32, radius: f32) -> UnitDef {
    UnitDef {
        id: id.to_string(),
        name: "Test unit".to_string(),
        speed,
        footprint: Footprint::Circle { radius },
    }
}

fn placement(unit: &str, owner_slot: u8, x: f32, y: f32) -> Placement {
    Placement {
        unit: unit.to_string(),
        owner_slot,
        pos: WorldPos { x, y },
    }
}

fn map_def(size: f32, placements: Vec<Placement>) -> MapDef {
    MapDef {
        id: "field".to_string(),
        size: MapSize { x: size, y: size },
        placements,
    }
}

fn register(app: &mut bevy_app::App, origin: &PackageId, def: UnitDef) -> Result<(), ModError> {
    app.world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(origin, def)
        .map(|_| ())
}

/// Register an unit `scout` in `origin`, then load `map`. Returns the loader error and the
/// gameplay entity count after the call.
fn load_bad_map(origin: &str, map: MapDef) -> (ModError, usize) {
    let origin = pkg(origin);
    let mut app = headless_app(20);
    register(&mut app, &origin, unit_def("scout", 3.0, 0.5)).unwrap();
    let err = load_map(app.world_mut(), &origin, &map).unwrap_err();
    (err, gameplay_entity_count(app.world_mut()))
}

fn assert_clean(err: &ModError, origin: &str) {
    assert_eq!(err.package, origin);
    assert_eq!(err.file, None, "the registry leaves `file` for the loader");
    for name in TYPE_NAMES {
        assert!(
            !err.message.contains(name),
            "message leaks a type name: {}",
            err.message
        );
    }
}

/// Every case returns the errors it provoked, for the origin it gets and the other namespace.
type Case = fn(&str, &str) -> Vec<ModError>;

fn case_local_id_with_colon(origin: &str, other: &str) -> Vec<ModError> {
    let mut app = headless_app(20);
    let origin = pkg(origin);
    vec![
        register(
            &mut app,
            &origin,
            unit_def(&format!("{other}:scout"), 3.0, 0.5),
        )
        .unwrap_err(),
    ]
}

fn case_speed_out_of_range(origin: &str, _other: &str) -> Vec<ModError> {
    let mut app = headless_app(20);
    let origin = pkg(origin);
    [0.0, -1.0, f32::NAN, f32::INFINITY, 101.0]
        .into_iter()
        .map(|speed| register(&mut app, &origin, unit_def("scout", speed, 0.5)).unwrap_err())
        .collect()
}

fn case_radius_out_of_range(origin: &str, _other: &str) -> Vec<ModError> {
    let mut app = headless_app(20);
    let origin = pkg(origin);
    [0.0, -0.5, f32::NAN, 50.5]
        .into_iter()
        .map(|radius| register(&mut app, &origin, unit_def("scout", 3.0, radius)).unwrap_err())
        .collect()
}

fn case_duplicate_content_id(origin: &str, _other: &str) -> Vec<ModError> {
    let mut app = headless_app(20);
    let origin = pkg(origin);
    register(&mut app, &origin, unit_def("scout", 3.0, 0.5)).unwrap();
    vec![register(&mut app, &origin, unit_def("scout", 4.0, 0.5)).unwrap_err()]
}

fn case_map_unit_of_other_package(origin: &str, other: &str) -> Vec<ModError> {
    let map = map_def(
        64.0,
        vec![placement(&format!("{other}:scout"), 0, 8.0, 8.0)],
    );
    vec![load_bad_map(origin, map).0]
}

fn case_map_owner_slot_2(origin: &str, _other: &str) -> Vec<ModError> {
    let map = map_def(
        64.0,
        vec![placement(&format!("{origin}:scout"), 2, 8.0, 8.0)],
    );
    vec![load_bad_map(origin, map).0]
}

fn case_map_pos_outside_size(origin: &str, _other: &str) -> Vec<ModError> {
    [(70.0, 8.0), (8.0, -1.0), (f32::NAN, 8.0)]
        .into_iter()
        .map(|(x, y)| {
            let map = map_def(64.0, vec![placement(&format!("{origin}:scout"), 0, x, y)]);
            load_bad_map(origin, map).0
        })
        .collect()
}

fn case_map_size_invalid(origin: &str, _other: &str) -> Vec<ModError> {
    [0.0, -64.0, f32::NAN, f32::INFINITY]
        .into_iter()
        .map(|size| load_bad_map(origin, map_def(size, vec![])).0)
        .collect()
}

fn case_map_unknown_unit(origin: &str, _other: &str) -> Vec<ModError> {
    let map = map_def(
        64.0,
        vec![placement(&format!("{origin}:ghost"), 0, 8.0, 8.0)],
    );
    vec![load_bad_map(origin, map).0]
}

/// Name, expected field path (one per error), case.
fn cases() -> Vec<(&'static str, Vec<&'static str>, Case)> {
    vec![
        ("local id with colon", vec!["id"], case_local_id_with_colon),
        ("speed", vec!["speed"; 5], case_speed_out_of_range),
        (
            "radius",
            vec!["footprint.radius"; 4],
            case_radius_out_of_range,
        ),
        ("duplicate id", vec!["id"], case_duplicate_content_id),
        (
            "map unit of other package",
            vec!["placements[0].unit"],
            case_map_unit_of_other_package,
        ),
        (
            "map owner slot",
            vec!["placements[0].owner_slot"],
            case_map_owner_slot_2,
        ),
        (
            "map pos",
            vec!["placements[0].pos"; 3],
            case_map_pos_outside_size,
        ),
        ("map size", vec!["size"; 4], case_map_size_invalid),
        (
            "map unknown unit",
            vec!["placements[0].unit"],
            case_map_unknown_unit,
        ),
    ]
}

fn run(case: Case, origin: &str) -> Vec<ModError> {
    case(origin, other_of(origin))
}

#[test]
fn t2b_local_id_with_colon_rejected() {
    for origin in NAMESPACES {
        let errs = run(case_local_id_with_colon, origin);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].field_path, "id");
        assert!(errs[0].message.contains("must not contain `:`"));
        assert_clean(&errs[0], origin);
    }
}

#[test]
fn t2b_speed_out_of_range_rejected() {
    for origin in NAMESPACES {
        for err in run(case_speed_out_of_range, origin) {
            assert_eq!(err.field_path, "speed");
            assert_clean(&err, origin);
        }
        // The upper bound itself is accepted.
        let mut app = headless_app(20);
        register(&mut app, &pkg(origin), unit_def("fast", 100.0, 0.5)).unwrap();
    }
}

#[test]
fn t2b_radius_out_of_range_rejected() {
    for origin in NAMESPACES {
        for err in run(case_radius_out_of_range, origin) {
            assert_eq!(err.field_path, "footprint.radius");
            assert_clean(&err, origin);
        }
        let mut app = headless_app(20);
        register(&mut app, &pkg(origin), unit_def("wide", 3.0, 50.0)).unwrap();
    }
}

#[test]
fn t2b_duplicate_content_id_rejected() {
    for origin in NAMESPACES {
        let errs = run(case_duplicate_content_id, origin);
        assert_eq!(errs[0].field_path, "id");
        assert!(errs[0].message.contains("already registered"));
        assert!(errs[0].message.contains(&format!("{origin}:scout")));
        assert_clean(&errs[0], origin);
    }
}

#[test]
fn t2b_map_unit_of_other_package_rejected() {
    let (err, _) = load_bad_map(
        "acme",
        map_def(64.0, vec![placement("base:rifleman", 0, 8.0, 8.0)]),
    );
    assert_eq!(err.field_path, "placements[0].unit");
    assert!(err.message.contains("is not in package `acme`"));
    assert_clean(&err, "acme");
    for origin in NAMESPACES {
        let errs = run(case_map_unit_of_other_package, origin);
        assert_eq!(errs[0].field_path, "placements[0].unit");
    }
}

#[test]
fn t2b_map_owner_slot_2_rejected() {
    for origin in NAMESPACES {
        let errs = run(case_map_owner_slot_2, origin);
        assert_eq!(errs[0].field_path, "placements[0].owner_slot");
        assert_clean(&errs[0], origin);
    }
}

#[test]
fn t2b_map_pos_outside_size_rejected() {
    for origin in NAMESPACES {
        let map = map_def(
            64.0,
            vec![placement(&format!("{origin}:scout"), 0, 70.0, 8.0)],
        );
        let (err, spawned) = load_bad_map(origin, map);
        assert_eq!(err.field_path, "placements[0].pos");
        assert_clean(&err, origin);
        assert_eq!(spawned, 0, "a rejected map spawns nothing");
        for err in run(case_map_pos_outside_size, origin) {
            assert_eq!(err.field_path, "placements[0].pos");
        }
    }
}

#[test]
fn t2b_bad_second_placement_spawns_nothing() {
    // The first placement is valid. The check runs before any spawn.
    for origin in NAMESPACES {
        let good = placement(&format!("{origin}:scout"), 0, 8.0, 8.0);
        let bad = placement(&format!("{origin}:scout"), 0, 70.0, 8.0);
        let (err, spawned) = load_bad_map(origin, map_def(64.0, vec![good, bad]));
        assert_eq!(err.field_path, "placements[1].pos");
        assert_eq!(spawned, 0);
    }
}

#[test]
fn t2b_map_size_and_unknown_unit_rejected() {
    for origin in NAMESPACES {
        for err in run(case_map_size_invalid, origin) {
            assert_eq!(err.field_path, "size");
            assert_clean(&err, origin);
        }
        let errs = run(case_map_unknown_unit, origin);
        assert_eq!(errs[0].field_path, "placements[0].unit");
        assert!(errs[0].message.contains("unknown unit type"));
    }
}

#[test]
fn t2b_same_rules_for_both_namespaces() {
    let normalise = |m: &str| m.replace("base", "<pkg>").replace("acme", "<pkg>");
    for (name, expected_fields, case) in cases() {
        let base = run(case, "base");
        let acme = run(case, "acme");
        assert_eq!(
            base.len(),
            expected_fields.len(),
            "{name}: base error count"
        );
        assert_eq!(
            acme.len(),
            expected_fields.len(),
            "{name}: acme error count"
        );
        for (i, field) in expected_fields.iter().enumerate() {
            assert_eq!(base[i].field_path, *field, "{name}[{i}]: base field");
            assert_eq!(acme[i].field_path, *field, "{name}[{i}]: acme field");
            assert_eq!(
                normalise(&base[i].message),
                normalise(&acme[i].message),
                "{name}[{i}]: messages differ beyond the package name"
            );
            assert_clean(&base[i], "base");
            assert_clean(&acme[i], "acme");
        }
    }
}
