use std::path::PathBuf;

use turts_mod_api::{
    AlertTrigger, Anchor, CameraStart, ContentsPolicy, Dimension, Footprint, Predicate, Primitive,
    ResourceKind, SlotRelation, Source, TextValue, UnitDef, Value,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{UnitTypes, audit_state, headless_app};

fn package_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}"))
}

fn archive_at(path: &std::path::Path) -> PackageArchive {
    PackageArchive::parse(&pack_dir(path).expect("pack package directory"))
        .expect("parse package archive")
}

fn unit<'a>(definitions: &'a [(String, UnitDef)], id: &str) -> &'a UnitDef {
    definitions
        .iter()
        .find(|(_, definition)| definition.id == id)
        .map(|(_, definition)| definition)
        .unwrap_or_else(|| panic!("muster exports unit `{id}`"))
}

#[test]
fn muster_loads_with_515_units() {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let archive = archive_at(&package);
    let mut app = headless_app(20);

    load_package(app.world_mut(), &archive).expect("load muster through the package loader");

    let definitions = archive.units().expect("parse muster unit definitions");
    let trooper = unit(&definitions, "trooper");
    assert!(matches!(
        &trooper.footprint,
        Footprint::Circle { radius } if *radius == 0.5
    ));
    assert_eq!(trooper.movement.as_ref().unwrap().speed, 3.0);
    assert_eq!(trooper.health.unwrap().max, 50);
    let weapon = trooper.weapon.as_ref().unwrap();
    assert_eq!(weapon.range, 4.0);
    assert_eq!(weapon.damage, 5);
    assert_eq!(weapon.cooldown_ms, 1000);
    assert_eq!(weapon.targets.slots, SlotRelation::Other);
    assert_eq!(trooper.removal.unwrap().contents, ContentsPolicy::Destroy);

    let beacon = unit(&definitions, "beacon");
    assert!(beacon.movement.is_some());
    assert!(beacon.weapon.is_none());
    let dummy = unit(&definitions, "dummy");
    assert!(dummy.movement.is_some());
    assert_eq!(dummy.health.unwrap().max, 40);
    assert!(dummy.weapon.is_none());
    let post = unit(&definitions, "post");
    assert!(post.movement.is_none());
    assert!(matches!(
        &post.footprint,
        Footprint::Circle { radius } if *radius == 2.0
    ));
    assert!(unit(&definitions, "worker").gatherer.is_some());
    assert!(unit(&definitions, "depot").drop_off.is_some());

    let (_, map) = archive
        .maps()
        .expect("parse muster map")
        .into_iter()
        .next()
        .expect("muster exports one map");
    assert_eq!((map.size.x, map.size.y), (64.0, 48.0));
    assert_eq!(map.nodes.len(), 1);
    assert_eq!(map.nodes[0].node, "muster:ore_node");
    assert!(
        map.slot_economy
            .iter()
            .find(|slot| slot.slot == 0)
            .is_some_and(|slot| slot
                .balances
                .iter()
                .any(|balance| { balance.balance == "muster:ore" && balance.amount > 0 }))
    );
    assert!(
        archive
            .resources()
            .expect("parse muster resources")
            .iter()
            .any(|(_, resource)| resource.id == "ore" && resource.kind == ResourceKind::Balance)
    );

    let (_, hud) = archive
        .interfaces()
        .expect("parse muster interface")
        .into_iter()
        .next()
        .expect("muster exports one interface");
    assert!(
        hud.alerts
            .iter()
            .any(|alert| { alert.trigger == AlertTrigger::OwnUnitDamaged && alert.minimap_ping })
    );

    let state = audit_state(app.world_mut());
    assert_eq!(state.units.len(), 515, "muster's map contains 515 units");
    let counts = state.units.iter().fold(
        std::collections::BTreeMap::<(u8, String), usize>::new(),
        |mut counts, unit| {
            *counts
                .entry((unit.owner_slot, unit.content_id.clone()))
                .or_default() += 1;
            counts
        },
    );
    assert_eq!(counts.get(&(0, "muster:trooper".to_owned())), Some(&40));
    assert_eq!(counts.get(&(0, "muster:beacon".to_owned())), Some(&460));
    assert_eq!(counts.get(&(0, "muster:worker".to_owned())), Some(&2));
    assert_eq!(counts.get(&(0, "muster:depot".to_owned())), Some(&1));
    assert_eq!(counts.get(&(0, "muster:post".to_owned())), Some(&1));
    assert_eq!(counts.get(&(1, "muster:dummy".to_owned())), Some(&10));
    assert_eq!(counts.get(&(1, "muster:post".to_owned())), Some(&1));
}

