use std::path::PathBuf;

use turts_mod_api::{ContentId, Shape, VisualTarget};
use turts_mod_runtime::archive::PackageArchive;
use turts_pack::pack_checked;

fn hud4_archive() -> PackageArchive {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/muster_hud4");
    let bytes = pack_checked(&path).expect("pack and validate HUD4 fixture");
    PackageArchive::parse(&bytes).expect("parse HUD4 fixture")
}

fn cid(value: &str) -> ContentId {
    ContentId::parse(value).expect("fixture content id is valid")
}

#[test]
fn hud4_fixture_roles_match_actual_public_capabilities_and_target_relations() {
    let archive = hud4_archive();
    let catalog = archive.content().expect("validate authored public catalog");
    let attacker = &catalog.units[&cid("muster:trooper")];
    assert_eq!(
        attacker.weapon.as_ref().map(|weapon| weapon.targets.slots),
        Some(turts_mod_api::SlotRelation::Other)
    );
    assert!(attacker.gatherer.is_none());

    let worker = &catalog.units[&cid("muster:worker")];
    assert!(worker.weapon.is_none());
    assert!(
        worker
            .gatherer
            .as_ref()
            .is_some_and(|gatherer| gatherer.items.contains(&cid("muster:ore_chunk")))
    );

    let foreign_target = &catalog.units[&cid("muster:dummy")];
    assert!(foreign_target.weapon.is_none());
    let node = &catalog.nodes[&cid("muster:ore_node")];
    assert_eq!(node.item, cid("muster:ore_chunk"));

    let field = &archive.maps().expect("parse fixture map")[0].1;
    assert!(
        field
            .placements
            .iter()
            .any(|entry| entry.unit == "muster:trooper" && entry.owner_slot == 0)
    );
    assert!(
        field
            .placements
            .iter()
            .any(|entry| entry.unit == "muster:worker" && entry.owner_slot == 0)
    );
    assert!(
        field
            .placements
            .iter()
            .any(|entry| entry.unit == "muster:dummy" && entry.owner_slot == 1)
    );
    assert_eq!(field.nodes.len(), 1);
}

#[test]
fn hud4_fixture_uses_visible_shape_signatures_for_pixel_roles() {
    let visuals = hud4_archive().visuals().expect("parse fixture visuals");
    let shape_for = |target: VisualTarget| {
        visuals
            .iter()
            .find(|(_, visual)| visual.target == target)
            .map(|(_, visual)| visual.shape.clone())
    };
    assert!(
        matches!(shape_for(VisualTarget::Unit("muster:trooper".into())), Some(Shape::Capsule { height, .. }) if height >= 2.0)
    );
    assert!(
        matches!(shape_for(VisualTarget::Unit("muster:worker".into())), Some(Shape::Cuboid { x, .. }) if x < 1.0)
    );
    assert!(
        matches!(shape_for(VisualTarget::Unit("muster:dummy".into())), Some(Shape::Capsule { radius, .. }) if radius >= 0.8)
    );
    assert!(
        matches!(shape_for(VisualTarget::Node("muster:ore_node".into())), Some(Shape::Cuboid { x, .. }) if x >= 3.0)
    );
}
