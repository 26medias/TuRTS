//! API 0.2.18 (M6) public utility authoring: `mods/utility_yard` loads, and each refusal of
//! `install_utilities` fires with its field path. Every case edits the real fixture files.
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use turts_mod_api::{ContentId, ModError, SupplyScope, UtilityExposure, UtilitySymbol};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::pack::pack_files;

type Files = BTreeMap<String, Vec<u8>>;

fn fixture() -> Files {
    fn walk(root: &Path, dir: &Path, out: &mut Files) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let key = path.strip_prefix(root).unwrap().to_str().unwrap();
                out.insert(key.replace('\\', "/"), fs::read(&path).unwrap());
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/utility_yard");
    let mut out = Files::new();
    walk(&root, &root, &mut out);
    out
}

fn edit(files: &mut Files, path: &str, before: &str, after: &str) {
    let text = String::from_utf8(files[path].clone()).unwrap();
    assert!(text.contains(before), "`{before}` is not in {path}");
    files.insert(path.into(), text.replacen(before, after, 1).into_bytes());
}

fn content(files: &Files) -> Result<turts_mod_api::ContentCatalog, ModError> {
    PackageArchive::parse(&pack_files(files).unwrap())
        .unwrap()
        .content()
}

/// Apply `change` to the fixture and expect one refusal at `(file, field)` with `needle` in it.
fn refused(change: impl FnOnce(&mut Files), file: &str, field: &str, needle: &str) {
    let mut files = fixture();
    change(&mut files);
    let error = content(&files).expect_err("the package must be refused");
    assert_eq!(
        (error.file.as_deref(), error.field_path.as_str()),
        (Some(file), field),
        "{error}"
    );
    assert!(error.message.contains(needle), "{error}");
}

const LINE: &str = "utilities/power_line.ron";

#[test]
fn utility_yard_loads_and_resolves_its_utility_type() {
    let archive = PackageArchive::parse(&pack_files(&fixture()).unwrap()).unwrap();
    let catalog = archive.content().unwrap();
    assert_eq!(catalog.utilities.len(), 1);
    let line = &catalog.utilities[&ContentId::parse("utility_yard:power_line").unwrap()];
    assert_eq!(
        line.carries,
        ContentId::parse("utility_yard:power").unwrap()
    );
    assert_eq!(
        (line.depth.min, line.depth.default, line.depth.max),
        (1, 2, 4)
    );
    assert_eq!((line.reach, line.max_segment_length), (40.0, 16.0));
    assert_eq!(line.exposure, UtilityExposure::Disconnect);
    assert_eq!(line.appearance.junction, UtilitySymbol::Diamond);
    assert_eq!(line.appearance.labels.short, "Underpowered");
    assert_eq!(
        line.segment_cost,
        vec![(ContentId::parse("utility_yard:loam").unwrap(), 1)]
    );
    let ports: Vec<_> = line.ports.keys().map(ToString::to_string).collect();
    assert_eq!(ports, ["utility_yard:generator", "utility_yard:workshop"]);
    assert_eq!(
        catalog.source_files[&line.id], LINE,
        "the source file of the utility type is recorded"
    );
    assert!(matches!(
        catalog.resources[&line.carries].kind,
        turts_mod_api::ResourceKind::Supply {
            scope: SupplyScope::Network,
            ..
        }
    ));
    // The terrain and operating checks still run on the same package.
    assert_eq!(catalog.terrain_tools.len(), 3);
    assert_eq!(archive.maps().unwrap().len(), 1);
}

