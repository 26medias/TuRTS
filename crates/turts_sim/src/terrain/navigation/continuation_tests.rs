#![allow(
    clippy::excessive_precision,
    reason = "Captures retain their original decimal form for reproducible geometry."
)]
//! Real Orders-phase behavior for a local terrain refusal and terminal controls.
use super::*;
use crate::{
    Owner, Position, SlotId, UnitId, UnitType,
    movement::{MovementActivity, PlainMoveProgress},
    orders::{Order, OrderTargetSim, UnitOrders},
};
#[test]
fn continued_terrain_order_keeps_local_plain_block_recoverable() {
    for mode in 0..7 {
        let mut app = crate::headless_app(20);
        let endpoint = Vec2::new(6., 4.);
        let order = Order {
            order: turts_mod_api::ContentId::parse("test:move").unwrap(),
            behaviour: turts_mod_api::Behaviour::Move,
            target: OrderTargetSim::Point(endpoint),
            group: 0,
        };
        let entity = app
            .world_mut()
            .spawn((
                UnitId(1),
                UnitType(turts_mod_api::UnitTypeId(1)),
                Owner(SlotId(0)),
                Position(Vec2::ONE),
                crate::MoveTarget(endpoint),
                UnitOrders::default(),
                PlainMoveProgress {
                    arrival_yield: false,
                    original_point: endpoint,
                    slot: endpoint,
                    pool_slots: vec![],
                    gather_goal: None,
                    activity: MovementActivity::Blocked,
                    stalled_ticks: 20,
                    best_distance: 3.,
                    claim_count: 3,
                    failure_recorded: false,
                },
            ))
            .id();
        app.world_mut()
            .insert_resource(crate::orders::PlainMoveOrderIdentities(BTreeMap::from([(
                1,
                order.clone(),
            )])));
        let mut navigation = TerrainNavigation::default();
        let mut member = Member::stamp(0, 0, 1, 0);
        member.cancelled = false;
        member.live_target = true;
        match mode {
            1 => member.cancelled = true,
            2 => member.live_target = false,
            3 => member.owner_fence = true,
            4 => member.owner_released = true,
            5 => {} // Wrong viewer owns this member.
            6 => {} // Flat control has no navigation resource.
            _ => {}
        }
        navigation.viewers[usize::from(mode == 5)]
            .members
            .insert(1, member);
        if mode != 6 {
            app.world_mut().insert_resource(navigation);
        }
        crate::orders::run_orders(app.world_mut());
        let orders = app.world().get::<UnitOrders>(entity).unwrap();
        if mode == 0 {
            assert!(orders.current.is_none());
            assert_eq!(
                app.world()
                    .resource::<crate::orders::PlainMoveOrderIdentities>()
                    .0
                    .get(&1),
                Some(&order)
            );
            assert!(app.world().get::<crate::MoveTarget>(entity).is_some());
            assert!(orders.last_failure.is_none());
            assert!(
                !app.world()
                    .get::<PlainMoveProgress>(entity)
                    .unwrap()
                    .failure_recorded
            );
        } else {
            assert!(orders.current.is_none(), "terminal control {mode}");
            assert_eq!(
                orders.last_failure.as_ref().unwrap().reason,
                crate::RejectReason::Blocked
            );
            assert!(
                app.world()
                    .get::<PlainMoveProgress>(entity)
                    .unwrap()
                    .failure_recorded
            );
        }
    }
}

#[test]
fn captured_plain_blocked_escape_retains_only_current_move_parking_authority() {
    let target = Vec2::new(53.45000076293945, 25.991857528686523);
    let goal = Vec2::new(50., 24.);
    let mut world = World::new();
    let entity = world
        .spawn((
            UnitId(28),
            Owner(SlotId(0)),
            UnitOrders {
                current: Some(Order {
                    order: turts_mod_api::ContentId::parse("quarry:order_move").unwrap(),
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: OrderTargetSim::Point(goal),
                    group: 1,
                }),
                ..Default::default()
            },
        ))
        .id();
    let unit = MovementFrameUnit {
        unit_id: 28,
        owner_slot: 0,
        unit_type: 1,
        position: Vec2::new(47.36805725097656, 25.462810516357422),
        radius: 1.1,
        speed: Some(3.5),
        target: Some(target),
        progress: None,
        plain_progress: Some(PlainMoveProgress {
            arrival_yield: false,
            original_point: target,
            slot: target,
            pool_slots: vec![],
            gather_goal: None,
            activity: MovementActivity::Blocked,
            stalled_ticks: 1013,
            best_distance: 6.104910373687744,
            claim_count: 0,
            failure_recorded: false,
        }),
        physical_escape: Some(crate::movement::PhysicalEscape {
            target: Some(Vec2::new(47.49305725097656, 25.462810516357422)),
        }),
    };
    for mode in 0..5 {
        let mut navigation = TerrainNavigation::default();
        let mut member = Member::stamp(0, 0, 28, 0);
        member.cancelled = mode == 1;
        member.owner_fence = mode == 2;
        member.owner_released = mode == 3;
        member.live_target = mode != 4;
        navigation.viewers[0].members.insert(28, member);
        assert_eq!(
            super::super::parking_public_goal(&world, &unit, &navigation),
            (mode == 0).then_some(goal),
            "authority mode {mode}"
        );
    }
    let mut navigation = TerrainNavigation::default();
    let mut member = Member::stamp(0, 0, 28, 0);
    member.cancelled = false;
    member.live_target = true;
    navigation.viewers[0].members.insert(28, member);
    world.get_mut::<Owner>(entity).unwrap().0 = SlotId(1);
    assert_eq!(
        super::super::parking_public_goal(&world, &unit, &navigation),
        None
    );
}

