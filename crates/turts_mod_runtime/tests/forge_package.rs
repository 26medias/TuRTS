//! Portable packaging and E1 coverage for the M3 forge content package.

use std::path::PathBuf;

use turts_mod_api::{ContentId, ContentsPolicy, Footprint, RefundRatio, RemovalDef};
use turts_mod_runtime::{
    archive::PackageArchive, hash::sha256_hex, pack::pack_dir, visuals::visuals_by_content,
};

fn forge_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge")
}

#[test]
fn forge_packs_and_validates() {
    let bytes = pack_dir(&forge_dir()).expect("forge must pack");
    let archive = PackageArchive::parse(&bytes).expect("forge archive must parse");
    assert_eq!(archive.manifest.id, "forge");
    assert_eq!(archive.manifest.version, "0.1.0");
    assert_eq!(archive.manifest.engine_api, "^0.2.1");
    let catalog = archive.content().expect("forge content must validate");
    assert_eq!(catalog.resources.len(), 2);
    assert_eq!(catalog.units.len(), 7);
    assert_eq!(catalog.nodes.len(), 1);
    assert_eq!(catalog.recipes.len(), 1);
    assert_eq!(catalog.technologies.len(), 1);
    let visuals =
        visuals_by_content(std::slice::from_ref(&archive)).expect("client visuals must validate");
    assert_eq!(visuals.len(), 8);
    let maps = archive.maps().expect("map definitions must parse");
    assert_eq!(maps.len(), 1);
    assert_eq!(maps[0].1.id, "proving_ground");
}

#[test]
fn forge_archive_hash_is_stable_across_two_packs() {
    let first = pack_dir(&forge_dir()).expect("first forge pack");
    let second = pack_dir(&forge_dir()).expect("second forge pack");
    assert_eq!(first, second);
    assert_eq!(sha256_hex(&first), sha256_hex(&second));
}

#[test]
fn forge_locked_fixture_resolves_its_prerequisite_chain() {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/m3/packages/forge_locked");
    let bytes = pack_dir(&dir).expect("forge_locked must pack");
    let archive = PackageArchive::parse(&bytes).expect("fixture archive must parse");
    assert_eq!(archive.manifest.id, "forge_locked");
    let catalog = archive
        .content()
        .expect("fixture prerequisite chain must validate");
    assert_eq!(catalog.technologies.len(), 2);
    assert_eq!(
        catalog.technologies[&ContentId::parse("forge_locked:precision_shells").unwrap()]
            .prerequisites,
        vec![ContentId::parse("forge_locked:hardened_shells").unwrap()]
    );
}