#[test]
fn a_bad_depth_range_is_refused_with_its_field() {
    for (before, after, field) in [
        (
            "default:2,min:1,max:4",
            "default:2,min:0,max:4",
            "depth.min",
        ),
        (
            "default:2,min:1,max:4",
            "default:1,min:2,max:4",
            "depth.default",
        ),
        (
            "default:2,min:1,max:4",
            "default:5,min:1,max:4",
            "depth.max",
        ),
        (
            "default:2,min:1,max:4",
            "default:2,min:1,max:65",
            "depth.max",
        ),
    ] {
        refused(|f| edit(f, LINE, before, after), LINE, field, "");
    }
    // 64 is the largest allowed depth.
    let mut files = fixture();
    edit(
        &mut files,
        LINE,
        "default:2,min:1,max:4",
        "default:2,min:1,max:64",
    );
    content(&files).unwrap();
}

#[test]
fn segment_length_and_reach_are_bounded() {
    for (before, after, field) in [
        (
            "max_segment_length:16.0",
            "max_segment_length:0.0",
            "max_segment_length",
        ),
        (
            "max_segment_length:16.0",
            "max_segment_length:-1.0",
            "max_segment_length",
        ),
        (
            "max_segment_length:16.0",
            "max_segment_length:64.5",
            "max_segment_length",
        ),
        (
            "max_segment_length:16.0",
            "max_segment_length:inf",
            "max_segment_length",
        ),
        ("reach:40.0", "reach:0.0", "reach"),
        ("reach:40.0", "reach:-3.0", "reach"),
        ("reach:40.0", "reach:inf", "reach"),
    ] {
        refused(|f| edit(f, LINE, before, after), LINE, field, "");
    }
    let mut files = fixture();
    edit(
        &mut files,
        LINE,
        "max_segment_length:16.0",
        "max_segment_length:64.0",
    );
    content(&files).unwrap();
}

#[test]
fn carries_must_be_a_network_supply_resource_of_this_package() {
    refused(
        |f| edit(f, LINE, "utility_yard:power\"", "utility_yard:nothing\""),
        LINE,
        "carries",
        "unknown resource",
    );
    refused(
        |f| edit(f, LINE, "utility_yard:power\"", "utility_yard:loam\""),
        LINE,
        "carries",
        "Supply resource with scope Network",
    );
    refused(
        |f| edit(f, LINE, "utility_yard:power\"", "other_pkg:power\""),
        LINE,
        "carries",
        "this package",
    );
    // A Slot-scope Supply resource is not a Network resource.
    refused(
        |f| {
            f.insert(
                "resources/slot.ron".into(),
                br#"(id:"slot",name:"Slot",unit_name:"unit",kind:Supply(scope:Slot,counts:Completed))"#
                    .to_vec(),
            );
            edit(
                f,
                "manifest.ron",
                "\"resources/power.ron\"",
                "\"resources/power.ron\",\"resources/slot.ron\"",
            );
            edit(f, LINE, "utility_yard:power\"", "utility_yard:slot\"");
        },
        LINE,
        "carries",
        "scope Network",
    );
}

#[test]
fn each_network_resource_has_exactly_one_utility_type() {
    // A Network resource that no utility type carries.
    refused(
        |f| {
            f.insert(
                "resources/water.ron".into(),
                br#"(id:"water",name:"Water",unit_name:"unit",kind:Supply(scope:Network,counts:Completed))"#
                    .to_vec(),
            );
            edit(
                f,
                "manifest.ron",
                "\"resources/power.ron\"",
                "\"resources/power.ron\",\"resources/water.ron\"",
            );
        },
        "resources/water.ron",
        "kind.scope",
        "needs one utility type",
    );
    // The export list drops the utility: the one resource now has none.
    refused(
        |f| {
            edit(
                f,
                "manifest.ron",
                "utilities:[\"utilities/power_line.ron\"]",
                "utilities:[]",
            )
        },
        "resources/power.ron",
        "kind.scope",
        "needs one utility type",
    );
    // Two utility types for one resource.
    refused(
        |f| {
            let second = String::from_utf8(f[LINE].clone())
                .unwrap()
                .replace("\"power_line\"", "\"power_line_two\"");
            f.insert("utilities/two.ron".into(), second.into_bytes());
            edit(
                f,
                "manifest.ron",
                "\"utilities/power_line.ron\"",
                "\"utilities/power_line.ron\",\"utilities/two.ron\"",
            );
        },
        "utilities/two.ron",
        "carries",
        "already carried",
    );
}

