use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use turts_mod_api::{
    ActionDef, AssetsDef, Behaviour, Binding, Capability, ContentCatalog, FirePolicy, ImageAsset,
    InputMapDef, Manifest, OrderRulesDef, SmartTarget,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{bootstrap::InstalledCatalog, headless_app};

const PACKAGES: [&str; 5] = ["acme", "base", "cedar", "haul", "forge"];
static TEMP_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn package_dir(id: &str) -> PathBuf {
    repo_root().join("mods").join(id)
}

fn order_ref(package: &str, verb: &str) -> String {
    format!("{package}:order_{verb}")
}

fn order_content_id(package: &str, verb: &str) -> turts_mod_api::ContentId {
    turts_mod_api::ContentId::parse(&order_ref(package, verb)).expect("valid order id")
}

fn unique_temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "turts-mc-packages-{name}-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create temporary package directory");
    for entry in fs::read_dir(source).expect("read package directory") {
        let entry = entry.expect("read package entry");
        let target = destination.join(entry.file_name());
        if entry.file_type().expect("read package entry type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy package file");
        }
    }
}

fn read_manifest(path: &Path) -> Manifest {
    ron::from_str(&fs::read_to_string(path.join("manifest.ron")).expect("read manifest"))
        .expect("parse package manifest")
}

fn write_manifest(path: &Path, manifest: &Manifest) {
    let ron = ron::ser::to_string(manifest)
        .expect("serialize package manifest")
        .replace("dependencies:({})", "dependencies:{}");
    fs::write(path.join("manifest.ron"), ron).expect("write package manifest");
}

fn archive_from_dir(path: &Path) -> PackageArchive {
    let bytes = pack_dir(path).expect("pack temporary package");
    PackageArchive::parse(&bytes).expect("parse temporary package archive")
}

fn add_data_order_fixture_inputs(path: &Path, manifest: &mut Manifest) {
    let input_file = "input/bindings.ron";
    if !manifest.exports.input.iter().any(|file| file == input_file) {
        manifest.exports.input.push(input_file.into());
    }
    let input_path = path.join(input_file);
    let mut input = if input_path.exists() {
        ron::from_str::<InputMapDef>(
            &fs::read_to_string(&input_path).expect("read package bindings"),
        )
        .expect("parse package bindings")
    } else {
        InputMapDef {
            actions: Vec::new(),
            engine_overrides: Vec::new(),
        }
    };
    input.actions.push(ActionDef {
        name: "key_dash".into(),
        default: vec![Binding::Key {
            code: "KeyD".into(),
            mods: Vec::new(),
        }],
    });
    fs::create_dir_all(path.join("input")).expect("create input directory");
    fs::write(
        input_path,
        ron::ser::to_string(&input).expect("serialize package bindings"),
    )
    .expect("write package bindings");

    let assets_file = "assets/assets.ron";
    if !manifest
        .exports
        .assets
        .iter()
        .any(|file| file == assets_file)
    {
        manifest.exports.assets.push(assets_file.into());
    }
    let assets_path = path.join(assets_file);
    let mut assets = if assets_path.exists() {
        ron::from_str::<AssetsDef>(&fs::read_to_string(&assets_path).expect("read package assets"))
            .expect("parse package assets")
    } else {
        AssetsDef {
            images: Vec::new(),
            fonts: Vec::new(),
            sounds: Vec::new(),
            cursors: Vec::new(),
        }
    };
    assets.images.push(ImageAsset {
        id: "test_order_image".into(),
        path: "assets/test_order_image.png".into(),
        nine_slice: None,
    });
    fs::create_dir_all(path.join("assets")).expect("create assets directory");
    fs::write(
        assets_path,
        ron::ser::to_string(&assets).expect("serialize package assets"),
    )
    .expect("write package assets");
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&1u32.to_be_bytes());
    png.extend_from_slice(&1u32.to_be_bytes());
    png.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0]);
    fs::write(path.join("assets/test_order_image.png"), png).expect("write package icon PNG");
}