#[test]
fn fixture_hud1_loads() {
    let archive = archive_at(&package_dir("muster_hud1"));
    let mut app = headless_app(20);

    load_package(app.world_mut(), &archive).expect("load the HUD1 fixture package");
    let interfaces = archive.interfaces().expect("parse fixture interfaces");

    assert_eq!(interfaces.len(), 1, "HUD1 fixture exports its HUD");
    let (_, hud) = &interfaces[0];
    assert_eq!(hud.id, "hud");
    assert_eq!(hud.root.len(), 3, "HUD1 uses the three D8.5 root widgets");

    let card = &hud.root[0];
    assert_eq!(card.id, "card");
    assert_eq!(card.at.unwrap().anchor, Anchor::TopLeft);
    assert_eq!(card.at.unwrap().offset, (8.0, 8.0));
    assert_eq!(card.size.w, Dimension::Px(272.0));
    assert_eq!(card.size.h, Dimension::Px(204.0));
    assert_eq!(
        card.visible,
        [Predicate::SelectionNotEmpty, Predicate::PrimaryOwned]
    );
    assert!(matches!(
        &card.widget,
        Primitive::Grid {
            source: Source::CommandCard,
            columns: 4,
            rows: 3,
            cell: (64, 64),
            gap: 4,
            ..
        }
    ));
    let Primitive::Grid { item, .. } = &card.widget else {
        panic!("HUD1 card is a command-card grid");
    };
    assert_eq!(item.id, "b");
    assert_eq!(item.size.w, Dimension::Fill);
    assert_eq!(item.size.h, Dimension::Fill);
    assert!(matches!(
        &item.widget,
        Primitive::Button {
            icon: Some(turts_mod_api::ButtonIcon::Item(
                turts_mod_api::IconField::Icon
            )),
            action: turts_mod_api::ButtonAction::Item(turts_mod_api::ActionField::Action),
            hotkey_label: true,
            ..
        }
    ));
    assert!(card.tooltip.is_some());

    let minimap = &hud.root[1];
    assert_eq!(minimap.id, "minimap");
    assert_eq!(minimap.at.unwrap().anchor, Anchor::BottomRight);
    assert_eq!(minimap.at.unwrap().offset, (-8.0, -8.0));
    assert_eq!(minimap.size.w, Dimension::Px(200.0));
    assert_eq!(minimap.size.h, Dimension::Px(200.0));
    assert!(matches!(
        &minimap.widget,
        Primitive::MinimapView { frame, .. } if frame == "#ffffff"
    ));

    let ore = &hud.root[2];
    assert_eq!(ore.id, "ore");
    assert_eq!(ore.at.unwrap().anchor, Anchor::Bottom);
    assert_eq!(ore.at.unwrap().offset, (0.0, -4.0));
    assert_eq!(ore.size.w, Dimension::Px(160.0));
    assert_eq!(ore.size.h, Dimension::Px(28.0));
    assert!(matches!(
        &ore.widget,
        Primitive::Text {
            text: TextValue::Format(pattern, args),
            size: 18,
            ..
        } if pattern == "Ore {0}" && args == &[Value::Balance("muster:ore".to_owned())]
    ));
}

