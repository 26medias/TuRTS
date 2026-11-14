//! Presentation data must not change admission, movement, harvesting, or the audited world.
use bevy_math::Vec2;
use std::{collections::BTreeMap, path::Path};
use turts_mod_api::{Antialiasing, DistanceFogDef, EnvironmentDef, MapDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};
use turts_sim::{Command, CommandKind, CommandQueue, SlotId, Slots, audit_state, headless_app};

fn files() -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let mut files = BTreeMap::new();
    walk(&root, &root, &mut files);
    files
}

#[test]
fn environment_does_not_change_a_moving_and_harvesting_match() {
    let mut files = files();
    let text = String::from_utf8(files["manifest.ron"].clone()).unwrap();
    let manifest: turts_mod_api::Manifest = ron::from_str(&text).unwrap();
    files.insert(
        "manifest.ron".into(),
        text.replace(&manifest.engine_api, "^0.2.10").into_bytes(),
    );
    let mut map: MapDef =
        ron::from_str(std::str::from_utf8(&files["maps/field.ron"]).unwrap()).unwrap();
    map.ground.as_mut().unwrap().overlay = None;
    let mut worlds = Vec::new();
    for enabled in [false, true] {
        map.environment = enabled.then(|| EnvironmentDef {
            background: "#B8D4DF".into(),
            ground_margin: 64.0,
            boundary_colour: Some("#536A46".into()),
            fog: Some(DistanceFogDef {
                colour: "#B8D4DF".into(),
                start: 80.0,
                end: 220.0,
                opacity: 0.2,
            }),
            antialiasing: Antialiasing::Fxaa,
        });
        files.insert(
            "maps/field.ron".into(),
            ron::ser::to_string(&map).unwrap().into_bytes(),
        );
        let package = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
        let mut app = headless_app(20);
        turts_pack::load_package(app.world_mut(), &package).unwrap();
        app.world_mut().resource_mut::<Slots>().claim(42).unwrap();
        app.world_mut().resource_mut::<Slots>().claim(43).unwrap();
        app.world_mut().resource_mut::<CommandQueue>().0.extend([
            Command {
                slot: SlotId(0),
                seq: 0,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![3],
                    target: Vec2::new(18.0, 30.0),
                },
            },
            Command {
                slot: SlotId(0),
                seq: 1,
                kind: CommandKind::Gather {
                    unit_id: 2,
                    node_id: turts_sim::NodeId(1),
                },
            },
        ]);
        worlds.push(app);
    }
    assert_eq!(
        audit_state(worlds[0].world_mut()),
        audit_state(worlds[1].world_mut())
    );
    for _ in 0..2000 {
        for app in &mut worlds {
            app.update();
        }
    }
    let first = audit_state(worlds[0].world_mut());
    let second = audit_state(worlds[1].world_mut());
    assert_eq!(first, second);
    let view = turts_sim::completed_world(worlds[0].world());
    assert!(
        view.nodes.iter().any(|node| node.remaining < 2000),
        "harvesting must actually run"
    );
    let moved = view.units.iter().find(|unit| unit.unit_id == 3).unwrap();
    assert!(
        moved.pos.distance(Vec2::new(18.0, 30.0)) < 0.2,
        "movement must actually run"
    );
}