#[test]
fn destination_scope_requires_original_epoch_goal_owner_and_shape() {
    let mut n = TerrainNavigation::default();
    let profile = Traversal {
        max_step: 0,
        max_grade: turts_rules::terrain::Ratio {
            numerator: 0,
            denominator: 1,
        },
    };
    let goal = Vec2::new(40., 10.);
    let region = DestinationRegion {
        goal,
        min: goal,
        max: goal,
        cohort_radius: 1.1,
    };
    let mut unit = MovementFrameUnit {
        unit_id: 1,
        owner_slot: 0,
        unit_type: 1,
        position: Vec2::new(10., 10.),
        radius: 1.1,
        speed: Some(3.5),
        target: Some(Vec2::new(40., 11.)),
        progress: None,
        plain_progress: None,
        physical_escape: None,
    };
    assert_eq!(
        n.parking_region(&unit, goal, profile),
        None,
        "missing initial provenance never reconstructs scope"
    );
    n.record_dispatch_scopes(0, region, &[(1, 1, 1.1, profile)], 1);
    let member = n.viewers[0].members.get_mut(&1).unwrap();
    member.cancelled = false;
    member.live_target = true;
    let before_bytes = n.viewers[0].resident_bytes();
    assert_eq!(n.parking_region(&unit, goal, profile), Some(region));
    unit.unit_type = 2;
    assert_eq!(n.parking_region(&unit, goal, profile), None);
    unit.unit_type = 1;
    unit.radius = 0.4;
    assert_eq!(n.parking_region(&unit, goal, profile), None);
    unit.radius = 1.1;
    unit.owner_slot = 1;
    assert_eq!(n.parking_region(&unit, goal, profile), None);
    unit.owner_slot = 0;
    assert_eq!(n.parking_region(&unit, goal + Vec2::X, profile), None);
    let mut changed_profile = profile;
    changed_profile.max_step = 1;
    assert_eq!(n.parking_region(&unit, goal, changed_profile), None);
    n.viewers[0].members.get_mut(&1).unwrap().epoch += 1;
    assert_eq!(n.parking_region(&unit, goal, profile), None);
    n.viewers[0].cancel(1, 2, Some(0));
    assert!(n.viewers[0].members[&1].parking_scope.is_none());
    assert_eq!(n.parking_region(&unit, goal, profile), None);
    assert_eq!(
        n.viewers[0].resident_bytes(),
        before_bytes,
        "scope lives inside already charged member index storage"
    );
}