#[test]
fn muster_camera_starts_on_troopers_with_post_in_view() {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let archive = archive_at(&package);
    let (_, interface) = archive
        .interfaces()
        .expect("parse muster interface")
        .into_iter()
        .next()
        .expect("muster exports one interface");
    let (_, map) = archive
        .maps()
        .expect("parse muster map")
        .into_iter()
        .next()
        .expect("muster exports one map");
    let (start_x, start_y) = match interface.camera.start {
        CameraStart::Point(x, y) => (x, y),
        start => panic!("muster camera must start on the troopers, got {start:?}"),
    };
    let troopers: Vec<_> = map
        .placements
        .iter()
        .filter(|placement| placement.unit == "muster:trooper" && placement.owner_slot == 0)
        .collect();
    assert_eq!(troopers.len(), 40);
    let troop_centre = (
        troopers.iter().map(|unit| unit.pos.x).sum::<f32>() / troopers.len() as f32,
        troopers.iter().map(|unit| unit.pos.y).sum::<f32>() / troopers.len() as f32,
    );
    assert!((start_x - troop_centre.0).abs() <= 0.001);
    assert!((start_y - troop_centre.1).abs() <= 0.001);

    // The FOV and height define the nominal ground-plane rectangle for a 1280 x 720 view.
    let half_height =
        interface.camera.height * (interface.camera.field_of_view.to_radians() * 0.5).tan();
    let half_width = half_height * (1280.0 / 720.0);
    let post = map
        .placements
        .iter()
        .find(|placement| placement.unit == "muster:post" && placement.owner_slot == 0)
        .expect("slot 0 has its post");
    for position in troopers
        .iter()
        .map(|unit| &unit.pos)
        .chain(std::iter::once(&post.pos))
    {
        assert!((position.x - start_x).abs() <= half_width);
        assert!((position.y - start_y).abs() <= half_height);
    }
}

#[test]
fn troop_and_beacon_centres_fit_one_screen_box_without_other_units() {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let archive = archive_at(&package);
    let (_, interface) = archive
        .interfaces()
        .expect("parse muster interface")
        .into_iter()
        .next()
        .expect("muster exports one interface");
    let (_, map) = archive
        .maps()
        .expect("parse muster map")
        .into_iter()
        .next()
        .expect("muster exports one map");
    let selected: Vec<_> = map
        .placements
        .iter()
        .filter(|placement| {
            placement.owner_slot == 0
                && matches!(placement.unit.as_str(), "muster:trooper" | "muster:beacon")
        })
        .collect();
    assert_eq!(selected.len(), 500);

    let min_x = selected
        .iter()
        .map(|unit| unit.pos.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = selected
        .iter()
        .map(|unit| unit.pos.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = selected
        .iter()
        .map(|unit| unit.pos.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = selected
        .iter()
        .map(|unit| unit.pos.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let box_width = max_x - min_x;
    let box_height = max_y - min_y;
    assert!(box_width / box_height <= 1280.0 / 720.0);

    let CameraStart::Point(camera_x, camera_y) = interface.camera.start else {
        panic!("muster camera must start on the troopers");
    };
    let half_height =
        interface.camera.height * (interface.camera.field_of_view.to_radians() * 0.5).tan();
    let half_width = half_height * (1280.0 / 720.0);
    assert!(min_x >= camera_x - half_width && max_x <= camera_x + half_width);
    assert!(min_y >= camera_y - half_height && max_y <= camera_y + half_height);

    let others_inside = map
        .placements
        .iter()
        .filter(|unit| {
            !(unit.owner_slot == 0
                && matches!(unit.unit.as_str(), "muster:trooper" | "muster:beacon"))
        })
        .filter(|unit| {
            unit.pos.x >= min_x && unit.pos.x <= max_x && unit.pos.y >= min_y && unit.pos.y <= max_y
        })
        .map(|unit| unit.unit.as_str())
        .collect::<Vec<_>>();
    assert!(
        others_inside.is_empty(),
        "selection box contains {others_inside:?}"
    );
}

#[test]
fn beacon_is_the_first_mobile_unit_type() {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let archive = archive_at(&package);
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).expect("load muster before inspecting registration");
    let unit_types = app.world().resource::<UnitTypes>();
    let first_mobile = unit_types
        .entries()
        .iter()
        .find(|unit| unit.movement.is_some())
        .expect("muster has a mobile unit type");

    assert_eq!(first_mobile.content_id, "muster:beacon");
}
