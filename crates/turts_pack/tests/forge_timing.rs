mod common;
use common::{
    conservation::OracleCatalog,
    forge_support::{cid, forge_dir, loaded, package_fixture, step},
};
use turts_sim::{EconomyEventKind, audit_state};

#[test]
fn forge_literal_phase_timing_at_twenty_hz() {
    let path = forge_dir();
    let (mut app, oracle, mut history) = loaded(&path);
    let mut extracted = Vec::new();
    let mut produced = Vec::new();
    let mut arm_trace = Vec::new();
    let mut first_belt_tick = None;
    let mut first_press_arm_hand_tick = None;
    for tick in 1..=300 {
        for op in step(&mut app, &oracle, &mut history) {
            for event in op.events {
                match event.kind {
                    EconomyEventKind::Extracted { .. } if extracted.len() < 3 => {
                        extracted.push(event.tick)
                    }

                    EconomyEventKind::ItemsProduced { .. } if produced.len() < 3 => {
                        produced.push(event.tick)
                    }
                    _ => {}
                }
            }
        }
        let state = audit_state(app.world_mut());
        if tick <= 40 {
            let row = state.raw_inserters.iter().find(|r| r.unit == 2).unwrap();
            arm_trace.push((
                tick,
                row.stage,
                row.elapsed,
                common::forge_support::inventory_amount(&state, 2, "forge:ore"),
            ));
        }
        if first_press_arm_hand_tick.is_none()
            && common::forge_support::inventory_amount(&state, 3, "forge:ore") > 0
        {
            first_press_arm_hand_tick = Some(tick);
        }
        if first_belt_tick.is_none()
            && state
                .raw_belts
                .iter()
                .flat_map(|b| &b.raw_slots)
                .any(|slot| slot.item.is_some())
        {
            first_belt_tick = Some(tick);
        }
    }
    assert_eq!(
        extracted,
        [10, 20, 30],
        "extractor carry produces two ore per second"
    );
    assert_eq!(
        arm_trace[9],
        (
            10,
            turts_sim::economy::views_m3::RawInserterStageAudit::ToDestination,
            0,
            1
        )
    );
    assert_eq!(
        arm_trace[14],
        (
            15,
            turts_sim::economy::views_m3::RawInserterStageAudit::Returning,
            0,
            0
        )
    );
    assert_eq!(
        arm_trace[19],
        (
            20,
            turts_sim::economy::views_m3::RawInserterStageAudit::ToDestination,
            0,
            1
        )
    );
    assert_eq!(
        first_belt_tick,
        Some(15),
        "the first ore reaches the belt on tick 15"
    );
    assert_eq!(
        first_press_arm_hand_tick,
        Some(115),
        "the first item is picked at tick 115 after the 100-tick belt transit"
    );
    assert_eq!(produced, [170, 211, 252], "press item cycles are 41 ticks");
}

#[test]
fn weapon_cooldown_is_twenty_ticks_with_preloaded_ammo() {
    let path = package_fixture("forge_c15");
    let (mut app, oracle, mut history) = loaded(&path);
    let initial = audit_state(app.world_mut());
    assert_eq!(
        common::forge_support::inventory_amount(&initial, 7, "forge:shell"),
        5,
        "fixture preloads five shells in the magazine"
    );
    assert_eq!(
        OracleCatalog::parse(&path).unwrap().units[&cid("forge:drone")]
            .health
            .unwrap()
            .max,
        400
    );
    let mut shots = Vec::new();
    for _ in 0..81 {
        for op in step(&mut app, &oracle, &mut history) {
            for event in op.events {
                if matches!(event.kind, EconomyEventKind::Fired { target: 9, .. }) {
                    shots.push(event.tick);
                }
            }
        }
    }
    assert_eq!(
        shots,
        [1, 21, 41, 61, 81],
        "weapon cooldown is exactly 20 ticks at 20 Hz"
    );
}