#[test]
fn ports_name_static_unit_types_once_near_their_footprint() {
    refused(
        |f| edit(f, LINE, "utility_yard:workshop\"", "utility_yard:nothing\""),
        LINE,
        "ports[1].unit",
        "unknown unit type",
    );
    // A mobile unit cannot hold a port.
    refused(
        |f| edit(f, LINE, "utility_yard:workshop\"", "utility_yard:porter\""),
        LINE,
        "ports[1].unit",
        "movement",
    );
    refused(
        |f| {
            edit(
                f,
                LINE,
                "utility_yard:workshop\"",
                "utility_yard:generator\"",
            )
        },
        LINE,
        "ports[1].unit",
        "second port",
    );
    // Generator radius 1.2: the limit is 3.2 world units.
    refused(
        |f| edit(f, LINE, "offset:(x:1.2,y:0.0)", "offset:(x:3.3,y:0.0)"),
        LINE,
        "ports[0].offset",
        "within",
    );
    refused(
        |f| edit(f, LINE, "offset:(x:1.2,y:0.0)", "offset:(x:inf,y:0.0)"),
        LINE,
        "ports[0].offset",
        "finite",
    );
    let mut files = fixture();
    edit(
        &mut files,
        LINE,
        "offset:(x:1.2,y:0.0)",
        "offset:(x:3.2,y:0.0)",
    );
    content(&files).unwrap();
}

#[test]
fn a_provider_or_consumer_needs_a_port() {
    const GENERATOR: &str = "(unit:\"utility_yard:generator\",offset:(x:1.2,y:0.0))";
    const WORKSHOP: &str = "(unit:\"utility_yard:workshop\",offset:(x:-1.4,y:0.0))";
    let both = format!("ports:[{GENERATOR},{WORKSHOP}]");
    // Keep the generator port only: the consumer has none.
    refused(
        |f| edit(f, LINE, &both, &format!("ports:[{GENERATOR}]")),
        "units/workshop.ron",
        "consumes[0].resource",
        "no port",
    );
    // Keep the workshop port only: the provider has none.
    refused(
        |f| edit(f, LINE, &both, &format!("ports:[{WORKSHOP}]")),
        "units/generator.ron",
        "provides[0].resource",
        "no port",
    );
}

#[test]
fn a_producer_or_weapon_when_short_on_a_network_resource_needs_a_port() {
    // The workshop keeps its producer `when_short` but loses its consumption and its port.
    refused(
        |f| {
            edit(
                f,
                "units/workshop.ron",
                ",consumes:[(resource:\"utility_yard:power\",amount:6)]",
                "",
            );
            edit(
                f,
                LINE,
                ",(unit:\"utility_yard:workshop\",offset:(x:-1.4,y:0.0))",
                "",
            );
        },
        "units/workshop.ron",
        "producer.when_short[0].resource",
        "no port",
    );
    // A weapon `when_short` on the shed (no port).
    refused(
        |f| {
            edit(
                f,
                "units/shed.ron",
                "health:Some((max:200))",
                "health:Some((max:200)),weapon:Some((range:4.0,cooldown_ms:1000,damage:5,ammo:None,targets:(slots:Enemy),when_short:[(resource:\"utility_yard:power\",effect:Disable)]))",
            );
        },
        "units/shed.ron",
        "weapon.when_short[0].resource",
        "no port",
    );
}

#[test]
fn a_constructor_when_short_cannot_name_a_network_resource() {
    refused(
        |f| {
            edit(
                f,
                "units/depot.ron",
                "queue_capacity:4)",
                "queue_capacity:4,when_short:[(resource:\"utility_yard:power\",effect:Rate((numerator:1,denominator:2)))])",
            );
        },
        "units/depot.ron",
        "constructor.when_short[0].resource",
        "constructor has no port",
    );
}

