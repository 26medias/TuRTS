//! T3 (design 7): claiming a slot creates no gameplay entity.

use turts_mod_api::{
    Footprint, MapDef, MapSize, PackageId, Placement, UnitDef, UnitRegistry, WorldPos,
};
use turts_sim::{
    ClaimError, SlotId, Slots, UnitTypes, gameplay_entity_count, headless_app, load_map, snapshot,
};

fn app_with_one_unit() -> bevy_app::App {
    let mut app = headless_app(20);
    let origin = PackageId("fixture_pkg".to_string());
    let def = UnitDef {
        id: "walker".to_string(),
        name: "Walker".to_string(),
        speed: 3.0,
        footprint: Footprint::Circle { radius: 0.5 },
    };
    app.world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(&origin, def)
        .unwrap();
    let map = MapDef {
        id: "field".to_string(),
        size: MapSize { x: 64.0, y: 64.0 },
        placements: vec![Placement {
            unit: "fixture_pkg:walker".to_string(),
            owner_slot: 0,
            pos: WorldPos { x: 8.0, y: 8.0 },
        }],
    };
    load_map(app.world_mut(), &origin, &map).unwrap();
    app
}

#[test]
fn t3_claim_creates_nothing() {
    let mut app = app_with_one_unit();
    assert_eq!(gameplay_entity_count(app.world_mut()), 1);
    app.world_mut().resource_mut::<Slots>().claim(11).unwrap(); // slot 0
    app.world_mut().resource_mut::<Slots>().claim(22).unwrap(); // slot 1
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(gameplay_entity_count(app.world_mut()), 1);
    assert_eq!(snapshot(app.world_mut()).len(), 1);
}

#[test]
fn claim_same_player_twice_is_rejected() {
    let mut slots = Slots::new(2);
    assert_eq!(slots.claim(11), Ok(SlotId(0)));
    assert_eq!(slots.claim(11), Err(ClaimError::AlreadyClaimed));
    assert_eq!(slots.slot_of(11), Some(SlotId(0)));
    assert_eq!(slots.owner_of(SlotId(0)), Some(11));
    // The rejected claim took no second slot (D25).
    assert_eq!(slots.owner_of(SlotId(1)), None);
}

#[test]
fn third_claim_has_no_open_slot() {
    let mut slots = Slots::new(2);
    assert_eq!(slots.claim(11), Ok(SlotId(0)));
    assert_eq!(slots.claim(22), Ok(SlotId(1)));
    assert_eq!(slots.claim(33), Err(ClaimError::NoOpenSlot));
    assert_eq!(slots.slot_of(33), None);
    // A known player is told "already claimed" even when no slot is open.
    assert_eq!(slots.claim(22), Err(ClaimError::AlreadyClaimed));
    assert_eq!(slots.owner_of(SlotId(5)), None);
}