fn load_catalog(archive: &PackageArchive) -> ContentCatalog {
    let mut app = headless_app(20);
    load_package(app.world_mut(), archive).expect("load package through public loader");
    app.world().resource::<InstalledCatalog>().0.clone()
}

#[test]
fn five_packages_load_with_0_2_3() {
    for id in PACKAGES {
        let archive = archive_from_dir(&package_dir(id));
        let catalog = load_catalog(&archive);
        let moving_unit_ids = catalog
            .units
            .iter()
            .filter(|(_, unit)| unit.movement.is_some())
            .map(|(unit_id, _)| unit_id.to_string())
            .collect::<Vec<_>>();
        assert!(
            catalog.orders.values().any(|order| {
                order.behaviour == Behaviour::Move
                    && order
                        .units
                        .iter()
                        .any(|unit| moving_unit_ids.contains(unit))
            }),
            "{id} exports a Move order usable by a unit with movement"
        );
        assert_eq!(archive.manifest.engine_api, "^0.2.3", "{id} engine API");
    }
}

#[test]
fn old_acme_copy_loads_with_zero_orders() {
    let old_copy = unique_temp_dir("old-acme");
    copy_tree(&package_dir("acme"), &old_copy);
    let mut manifest = read_manifest(&old_copy);
    manifest.engine_api = "^0.2.2".into();
    manifest.exports.orders.clear();
    manifest.exports.interface.clear();
    manifest.exports.input.clear();
    manifest.exports.assets.clear();
    write_manifest(&old_copy, &manifest);

    let archive = archive_from_dir(&old_copy);
    let catalog = load_catalog(&archive);
    assert!(
        catalog.orders.is_empty(),
        "the old package defines no orders"
    );
    fs::remove_dir_all(old_copy).expect("remove old package copy");
}

fn version_row_fixture(row: &str) -> PathBuf {
    let path = unique_temp_dir(&row.replace('.', "-"));
    copy_tree(&package_dir("acme"), &path);
    let mut manifest = read_manifest(&path);
    manifest.engine_api = "^0.2.2".into();
    manifest.exports.orders.clear();
    manifest.exports.interface.clear();
    manifest.exports.input.clear();
    manifest.exports.assets.clear();

    match row {
        "exports.orders" => {
            manifest.exports.orders = vec!["orders/rules.ron".into()];
            fs::create_dir_all(path.join("orders")).expect("create orders directory");
            fs::write(
                path.join("orders/rules.ron"),
                ron::ser::to_string(&OrderRulesDef {
                    queue_depth: 16,
                    smart: Vec::new(),
                })
                .expect("serialize order rules"),
            )
            .expect("write order rules");
        }
        "exports.interface" => {
            manifest.exports.interface = vec!["interface/hud.ron".into()];
            fs::create_dir_all(path.join("interface")).expect("create interface directory");
            fs::write(path.join("interface/hud.ron"), EMPTY_INTERFACE)
                .expect("write empty interface");
        }
        "exports.input" => {
            manifest.exports.input = vec!["input/bindings.ron".into()];
            fs::create_dir_all(path.join("input")).expect("create input directory");
            fs::write(
                path.join("input/bindings.ron"),
                ron::ser::to_string(&InputMapDef {
                    actions: Vec::<ActionDef>::new(),
                    engine_overrides: Vec::new(),
                })
                .expect("serialize empty input map"),
            )
            .expect("write input map");
        }
        "exports.assets" => {
            manifest.exports.assets = vec!["assets/assets.ron".into()];
            fs::create_dir_all(path.join("assets")).expect("create assets directory");
            fs::write(
                path.join("assets/assets.ron"),
                ron::ser::to_string(&AssetsDef {
                    images: Vec::new(),
                    fonts: Vec::new(),
                    sounds: Vec::new(),
                    cursors: Vec::new(),
                })
                .expect("serialize empty assets"),
            )
            .expect("write assets");
        }
        _ => panic!("unknown version table row {row}"),
    }
    write_manifest(&path, &manifest);
    path
}

