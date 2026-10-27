mod common;
use common::{advance, cid, economy_archive, scenario};
use turts_pack::load_package;
use turts_sim::{
    Command, CommandKind, EconomyEventKind, SlotId, apply_command, audit_state, committed_tick,
};

fn command(seq: u32, producer: u32, recipe: &str) -> Command {
    Command {
        slot: SlotId(0),
        seq,
        kind: CommandKind::Produce {
            producer_id: producer,
            recipe_id: cid(recipe),
        },
    }
}

#[test]
fn completion_spawns_once_after_exact_duration() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let job = apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    advance(&mut app, 120);
    assert_eq!(audit_state(app.world_mut()).units.len(), 5);
    app.update();
    let after = audit_state(app.world_mut());
    assert_eq!(after.units.len(), 6);
    assert!(
        after
            .units
            .iter()
            .any(|u| u.content_id == "probe:worker" && u.owner_slot == 0)
    );
    assert_eq!(committed_tick(app.world()).spawned.len(), 1);
    assert!(committed_tick(app.world()).events.iter().any(|e| matches!(e.kind, EconomyEventKind::ProductionCompleted { producer: 1, job: id, .. } if id == job.0)));
    app.update();
    assert_eq!(audit_state(app.world_mut()).units.len(), 6);
    assert_eq!(committed_tick(app.world()).spawned.len(), 0);
}

#[test]
fn queued_cancellation_releases_reservation_without_refund() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let job = apply_command(app.world_mut(), &command(1, 1, "probe:hire"))
        .outcome
        .unwrap()
        .job_id
        .unwrap();
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: job,
            },
        },
    );
    assert!(result.outcome.is_ok());
    let state = audit_state(app.world_mut());
    let timber = state
        .economy
        .inventories
        .iter()
        .find(|i| i.unit == 1)
        .unwrap()
        .items
        .iter()
        .find(|i| i.resource == cid("probe:timber"))
        .unwrap();
    assert_eq!((timber.on_hand, timber.reserved), (100, 0));
    assert!(
        state
            .economy
            .queues
            .iter()
            .find(|q| q.producer == 1)
            .unwrap()
            .jobs
            .is_empty()
    );
    app.update();
    assert!(committed_tick(app.world()).events.iter().any(
        |e| matches!(e.kind, EconomyEventKind::ProductionCancelled { job: id, .. } if id == job.0)
    ));
}