#[test]
fn cohort_scope_admission_refusal_is_atomic_under_handle_and_enumeration_reversal() {
    let profile = Traversal {
        max_step: 0,
        max_grade: turts_rules::terrain::Ratio {
            numerator: 0,
            denominator: 1,
        },
    };
    let region = DestinationRegion {
        goal: Vec2::splat(4.),
        min: Vec2::splat(4.),
        max: Vec2::splat(4.),
        cohort_radius: 1.1,
    };
    // Build a real immutable terrain phase. The dense live cache rows below
    // are a capacity-state probe; no overlapping body movement is committed.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/quarry");
    let archive = turts_mod_runtime::archive::PackageArchive::parse(
        &turts_mod_runtime::pack::pack_dir(&path).unwrap(),
    )
    .unwrap();
    let catalog = archive.content().unwrap();
    let (file, map) = archive.maps().unwrap().remove(0);
    let resolved = turts_mod_runtime::terrain::validate_terrain_map(
        &turts_mod_api::PackageId("quarry".into()),
        &file,
        &map,
        &catalog,
    )
    .unwrap();
    let mut app = crate::headless_app(20);
    let staged = crate::stage_package_with_terrain(
        app.world(),
        catalog,
        turts_mod_api::SourceDef { file, value: map },
        20,
        turts_mod_api::ContentId::parse("quarry:field").unwrap(),
        resolved,
        Some(crate::terrain::PrivateTerrainSeed::new(1)),
    )
    .unwrap();
    crate::install_package(app.world_mut(), staged);
    app.world_mut().resource_mut::<crate::Tick>().0 = 2;
    let mut units = Vec::new();
    for i in 0..MAX_MEMBERS - 1 {
        let id = 10000 + i as u32;
        app.world_mut().spawn((
            UnitId(id),
            Owner(SlotId(0)),
            UnitType(turts_mod_api::UnitTypeId(1)),
            Position(Vec2::new(2., 4.)),
            UnitOrders {
                current: Some(Order {
                    order: turts_mod_api::ContentId::parse("quarry:order_move").unwrap(),
                    behaviour: turts_mod_api::Behaviour::Move,
                    target: OrderTargetSim::Point(region.goal),
                    group: 0,
                }),
                ..Default::default()
            },
        ));
        units.push(MovementFrameUnit {
            unit_id: id,
            owner_slot: 0,
            unit_type: 1,
            position: Vec2::new(2., 4.),
            radius: 1.1,
            speed: Some(3.5),
            target: Some(region.goal),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        });
    }
    crate::terrain::phase::capture_phase(app.world_mut(), turts_mod_api::ObservationPhase::Start);
    let phase = crate::terrain::phase(app.world(), turts_mod_api::ObservationPhase::Start).unwrap();
    let frame = MovementFrame {
        bounds: Vec2::splat(64.),
        tick_hz: 20,
        units,
        pools: BTreeMap::new(),
    };
    let mut expected = None;
    for reverse in [false, true] {
        let mut n = TerrainNavigation::default();
        let book = &mut n.viewers[0];
        for i in 0..MAX_MEMBERS - 1 {
            let id = 10000 + i as u32;
            book.cancel(id, 1, Some(0));
            let member = book.members.get_mut(&id).unwrap();
            member.cancelled = false;
            member.live_target = true;
            member.endpoint = region.goal;
            member.radius = 1.1f32.to_bits();
            member.profile = profile;
            member.parking_scope = Some(ParkingScope {
                region,
                epoch: member.epoch,
                radius: 1.1f32.to_bits(),
                kind: 1,
                profile,
            });
        }
        assert_eq!(book.members.len(), MAX_MEMBERS - 1);
        let old_slots = book.member_ids.clone();
        let mut records = vec![
            (1, 1, 1.1, profile),
            (2, 1, 1.1, profile),
            (3, 1, 1.1, profile),
        ];
        if reverse {
            records.reverse();
            for row in &mut records {
                row.0 = 100 - row.0;
            }
        }
        n.record_dispatch_scopes(0, region, &records, 2);
        let book = &n.viewers[0];
        assert_eq!(book.members.len(), MAX_MEMBERS - 1);
        assert_eq!(
            book.member_ids, old_slots,
            "rollback cannot perturb old member slots"
        );
        for (id, _, _, _) in records {
            assert!(!book.members.contains_key(&id));
        }
        assert!(book.members.values().all(|m| m.parking_scope.is_some()));
        let mut candidate = frame.clone();
        let mut waiting = BTreeSet::new();
        n.viewers[0].propose(
            app.world(),
            &frame,
            &phase,
            &vec![Some(profile); frame.units.len()],
            &mut candidate,
            &mut waiting,
            &BTreeSet::new(),
        );
        let book = &n.viewers[0];
        assert!(
            book.members.values().all(|m| m.parking_scope.is_some()),
            "refused optional cohort cannot erase unrelated live scope at next propose"
        );
        let observed = (
            book.members.len(),
            book.diagnostics.invalidation_entries_this_tick,
            book.resident_bytes(),
        );
        if let Some(expected) = expected {
            assert_eq!(observed, expected);
        } else {
            expected = Some(observed);
        }
    }
}

#[test]
fn optional_scope_batch_does_not_erase_genuine_prior_cancellation_overflow() {
    let profile = Traversal {
        max_step: 0,
        max_grade: turts_rules::terrain::Ratio {
            numerator: 0,
            denominator: 1,
        },
    };
    let region = DestinationRegion {
        goal: Vec2::splat(4.),
        min: Vec2::splat(4.),
        max: Vec2::splat(4.),
        cohort_radius: 1.1,
    };
    let mut n = TerrainNavigation::default();
    n.record_dispatch_scopes(0, region, &[(1, 1, 1.1, profile)], 1);
    n.viewers[0].members.get_mut(&1).unwrap().cancelled = false;
    n.viewers[0].members.get_mut(&1).unwrap().live_target = true;
    n.viewers[0].cancellation_overflow = true;
    let work = n.viewers[0].diagnostics.invalidation_entries_this_tick;
    n.record_dispatch_scopes(0, region, &[(2, 1, 1.1, profile)], 1);
    assert!(
        n.viewers[0].cancellation_overflow,
        "genuine cancellation must remain for next propose"
    );
    assert!(n.viewers[0].members[&1].parking_scope.is_some());
    assert!(!n.viewers[0].members.contains_key(&2));
    assert_eq!(
        n.viewers[0].diagnostics.invalidation_entries_this_tick,
        work
    );
}