#[test]
fn forge_keeps_spec_placeholders_and_map_order() {
    let archive = PackageArchive::parse(&pack_dir(&forge_dir()).unwrap()).unwrap();
    let units = archive.units().unwrap();
    let unit = |id: &str| -> &turts_mod_api::UnitDef {
        &units
            .iter()
            .find(|(_, definition)| definition.id == id)
            .unwrap()
            .1
    };
    let drill = unit("drill");
    assert_eq!(drill.footprint, Footprint::Circle { radius: 1.2 });
    assert_eq!(drill.extractor.as_ref().unwrap().per_second, 2);
    assert_eq!(drill.extractor.as_ref().unwrap().interaction_range, 0.5);
    assert_eq!(drill.inventory.as_ref().unwrap().capacity, 4);
    assert!(drill.removal.is_none());

    let belt = unit("belt");
    assert_eq!(belt.footprint, Footprint::Circle { radius: 0.5 });
    assert_eq!(belt.conveyor.as_ref().unwrap().slots_per_piece, 4);
    assert_eq!(belt.conveyor.as_ref().unwrap().slots_per_second, 4);
    assert_eq!(
        belt.removal,
        Some(RemovalDef {
            by_command: true,
            contents: ContentsPolicy::Destroy
        })
    );

    let inserter = unit("arm_loader");
    assert_eq!(inserter.footprint, Footprint::Circle { radius: 0.4 });
    let arm = inserter.inserter.as_ref().unwrap();
    assert_eq!((arm.reach, arm.swing_ms, arm.return_ms), (1.0, 250, 250));
    let hand = inserter
        .inventory
        .as_ref()
        .expect("inserter hand inventory");
    assert_eq!(hand.capacity, 1);
    assert_eq!(
        hand.accepts,
        vec!["forge:ore".to_string(), "forge:shell".to_string()]
    );
    assert_eq!(
        inserter.removal,
        Some(RemovalDef {
            by_command: true,
            contents: ContentsPolicy::Destroy
        })
    );

    let press = unit("press");
    assert_eq!(press.inventory.as_ref().unwrap().capacity, 6);
    assert_eq!(press.output_inventory.as_ref().unwrap().capacity, 4);
    assert!(press.removal.is_none());
    let recipe = archive.recipes().unwrap().pop().unwrap().1;
    assert_eq!(recipe.inputs[0].amount, 2);
    assert_eq!(recipe.outputs[0].amount, 1);
    assert_eq!(recipe.duration_ms, 2000);
    assert_eq!(
        recipe.cancel_refunds.running,
        RefundRatio {
            numerator: 1,
            denominator: 1
        }
    );
    assert_eq!(
        recipe.cancel_refunds.output_blocked,
        RefundRatio {
            numerator: 1,
            denominator: 1
        }
    );
    let technology = archive.technologies().unwrap().pop().unwrap().1;
    assert_eq!(technology.duration_ms, 10000);
    assert_eq!(technology.costs[0].amount, 10);
    assert_eq!(
        technology.payment,
        turts_mod_api::PaymentPolicy::ReserveThenCharge
    );
    assert_eq!(
        technology.cancel_refunds.queued,
        RefundRatio {
            numerator: 1,
            denominator: 1
        }
    );
    assert_eq!(
        technology.cancel_refunds.running,
        RefundRatio {
            numerator: 1,
            denominator: 2
        }
    );
    assert!(matches!(
        technology.effects.as_slice(),
        [turts_mod_api::EffectDef::WeaponModifier { add: 10, .. }]
    ));

    let turret = unit("turret");
    let weapon = turret.weapon.as_ref().unwrap();
    assert_eq!(
        (weapon.range, weapon.cooldown_ms, weapon.damage),
        (8.0, 1000, 10)
    );
    assert_eq!(weapon.ammo.as_ref().unwrap().per_shot, 1);
    assert!(turret.removal.is_none());
    let lab = unit("lab");
    assert_eq!(lab.inventory.as_ref().unwrap().capacity, 10);
    assert!(lab.removal.is_none());
    let drone = unit("drone");
    assert_eq!(drone.movement.as_ref().unwrap().speed, 2.0);
    assert_eq!(drone.health.unwrap().max, 100);
    assert!(!drone.removal.unwrap().by_command);

    let map = archive.maps().unwrap().pop().unwrap().1;
    assert_eq!((map.size.x, map.size.y), (32.0, 16.0));
    let expected = [
        "forge:drill",
        "forge:arm_loader",
        "forge:arm_loader",
        "forge:press",
        "forge:arm_loader",
        "forge:arm_loader",
        "forge:turret",
        "forge:lab",
        "forge:drone",
        "forge:drone",
    ];
    assert_eq!(
        map.placements
            .iter()
            .map(|p| p.unit.as_str())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(map.placements[0].pos.x, 6.0);
    assert_eq!(map.placements[1].pos.x, 8.0);
    assert_eq!(map.placements[2].pos.x, 15.0);
    assert_eq!(map.placements[3].pos.x, 17.0);
    assert_eq!(map.placements[4].pos.y, 10.0);
    assert_eq!(map.placements[5].pos.y, 6.0);
    assert_eq!(map.placements[6].pos.y, 12.0);
    assert_eq!(map.placements[7].pos.y, 4.0);
    assert_eq!(map.placements[8].pos.x, 24.0);
    assert_eq!(map.placements[9].pos.x, 30.0);
    assert_eq!(map.nodes[0].remaining, 300);
    assert_eq!((map.nodes[0].pos.x, map.nodes[0].pos.y), (3.5, 8.0));
    assert_eq!(map.belts.len(), 1);
    assert_eq!(map.belts[0].pieces, 6);
    assert_eq!((map.belts[0].start.x, map.belts[0].start.y), (9.0, 8.0));
}
