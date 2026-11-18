//! Native authority-transition checks. These do not claim a public transfer command.
use super::*;

fn moving_fixture(reverse: bool) -> Fixture {
    let mut fixture = Fixture::configured(20, 100, 20, |catalog, map| {
        let worker = catalog.units.get_mut(&id("harvester")).unwrap();
        worker.movement = Some(MovementDef { speed: 0.25 });
        worker.health = Some(HealthDef { max: 100 });
        worker.removal = Some(RemovalDef {
            by_command: true,
            contents: ContentsPolicy::Destroy,
        });
        map.placements[1].owner_slot = 1;
        if reverse {
            map.placements.reverse();
        }
    });
    fixture.command(
        Behaviour::Move,
        OrderTargetSim::Point(crate::Vec2::new(6.5, 1.5)),
        QueueMode::Replace,
    );
    for _ in 0..4 {
        fixture.tick();
    }
    assert!(
        super::super::privileged_navigation_audit(fixture.app.world())
            .members
            .iter()
            .any(|member| member.unit == fixture.worker && member.viewer == 0),
        "the moving actor must have an active membership before its transition"
    );
    fixture
}

#[test]
fn ownership_transition_retires_old_intent_before_new_owner_can_move() {
    for reverse in [false, true] {
        let mut fixture = moving_fixture(reverse);
        let unit = fixture.worker;
        let entity = crate::unit_entity(fixture.app.world(), unit).unwrap();
        let before = fixture
            .app
            .world()
            .get::<crate::Position>(entity)
            .unwrap()
            .0;
        // Exercise an internal authoritative ownership transition. A public
        // player transfer command belongs to the later slot-lifecycle work.
        fixture
            .app
            .world_mut()
            .entity_mut(entity)
            .insert(crate::Owner(SlotId(1)));
        fixture.tick();
        assert!(
            super::super::privileged_navigation_audit(fixture.app.world())
                .members
                .iter()
                .all(|member| member.unit != unit),
            "neither viewer can inherit the old owner's movement membership"
        );
        for _ in 0..3 {
            assert_eq!(
                fixture
                    .app
                    .world()
                    .get::<crate::Position>(entity)
                    .unwrap()
                    .0,
                before
            );
            assert!(
                fixture
                    .app
                    .world()
                    .get::<crate::MoveTarget>(entity)
                    .is_none()
            );
            fixture.tick();
        }
        fixture.command_slot(
            SlotId(1),
            vec![unit],
            Behaviour::Move,
            OrderTargetSim::Point(crate::Vec2::new(6.5, 1.5)),
            QueueMode::Replace,
        );
        fixture.tick();
        assert!(
            fixture
                .app
                .world()
                .get::<crate::Position>(entity)
                .unwrap()
                .0
                .x
                > before.x
        );
        assert!(
            super::super::privileged_navigation_audit(fixture.app.world())
                .members
                .iter()
                .any(|member| member.unit == unit && member.viewer == 1)
        );
    }
}

#[test]
fn removal_and_zero_health_release_membership_under_allocation_reversal() {
    for reverse in [false, true] {
        for remove in [false, true] {
            let mut fixture = moving_fixture(reverse);
            let unit = fixture.worker;
            let before = fixture
                .app
                .world()
                .get::<crate::Position>(crate::unit_entity(fixture.app.world(), unit).unwrap())
                .unwrap()
                .0;
            if remove {
                fixture.command(Behaviour::Remove, OrderTargetSim::None, QueueMode::Replace);
            } else {
                // This is the real health transaction used by combat, rather
                // than an unchecked component write or a fabricated player input.
                crate::economy::transaction::transact(fixture.app.world_mut(), |tx| {
                    tx.stage_health(unit, 0);
                    Ok(())
                })
                .unwrap();
            }
            fixture.tick();
            fixture.tick();
            if !remove {
                let entity = crate::unit_entity(fixture.app.world(), unit).unwrap();
                assert_eq!(
                    fixture
                        .app
                        .world()
                        .get::<crate::Position>(entity)
                        .unwrap()
                        .0,
                    before,
                    "zero-health actor moved; reverse={reverse}"
                );
                assert!(
                    fixture
                        .app
                        .world()
                        .get::<crate::MoveTarget>(entity)
                        .is_none()
                );
                assert!(
                    fixture
                        .app
                        .world()
                        .get::<crate::orders::UnitOrders>(entity)
                        .is_none_or(|orders| orders.current.is_none() && orders.queue.is_empty())
                );
            }
            assert!(
                super::super::privileged_navigation_audit(fixture.app.world())
                    .members
                    .iter()
                    .all(|member| member.unit != unit),
                "membership remains; reverse={reverse}, remove={remove}"
            );
        }
    }
}
