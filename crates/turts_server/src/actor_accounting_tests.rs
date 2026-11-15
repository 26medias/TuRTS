//! Synthetic immutable completed views isolate accounting, not gameplay execution.
use super::*;
use turts_bot::{BotOrder, BotQueueMode, BotTarget, TaskKey};
use turts_mod_api::{CancelRefunds, ResolvedCost, ResolvedProduction};
use turts_sim::economy::{
    production::FundingState,
    views::{
        BalanceView, CostView, InventoryView, ItemView, JobView, OperatingJobView, OperatingView,
        QueueView,
    },
};

fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("cedar:{local}")).unwrap()
}
fn actor() -> BotActor {
    BotActor {
        slot: 0,
        profile: id("policy"),
    }
}
fn catalog() -> ContentCatalog {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
    let archive = turts_mod_runtime::archive::PackageArchive::parse(
        &turts_mod_runtime::pack::pack_dir(&path).unwrap(),
    )
    .unwrap();
    let mut catalog = archive.content().unwrap();
    catalog.resources.insert(
        id("credits"),
        turts_mod_api::ResourceDef {
            id: "credits".into(),
            name: "Credits".into(),
            unit_name: "credit".into(),
            kind: ResourceKind::Balance,
        },
    );
    catalog.production.insert(
        id("paid"),
        ResolvedProduction {
            id: id("paid"),
            name: "Paid".into(),
            output_unit: id("worker"),
            duration_ms: 600,
            costs: vec![ResolvedCost {
                source: CostSource::ProducerInventory,
                resource: id("wood"),
                amount: 12,
            }],
            payment: PaymentPolicy::ChargeOverTime,
            cancel_refunds: CancelRefunds {
                queued: Default::default(),
                running: Default::default(),
                output_blocked: Default::default(),
                destroyed: Default::default(),
            },
        },
    );
    catalog
}
fn unit(unit: u32, owner_slot: u8) -> turts_sim::world::UnitState {
    turts_sim::world::UnitState {
        unit_id: unit,
        content_id: id("worker").to_string(),
        owner_slot,
        facing: None,
        pos: turts_sim::Vec2::new(unit as f32, 0.0),
    }
}
fn view(tick: u32) -> WorldView {
    WorldView {
        tick,
        units: vec![unit(1, 0), unit(2, 0), unit(3, 1)],
        economy: turts_sim::EconomyStateView {
            balances: vec![
                BalanceView {
                    slot: 0,
                    resource: id("credits"),
                    on_hand: 90,
                    reserved: 10,
                },
                BalanceView {
                    slot: 1,
                    resource: id("credits"),
                    on_hand: 999,
                    reserved: 0,
                },
            ],
            inventories: vec![
                InventoryView {
                    unit: 1,
                    owner_slot: 0,
                    capacity: 50,
                    items: vec![ItemView {
                        resource: id("wood"),
                        on_hand: 20,
                        reserved: 0,
                    }],
                },
                InventoryView {
                    unit: 2,
                    owner_slot: 0,
                    capacity: 50,
                    items: vec![],
                },
                InventoryView {
                    unit: 3,
                    owner_slot: 1,
                    capacity: 50,
                    items: vec![ItemView {
                        resource: id("wood"),
                        on_hand: 999,
                        reserved: 0,
                    }],
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}
fn request(unit: u32, sequence: u32) -> (RequestKey, TrackedRequest) {
    let actor = actor();
    let key = RequestKey {
        actor: actor.clone(),
        sequence,
        unit,
    };
    let command = EmittedCommand {
        actor,
        sequence,
        task: TaskKey::new("economy", "opening", format!("builder-{unit}")),
        units: vec![unit],
        order: BotOrder {
            content: id("produce"),
            target: BotTarget::Content(id("paid")),
            queue: BotQueueMode::Replace,
            group: 0,
        },
    };
    (key, TrackedRequest::new(command))
}
fn watermark(tick: u32) -> Watermark {
    Watermark {
        completed_tick: tick,
        revision: u64::from(tick),
    }
}
fn batch(
    view: &WorldView,
    committed: &CommittedTick,
    catalog: &ContentCatalog,
    requests: &mut BTreeMap<RequestKey, TrackedRequest>,
) -> Result<ReconcileBatch, String> {
    build_batch(
        view,
        committed,
        catalog,
        &actor(),
        requests,
        watermark(view.tick),
        1000,
    )
}
fn paid_row(unit: u32, amount: u64) -> ReservationLineView {
    ReservationLineView {
        endpoint: ReservationEndpointView::Inventory(unit),
        resource: id("wood"),
        amount,
    }
}
fn event_row(unit: u32, amount: u64) -> turts_sim::CostLine {
    turts_sim::CostLine {
        source: EventSource::Inventory(unit),
        resource: id("wood"),
        amount,
    }
}
fn finalized_request(unit: u32, sequence: u32, job: u64) -> (RequestKey, TrackedRequest) {
    let (key, mut tracked) = request(unit, sequence);
    tracked.finalize(
        ActorResult {
            request: key.clone(),
            effect_watermark: watermark(1),
            accepted: true,
            work: Some(WorkIdentity {
                unit,
                id: job,
                kind: WorkKind::Production,
            }),
            reason: None,
        },
        Some(AdmittedWork {
            unit,
            job: turts_sim::JobId(job),
            recipe: id("paid"),
            construction: false,
            paid: vec![],
            reserved: vec![],
            done: 0,
            total: 12,
            payment: Some(PaymentPolicy::ChargeOverTime),
        }),
    );
    (key, tracked)
}
fn add_live_job(view: &mut WorldView, unit: u32, job: u64, done: u64, paid: u64, pending: u64) {
    view.queues.push(QueueView {
        producer: unit,
        owner_slot: 0,
        jobs: vec![JobView {
            id: job,
            recipe: id("paid"),
            stage: turts_sim::JobStage::Running,
            elapsed_ticks: done / 2,
            duration_ticks: 6,
            charged: if paid == 0 {
                vec![]
            } else {
                vec![CostView {
                    source: CostSource::ProducerInventory,
                    resource: id("wood"),
                    amount: paid,
                }]
            },
            reserved: vec![],
        }],
    });
    let operating = view.operating.get_or_insert(OperatingView {
        jobs: vec![],
        holds: vec![],
        rate_q: 2,
        supply: vec![],
        rates: vec![],
    });
    operating.jobs.push(OperatingJobView {
        unit,
        job,
        done,
        total: 12,
        held: false,
        held_since_tick: 0,
        funding: Some(FundingState {
            pending_charges: if pending == 0 {
                vec![]
            } else {
                vec![event_row(unit, pending)]
            },
            ..Default::default()
        }),
    });
}
#[test]
fn own_zero_inventory_accounts_are_exact_and_foreign_accounts_are_absent() {
    let catalog = catalog();
    let view = view(1);
    let result = batch(
        &view,
        &CommittedTick {
            tick: 1,
            ..Default::default()
        },
        &catalog,
        &mut BTreeMap::new(),
    )
    .unwrap();
    let own_zero = result
        .accounts
        .iter()
        .find(|row| {
            row.account
                == SpendKey {
                    endpoint: SourceEndpoint::Inventory {
                        unit: 2,
                        port: InventoryPort::Main,
                    },
                    resource: id("wood"),
                }
        })
        .unwrap();
    assert_eq!((own_zero.on_hand, own_zero.reserved), (0, 0));
    assert!(!result.accounts.iter().any(|row| matches!(
        row.account.endpoint,
        SourceEndpoint::SlotBalance(1) | SourceEndpoint::Inventory { unit: 3, .. }
    )));
    assert_eq!(
        result
            .accounts
            .iter()
            .find(|row| row.account.endpoint == SourceEndpoint::SlotBalance(0))
            .map(|row| (row.on_hand, row.reserved)),
        Some((90, 10))
    );
}
#[test]
fn unapplied_and_refused_members_are_separate_atomic_receipt_facts() {
    let catalog = catalog();
    let view = view(1);
    let (key, tracked) = request(1, 1);
    let (refused, mut other) = request(2, 2);
    other.finalize(
        ActorResult {
            request: refused.clone(),
            effect_watermark: watermark(1),
            accepted: false,
            work: None,
            reason: Some("unknown_order".into()),
        },
        None,
    );
    let mut requests = BTreeMap::from([(key.clone(), tracked), (refused.clone(), other)]);
    let result = batch(
        &view,
        &CommittedTick {
            tick: 1,
            ..Default::default()
        },
        &catalog,
        &mut requests,
    )
    .unwrap();
    assert_eq!(result.unapplied, vec![key]);
    assert_eq!(
        result.included,
        vec![ObservedRequestEffect::refusal(refused, watermark(1))]
    );
}
#[test]
fn cumulative_live_paid_and_flushed_buffers_do_not_count_one_debit_twice() {
    let catalog = catalog();
    let mut first = view(1);
    add_live_job(&mut first, 1, 5, 3, 3, 3);
    let (key, tracked) = finalized_request(1, 1, 5);
    let mut requests = BTreeMap::from([(key.clone(), tracked)]);
    let one = batch(
        &first,
        &CommittedTick {
            tick: 1,
            ..Default::default()
        },
        &catalog,
        &mut requests,
    )
    .unwrap();
    assert_eq!(one.included[0].paid[0].amount, 3);
    assert_eq!(one.included[0].work.as_ref().unwrap().done, 3);
    let mut second = view(2);
    add_live_job(&mut second, 1, 5, 5, 5, 1);
    let committed = CommittedTick {
        tick: 2,
        events: vec![turts_sim::EconomyEvent {
            tick: 2,
            ordinal: 0,
            owner_slot: 0,
            kind: EconomyEventKind::ProgressCharged {
                unit: 1,
                job: 5,
                charged: vec![event_row(1, 4)],
            },
        }],
        ..Default::default()
    };
    let two = batch(&second, &committed, &catalog, &mut requests).unwrap();
    assert_eq!(two.included[0].paid[0].amount, 5);
    assert_eq!(two.included[0].effect_watermark, watermark(1));
    assert_eq!(
        batch(&second, &committed, &catalog, &mut requests).unwrap(),
        two
    );
    assert_eq!(
        requests[&key].flushed.values().copied().collect::<Vec<_>>(),
        vec![4]
    );
}
#[test]
fn bound_or_late_member_failure_does_not_partly_advance_accounting() {
    let catalog = catalog();
    let mut view = view(1);
    add_live_job(&mut view, 1, 5, 3, 3, 3);
    add_live_job(&mut view, 2, 6, 4, 4, 99); // Deliberately inconsistent second member.
    let (one, first) = finalized_request(1, 1, 5);
    let (two, second) = finalized_request(2, 2, 6);
    let mut requests = BTreeMap::from([(one.clone(), first), (two.clone(), second)]);
    assert!(
        batch(
            &view,
            &CommittedTick {
                tick: 1,
                ..Default::default()
            },
            &catalog,
            &mut requests
        )
        .unwrap_err()
        .contains("flushed plus pending")
    );
    assert!(requests.values().all(|request| request.accounted.is_none()
        && request.last_effect.is_none()
        && request.flushed.is_empty()));
    assert!(
        build_batch(
            &view,
            &CommittedTick {
                tick: 1,
                ..Default::default()
            },
            &catalog,
            &actor(),
            &mut requests,
            watermark(1),
            1
        )
        .unwrap_err()
        .contains("row limit")
    );
    assert!(requests.values().all(|request| request.accounted.is_none()));
}
#[test]
fn different_original_inventories_remain_independent_and_missing_inventory_is_not_recreated() {
    let catalog = catalog();
    let mut view = view(1);
    add_live_job(&mut view, 1, 5, 3, 3, 3);
    add_live_job(&mut view, 2, 6, 4, 4, 4);
    let (one, first) = finalized_request(1, 1, 5);
    let (two, second) = finalized_request(2, 2, 6);
    let mut requests = BTreeMap::from([(one, first), (two, second)]);
    view.inventories.retain(|row| row.unit != 1);
    let result = batch(
        &view,
        &CommittedTick {
            tick: 1,
            ..Default::default()
        },
        &catalog,
        &mut requests,
    )
    .unwrap();
    assert_eq!(
        result.included[0].paid[0].account.endpoint,
        SourceEndpoint::Inventory {
            unit: 1,
            port: InventoryPort::Main
        }
    );
    assert_eq!(
        result.included[1].paid[0].account.endpoint,
        SourceEndpoint::Inventory {
            unit: 2,
            port: InventoryPort::Main
        }
    );
    assert!(!result.accounts.iter().any(|row| row.account.endpoint
        == SourceEndpoint::Inventory {
            unit: 1,
            port: InventoryPort::Main
        }));
}

#[test]
fn held_construction_retains_full_debt_then_tracks_a_site_after_constructor_loss() {
    use turts_sim::{economy::construction::HeldConstruction, receipts::CommandMember};
    let catalog = catalog();
    let (key, mut tracked) = request(1, 1);
    tracked.command.order.target = BotTarget::ContentAtPoint {
        content: id("paid"),
        point: turts_mod_api::WorldPos { x: 10.0, y: 10.0 },
    };
    tracked.finalize(
        ActorResult {
            request: key.clone(),
            effect_watermark: watermark(1),
            accepted: true,
            work: None,
            reason: None,
        },
        None,
    );
    let origin = CommandMember {
        slot: 0,
        sequence: 1,
        unit: 1,
    };
    let mut first = view(1);
    first.construction.held.insert(
        1,
        vec![HeldConstruction {
            origin: Some(origin),
            recipe: id("paid"),
            point: turts_sim::Vec2::new(10.0, 10.0),
        }],
    );
    let mut requests = BTreeMap::from([(key.clone(), tracked)]);
    let held = batch(
        &first,
        &CommittedTick {
            tick: 1,
            ..Default::default()
        },
        &catalog,
        &mut requests,
    )
    .unwrap();
    assert!(held.included[0].pending_placement);
    assert!(
        held.included[0].paid.is_empty()
            && held.included[0].reserved.is_empty()
            && !held.included[0].terminal
    );
    let mut next = view(2);
    next.units.retain(|unit| unit.unit_id != 1);
    next.inventories.retain(|inventory| inventory.unit != 1);
    // Site charged rows are public through serde and the read-only charged_rows accessor.
    next.construction.sites.insert(100,serde_json::from_value(serde_json::json!({
        "origin": { "slot": 0, "sequence": 1, "unit": 1 }, "funding": { "pending_charges": [{ "source": {"Inventory": 1}, "resource": id("wood"), "amount": 3 }] },
        "constructor": 1, "owner_slot": 0, "recipe": id("paid"), "job": 5, "done": 3, "total": 12,
        "charged": [{ "endpoint": { "Inventory": 1 }, "resource": id("wood"), "amount": 3 }]
    })).unwrap());
    let placed = batch(
        &next,
        &CommittedTick {
            tick: 2,
            ..Default::default()
        },
        &catalog,
        &mut requests,
    )
    .unwrap();
    let work = placed.included[0].work.as_ref().unwrap();
    assert_eq!(
        (
            work.identity.unit,
            work.identity.id,
            work.actor_unit,
            work.done
        ),
        (100, 5, 1, 3)
    );
    assert_eq!(placed.included[0].effect_watermark, watermark(1));
    assert!(!placed.included[0].pending_placement);
    assert_eq!(
        work.paid[0].account.endpoint,
        SourceEndpoint::Inventory {
            unit: 1,
            port: InventoryPort::Main
        }
    );
    assert!(!placed.accounts.iter().any(|row| row.account.endpoint
        == SourceEndpoint::Inventory {
            unit: 1,
            port: InventoryPort::Main
        }));
}
#[test]
fn same_tick_completion_and_destruction_keep_exact_actual_paid_and_terminal_state() {
    use turts_sim::receipts::{TerminalWork, TerminalWorkState};
    for (done, state) in [
        (12, TerminalWorkState::Completed),
        (3, TerminalWorkState::Destroyed),
        (0, TerminalWorkState::Cancelled),
    ] {
        let catalog = catalog();
        let view = view(1);
        let (key, tracked) = finalized_request(1, 1, 5);
        let mut requests = BTreeMap::from([(key.clone(), tracked)]);
        let paid = if done == 0 {
            vec![]
        } else {
            vec![paid_row(1, done)]
        };
        let mut committed = CommittedTick {
            tick: 1,
            terminal_work: vec![TerminalWork {
                unit: 1,
                job: 5,
                recipe: id("paid"),
                construction: false,
                done,
                total: 12,
                payment: Some(PaymentPolicy::ChargeOverTime),
                paid,
                reserved: vec![],
                state,
            }],
            ..Default::default()
        };
        if done > 0 {
            committed.events.push(turts_sim::EconomyEvent {
                tick: 1,
                ordinal: 0,
                owner_slot: 0,
                kind: EconomyEventKind::ProgressCharged {
                    unit: 1,
                    job: 5,
                    charged: vec![event_row(1, done)],
                },
            });
        }
        let result = batch(&view, &committed, &catalog, &mut requests).unwrap();
        let effect = &result.included[0];
        let work = effect.work.as_ref().unwrap();
        assert!(effect.terminal);
        assert_eq!(work.done, done);
        assert_eq!(work.total, 12);
        assert_eq!(
            work.state,
            if state == TerminalWorkState::Completed {
                WorkState::Finished
            } else {
                WorkState::Failed
            }
        );
        assert_eq!(effect.paid.iter().map(|row| row.amount).sum::<u64>(), done);
        let later = batch(
            &super::tests::view(2),
            &CommittedTick {
                tick: 2,
                ..Default::default()
            },
            &catalog,
            &mut requests,
        )
        .unwrap();
        assert_eq!(
            later.included, result.included,
            "terminal data survives its one-tick source event"
        );
    }
}
#[test]
fn cancelled_reserved_work_releases_its_claim_without_inventing_paid_costs() {
    use turts_sim::receipts::{TerminalWork, TerminalWorkState};
    let mut catalog = catalog();
    catalog.production.get_mut(&id("paid")).unwrap().payment = PaymentPolicy::ReserveThenCharge;
    let (key, mut tracked) = finalized_request(1, 1, 5);
    let hint = tracked.admitted_work.as_mut().unwrap();
    hint.payment = Some(PaymentPolicy::ReserveThenCharge);
    hint.reserved = vec![paid_row(1, 12)];
    let mut requests = BTreeMap::from([(key, tracked)]);
    let committed = CommittedTick {
        tick: 1,
        terminal_work: vec![TerminalWork {
            unit: 1,
            job: 5,
            recipe: id("paid"),
            construction: false,
            done: 0,
            total: 12,
            payment: Some(PaymentPolicy::ReserveThenCharge),
            paid: vec![],
            reserved: vec![],
            state: TerminalWorkState::Cancelled,
        }],
        ..Default::default()
    };
    let result = batch(&view(1), &committed, &catalog, &mut requests).unwrap();
    assert!(
        result.included[0].paid.is_empty()
            && result.included[0].reserved.is_empty()
            && result.included[0].terminal
    );
}