#[test]
fn each_version_row_refuses_0_2_2() {
    for row in [
        "exports.orders",
        "exports.interface",
        "exports.input",
        "exports.assets",
    ] {
        let path = version_row_fixture(row);
        let output = Command::new(env!("CARGO_BIN_EXE_turts-pack"))
            .arg(&path)
            .output()
            .expect("run package validator");
        assert_eq!(output.status.code(), Some(2), "{row}: {output:?}");
        assert!(output.stdout.is_empty(), "{row}: {output:?}");
        let stderr = String::from_utf8(output.stderr).expect("validator output is UTF-8");
        assert!(
            stderr.contains(row),
            "expected {row} in error, got {stderr}"
        );
        fs::remove_dir_all(path).expect("remove version row fixture");
    }
}

#[test]
fn a_new_order_binds_a_behaviour_by_data() {
    let path = unique_temp_dir("data-order");
    copy_tree(&package_dir("acme"), &path);
    let mut manifest = read_manifest(&path);
    manifest.engine_api = "^0.2.3".into();
    manifest.exports.orders.push("orders/dash.ron".into());
    add_data_order_fixture_inputs(&path, &mut manifest);
    write_manifest(&path, &manifest);
    fs::create_dir_all(path.join("orders")).expect("create orders directory");
    fs::write(
        path.join("orders/dash.ron"),
        "(id:\"dash\",name:\"Dash\",behaviour:Move,target:Point,fire:None,engage:None,icon:\"test_order_image\",hotkey:\"key_dash\",cursor:None,card:None,units:[\"acme:scout\"],description:\"Move the scout\")",
    )
    .expect("write data-defined order");

    let archive = archive_from_dir(&path);
    let catalog = load_catalog(&archive);
    let order_id = turts_mod_api::ContentId::parse("acme:dash").expect("valid order id");
    assert_eq!(
        catalog.orders.get(&order_id).map(|order| order.behaviour),
        Some(Behaviour::Move),
        "the new order binds Move through package data"
    );
    fs::remove_dir_all(path).expect("remove data-defined order fixture");
}

#[test]
fn forge_move_has_never_fire_policy() {
    let archive = archive_from_dir(&package_dir("forge"));
    let catalog = archive.content().expect("resolve Forge orders");
    let move_order = catalog
        .orders
        .get(&order_content_id("forge", "move"))
        .expect("Forge exposes its Move order");
    assert_eq!(
        move_order.fire,
        Some(FirePolicy::Never),
        "an explicit Move must not fire"
    );
}