#[test]
fn segment_cost_names_balance_resources_with_nonzero_amounts() {
    refused(
        |f| {
            edit(
                f,
                LINE,
                "resource:\"utility_yard:loam\"",
                "resource:\"utility_yard:power\"",
            )
        },
        LINE,
        "segment_cost[0].resource",
        "Balance",
    );
    refused(
        |f| {
            edit(
                f,
                LINE,
                "resource:\"utility_yard:loam\"",
                "resource:\"utility_yard:nothing\"",
            )
        },
        LINE,
        "segment_cost[0].resource",
        "unknown resource",
    );
    refused(
        |f| edit(f, LINE, "amount:1)", "amount:0)"),
        LINE,
        "segment_cost[0].amount",
        "positive",
    );
    refused(
        |f| {
            edit(
                f,
                LINE,
                "segment_cost:[(resource:\"utility_yard:loam\",amount:1)]",
                "segment_cost:[(resource:\"utility_yard:loam\",amount:1),(resource:\"utility_yard:loam\",amount:2)]",
            );
        },
        LINE,
        "segment_cost[1].resource",
        "second entry",
    );
    // An empty cost list is allowed.
    let mut files = fixture();
    edit(
        &mut files,
        LINE,
        "segment_cost:[(resource:\"utility_yard:loam\",amount:1)]",
        "segment_cost:[]",
    );
    assert!(
        content(&files)
            .unwrap()
            .utilities
            .values()
            .all(|u| u.segment_cost.is_empty())
    );
}

#[test]
fn colours_use_the_engine_colour_syntax() {
    refused(
        |f| edit(f, LINE, "colour:\"#f2c14e\"", "colour:\"yellow\""),
        LINE,
        "appearance.colour",
        "#RRGGBB",
    );
    refused(
        |f| {
            edit(
                f,
                LINE,
                "exposed_colour:\"#e0503c\"",
                "exposed_colour:\"#e0503\"",
            )
        },
        LINE,
        "appearance.exposed_colour",
        "#RRGGBB",
    );
}

#[test]
fn a_route_order_needs_the_route_target_and_the_list_needs_a_route_order() {
    refused(
        |f| edit(f, "orders/route.ron", "target:UtilityRoute", "target:Point"),
        "orders/route.ron",
        "target",
        "UtilityRoute requires UtilityRoute",
    );
    refused(
        |f| {
            edit(
                f,
                "interface/hud.ron",
                "UtilityRoutes(order:\"utility_yard:order_route\")",
                "UtilityRoutes(order:\"utility_yard:order_move\")",
            );
        },
        "interface/hud.ron",
        "root[4].widget.source.order",
        "behaviour UtilityRoute",
    );
    refused(
        |f| {
            edit(
                f,
                "interface/hud.ron",
                "UtilityRoutes(order:\"utility_yard:order_route\")",
                "UtilityRoutes(order:\"utility_yard:order_missing\")",
            );
        },
        "interface/hud.ron",
        "root[4].widget.source.order",
        "unknown utility route order",
    );
}

#[test]
fn the_new_fields_need_api_0_2_18_and_unknown_fields_get_a_hint() {
    let mut files = fixture();
    edit(&mut files, "manifest.ron", "^0.2.18", "^0.2.17");
    let error = content(&files).expect_err("an older api cannot use utilities");
    assert!(
        error.to_string().contains("0.2.18"),
        "the refusal names the API: {error}"
    );
    let mut files = fixture();
    edit(&mut files, LINE, "reach:40.0", "reech:40.0");
    let error = content(&files).expect_err("an unknown field is refused");
    assert!(
        error.to_string().contains("reach"),
        "did you mean hint: {error}"
    );
}
