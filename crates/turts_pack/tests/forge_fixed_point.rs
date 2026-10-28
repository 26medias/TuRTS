mod common;
use common::{
    conservation::stock,
    forge_support::{cid, command, forge_dir, inventory_amount, loaded, step},
};
use std::collections::BTreeMap;
use turts_sim::{CommandKind, EconomyEventKind, JobStage, audit_state};

#[test]
fn forge_spec_13_4_fixed_point_and_ten_vs_five_shots() {
    let path = forge_dir();
    let (mut app, oracle, mut history) = loaded(&path);
    let ore = oracle.item("forge:ore").unwrap();
    let shell = oracle.item("forge:shell").unwrap();
    let mut produce_started = false;
    let mut moved_second_target = false;
    let mut completed_tick = None;
    let mut shots = BTreeMap::<u32, u64>::new();
    let mut shells_produced = 0_u64;
    let mut ore_charged = 0_u64;
    let mut ore_extracted = 0_u64;
    let mut finished = false;
    for _ in 0..5000 {
        for op in step(&mut app, &oracle, &mut history) {
            for event in op.events {
                match event.kind {
                    EconomyEventKind::Fired { target, .. } => {
                        *shots.entry(target).or_default() += 1
                    }
                    EconomyEventKind::ItemsProduced { outputs, .. } => {
                        for line in outputs {
                            if line.resource == shell {
                                shells_produced += line.amount;
                            }
                        }
                    }
                    EconomyEventKind::ProductionStarted { charged, .. } => {
                        for line in charged {
                            if line.resource == ore {
                                ore_charged += line.amount;
                            }
                        }
                    }
                    EconomyEventKind::Extracted { item, amount, .. } if item == ore => {
                        ore_extracted += amount
                    }
                    EconomyEventKind::ResearchCompleted {
                        slot, technology, ..
                    } if slot == 0 && technology == cid("forge:hardened_shells") => {
                        completed_tick = Some(event.tick)
                    }
                    _ => {}
                }
            }
        }
        let state = audit_state(app.world_mut());
        if !produce_started && inventory_amount(&state, 8, "forge:shell") == 10 {
            let result = command(
                &mut app,
                &oracle,
                &mut history,
                0,
                1,
                CommandKind::Produce {
                    producer_id: 8,
                    recipe_id: cid("forge:hardened_shells"),
                },
            );
            assert!(
                result.outcome.is_ok(),
                "research starts when the lab holds ten shells"
            );
            produce_started = true;
        }
        if completed_tick.is_some() && !moved_second_target {
            let result = command(
                &mut app,
                &oracle,
                &mut history,
                1,
                1,
                CommandKind::MoveTo {
                    unit_ids: vec![10],
                    target: turts_sim::Vec2::new(24.0, 14.0),
                },
            );
            assert!(result.outcome.is_ok());
            moved_second_target = true;
        }
        let s = audit_state(app.world_mut());
        let job = s
            .raw_queues
            .iter()
            .find(|q| q.producer == 4)
            .and_then(|q| q.jobs.first());
        let jam = job.is_some_and(|j| j.stage == JobStage::OutputBlocked)
            && s.nodes.first().is_some_and(|n| n.remaining == 176)
            && s.raw_belts
                .iter()
                .flat_map(|b| &b.raw_slots)
                .filter(|x| x.item.is_some())
                .count()
                == 24
            && inventory_amount(&s, 1, "forge:ore") == 4
            && inventory_amount(&s, 4, "forge:ore") == 6
            && common::forge_support::output_amount(&s, 4, "forge:shell") == 4
            && inventory_amount(&s, 7, "forge:shell") == 5
            && inventory_amount(&s, 8, "forge:shell") == 10
            && [2, 3, 5, 6].iter().all(|id| {
                inventory_amount(&s, *id, "forge:ore") == 0
                    && inventory_amount(&s, *id, "forge:shell") == 0
            })
            && s.raw_queues
                .iter()
                .find(|q| q.producer == 4)
                .and_then(|q| q.jobs.first())
                .is_some_and(|j| {
                    j.charged
                        .iter()
                        .any(|line| line.resource == ore && line.amount == 2)
                });
        if jam && shots.get(&9) == Some(&10) && shots.get(&10) == Some(&5) {
            finished = true;
            break;
        }
    }
    let s = audit_state(app.world_mut());
    assert!(
        finished,
        "SPEC13.4 jam was not reached: node={}, press ore={}, belt ore={}, drill ore={}, shells produced={shells_produced}, ore charged={ore_charged}, ore extracted={ore_extracted}, shots={shots:?}, research={completed_tick:?}",
        s.nodes.first().map_or(0, |n| n.remaining),
        inventory_amount(&s, 4, "forge:ore"),
        s.raw_belts
            .iter()
            .flat_map(|b| &b.raw_slots)
            .filter(|x| x.item.as_ref() == Some(&ore))
            .count(),
        inventory_amount(&s, 1, "forge:ore")
    );
    assert_eq!(shells_produced, 44, "SPEC13.4 shell production table");
    assert_eq!(ore_charged, 90, "SPEC13.4 ore charge table");
    assert_eq!(ore_extracted, 124, "SPEC13.4 extraction table");
    assert_eq!(
        stock(&s, &oracle, &ore).unwrap(),
        210,
        "176 node + 6 press + 24 belt + 4 drill"
    );
    assert_eq!(
        stock(&s, &oracle, &shell).unwrap(),
        19,
        "25 shells made minus 15 shots and 10 research charge"
    );
    assert_eq!(
        shots.get(&9),
        Some(&10),
        "unresearched drone A takes ten base-damage shots"
    );
    assert_eq!(
        shots.get(&10),
        Some(&5),
        "researched drone B takes five upgraded shots"
    );
    assert_eq!(
        completed_tick,
        Some(1361),
        "reference research completion tick from the specified rates"
    );
    assert!(
        moved_second_target,
        "slot 1 move is released only after the research completion event"
    );
}