#[test]
fn producers_have_produce_and_cancel_orders() {
    let mut violations = Vec::new();
    for package in ["base", "cedar", "haul"] {
        let archive = archive_from_dir(&package_dir(package));
        let catalog = archive
            .content()
            .unwrap_or_else(|error| panic!("resolve {package} catalog: {error}"));
        let producer_units = catalog
            .units
            .values()
            .filter(|unit| unit.producer.is_some())
            .map(|unit| unit.id.to_string())
            .collect::<BTreeSet<_>>();
        if producer_units.is_empty() {
            violations.push(format!("{package} has no producer"));
        }
        for (verb, behaviour) in [
            ("produce", Behaviour::Produce),
            ("cancel", Behaviour::CancelProduction),
        ] {
            let local_id = format!("order_{verb}");
            let content_id = order_content_id(package, verb);
            let Some(order) = catalog.orders.get(&content_id) else {
                violations.push(format!("{package} producer package lacks {local_id}"));
                continue;
            };
            assert_eq!(
                order.behaviour, behaviour,
                "{package}:{local_id} uses the approved behavior"
            );
            if !order.units.iter().any(|unit| producer_units.contains(unit)) {
                violations.push(format!(
                    "{package}:{local_id} is not available to a producer"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "producer orders cover every producer package: {violations:?}"
    );
}

#[test]
fn resource_nodes_choose_gather_before_generic_move() {
    let mut violations = Vec::new();
    for package in ["base", "cedar", "haul"] {
        let archive = archive_from_dir(&package_dir(package));
        let smart = archive
            .order_rules()
            .unwrap_or_else(|error| panic!("read {package} rules: {error}"))
            .into_iter()
            .flat_map(|(_, rules)| rules.smart)
            .collect::<Vec<_>>();
        let gather_index = smart.iter().position(|rule| {
            rule.target == SmartTarget::ResourceNodeAccepted
                && rule.needs == Capability::Gatherer
                && rule.order == order_ref(package, "gather")
        });
        let generic_move_index = smart.iter().position(|rule| {
            rule.target == SmartTarget::Node
                && rule.needs == Capability::Movement
                && rule.order == order_ref(package, "move")
        });
        match (gather_index, generic_move_index) {
            (Some(gather), Some(move_order)) if gather < move_order => {}
            (Some(_), Some(_)) => violations.push(format!(
                "{package} puts generic Node movement before accepted-node gathering"
            )),
            (None, _) => violations.push(format!("{package} has no accepted-node Gather rule")),
            (_, None) => violations.push(format!("{package} has no generic Node Move rule")),
        }
    }
    assert!(
        violations.is_empty(),
        "smart rules prioritize accepted-node gathering: {violations:?}"
    );
}

#[test]
fn forge_smart_node_default_uses_move() {
    let archive = archive_from_dir(&package_dir("forge"));
    let smart = archive
        .order_rules()
        .expect("read Forge order rules")
        .into_iter()
        .flat_map(|(_, rules)| rules.smart)
        .collect::<Vec<_>>();
    assert!(
        smart.iter().any(|rule| {
            rule.target == SmartTarget::Node
                && rule.needs == Capability::Movement
                && rule.order == order_ref("forge", "move")
        }),
        "Forge routes an unmatched node to Move for a unit with movement"
    );
}

#[test]
fn forge_toggle_orders_cover_existing_toggleable_units() {
    let archive = archive_from_dir(&package_dir("forge"));
    let catalog = archive.content().expect("resolve Forge catalog");
    let toggleable_units = catalog
        .units
        .values()
        .filter(|unit| {
            unit.extractor.is_some()
                || unit.inserter.is_some()
                || unit.machine.is_some()
                || unit.research.is_some()
                || unit.producer.is_some()
                || unit.weapon.is_some()
        })
        .map(|unit| unit.id.to_string())
        .collect::<BTreeSet<_>>();
    let mut violations = Vec::new();
    for verb in ["enable", "disable"] {
        let local_id = format!("order_{verb}");
        let content_id = order_content_id("forge", verb);
        let Some(order) = catalog.orders.get(&content_id) else {
            violations.push(format!("Forge does not export {local_id}"));
            continue;
        };
        let expected_behaviour = if verb == "enable" {
            Behaviour::SetEnabled(true)
        } else {
            Behaviour::SetEnabled(false)
        };
        if order.behaviour != expected_behaviour {
            violations.push(format!(
                "{local_id} has behavior {:?}, expected {expected_behaviour:?}",
                order.behaviour
            ));
        }
        let subjects = order.units.iter().cloned().collect::<BTreeSet<_>>();
        if subjects != toggleable_units {
            violations.push(format!(
                "{local_id} subjects {subjects:?}, expected all M3-toggleable units {toggleable_units:?}"
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "Forge toggle orders match M3 toggleability and exclude belts: {violations:?}"
    );
}

const EMPTY_INTERFACE: &str = r##"(
    id:"hud",
    root:[],
    tooltip_style:(background:"#000000",colour:"#ffffff",font:None,size:12),
    cursors:{},
    camera:(pitch:0.7,yaw:0.0,field_of_view:60.0,minimum:10.0,maximum:100.0,start:MapCentre,height:40.0,margin:0.0,pan_speed:1.0,edge_band:true),
    light:None,
    slot_colours:[],
    world:(ground:"#000000",grid:"#ffffff",selection:"#ffffff",health:"#00ff00",range:"#ffffff",show_range:true,tooltip:(background:"#000000",colour:"#ffffff",font:None,size:12),health_bar:(width:32,height:4,background:"#000000",health:"#00ff00")),
    alerts:[],
    reasons:{},
    units:{},
    selection_priority:[],
    guide:[],
)"##;
