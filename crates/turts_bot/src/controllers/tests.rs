#![cfg(test)]
use super::*;
use std::collections::VecDeque;
use turts_mod_api::{BotGoal, BotKnowledge, BotOrderBinding, BotQuota, BotRole, BotStartWhen};
fn id(s: &str) -> ContentId {
    ContentId::parse(s).unwrap()
}
fn unit(unit: u32, x: f32, y: f32) -> UnitObservation {
    UnitObservation {
        unit,
        owner_slot: 0,
        content: id("fixture:worker"),
        position: WorldPos { x, y },
        radius: 0.5,
        capabilities: vec![],
        health: Some((10, 10)),
        site: false,
        own: Some(OwnUnitObservation {
            enabled: true,
            activity: OwnActivity::Idle,
            current_order: None,
            queued_orders: 0,
        }),
    }
}
fn controller() -> StockController {
    StockController {
        key: "policy".into(),
        policy: Policy::Economy,
        scans: vec![],
        basis: None,
        placement_steps: Default::default(),
    }
}
fn profile() -> BotDef {
    BotDef {
        id: "opponent".into(),
        name: "Opponent".into(),
        decision_ms: 1000,
        start_when: BotStartWhen::MatchRunning,
        controllers: vec![],
        knowledge: BotKnowledge {
            roles: vec![BotRole {
                key: "workers".into(),
                units: vec!["fixture:worker".into()],
                requires: vec![],
            }],
            orders: vec![BotOrderBinding {
                key: "gather".into(),
                order: "fixture:gather".into(),
                actor_role: "workers".into(),
            }],
            recovery: vec![BotGoal {
                key: "harvest".into(),
                priority: 90,
                action: BotGoalAction::Gather {
                    role: "workers".into(),
                    resource: "fixture:ore".into(),
                    order: "gather".into(),
                },
            }],
            ..Default::default()
        },
    }
}
fn run(
    policy: &mut StockController,
    profile: &BotDef,
    queries: &mut dyn BotQueries,
) -> Vec<Proposal> {
    run_with_metadata(policy, profile, queries, 1, &[], &[])
}
fn run_with_metadata(
    policy: &mut dyn BotController,
    profile: &BotDef,
    queries: &mut dyn BotQueries,
    basis: BasisId,
    role_counts: &[RoleCount],
    supply: &[SupplyObservation],
) -> Vec<Proposal> {
    let actor = BotActor {
        slot: 0,
        profile: id("fixture:opponent"),
    };
    let batch = ReconcileBatch {
        unapplied: vec![],
        watermark: Watermark {
            completed_tick: 1,
            revision: 1,
        },
        accounts: vec![],
        included: vec![],
        complete: true,
    };
    let frame = BotFrame {
        basis,
        accounting_basis: basis,
        completed_tick: 1,
        scheduler_tick: 1,
        active_ticks: 20,
        actor: &actor,
        tick_hz: 20,
        phase: MatchPhase::Running,
        profile,
        quota: BotQuota {
            queries: 32,
            candidates: 4096,
            rows: 1024,
            proposals: 32,
        },
        limits: Default::default(),
        reconciliation: &batch,
        role_counts,
        supply,
    };
    let mut sink = ProposalBuffer::new("policy", 32, 256);
    policy.step(&frame, queries, &mut sink);
    sink.into_proposals()
}
fn page(data: QueryData, state: PageState) -> Result<QueryPage, QueryError> {
    Ok(QueryPage {
        basis: 1,
        completed_tick: 1,
        data,
        state,
        coverage: Coverage::Complete,
        usage: Default::default(),
    })
}
struct Pages(VecDeque<Result<QueryPage, QueryError>>);
impl BotQueries for Pages {
    fn pin_basis(&mut self, _: BasisId) -> Result<(), QueryError> {
        Ok(())
    }
    fn release_basis(&mut self, _: BasisId) -> Result<(), QueryError> {
        Ok(())
    }
    fn query(&mut self, _: Query, _: Option<QueryCursor>, _: u32) -> Result<QueryPage, QueryError> {
        self.0.pop_front().expect("unexpected query")
    }
}
fn node(node: u32, x: f32, y: f32) -> NodeObservation {
    NodeObservation {
        node,
        content: id("fixture:vein"),
        item: id("fixture:ore"),
        position: WorldPos { x, y },
        radius: 1.,
        remaining: 100,
    }
}
#[test]
fn source_selection_uses_physical_position_and_refuses_exact_ties() {
    assert_eq!(
        unique_first(vec![unit(1, 8., 0.), unit(99, 2., 0.)])
            .unwrap()
            .unit,
        99
    );
    assert_eq!(
        unique_first(vec![unit(99, 8., 0.), unit(1, 2., 0.)])
            .unwrap()
            .unit,
        1
    );
    assert!(unique_first(vec![unit(1, 2., 0.), unit(99, 2., 0.)]).is_none());
}
#[test]
fn gathering_refuses_equal_distance_nodes_even_when_ids_are_inverted() {
    for nodes in [
        vec![node(1, -1., 0.), node(99, 1., 0.)],
        vec![node(99, -1., 0.), node(1, 1., 0.)],
    ] {
        let mut q = Pages(VecDeque::from([
            page(
                QueryData::Units(vec![unit(10, 0., 0.)]),
                PageState::Complete,
            ),
            page(QueryData::Nodes(nodes), PageState::Complete),
        ]));
        assert!(run(&mut controller(), &profile(), &mut q).is_empty());
    }
}
#[test]
fn gathering_keeps_existing_orders_and_never_moves_a_working_actor() {
    let mut worker = unit(10, 0., 0.);
    worker.own.as_mut().unwrap().activity = OwnActivity::Gathering;
    let mut q = Pages(VecDeque::from([
        page(QueryData::Units(vec![worker]), PageState::Complete),
        page(
            QueryData::Nodes(vec![node(20, 1., 0.)]),
            PageState::Complete,
        ),
    ]));
    assert!(run(&mut controller(), &profile(), &mut q).is_empty());
}
#[test]
fn incomplete_pages_never_choose_a_prefix_and_resume_on_the_pinned_cursor() {
    let mut policy = controller();
    let query = Query::Units(UnitQuery::own(vec![]));
    let cursor = QueryCursor { basis: 1, token: 7 };
    let mut q = Pages(VecDeque::from([
        page(
            QueryData::Units(vec![unit(1, 8., 0.)]),
            PageState::More(cursor),
        ),
        Err(QueryError::Budget),
    ]));
    assert!(policy.collect(query.clone(), &mut q).is_none());
    assert_eq!(policy.scans.len(), 1);
    let mut q = Pages(VecDeque::from([page(
        QueryData::Units(vec![unit(99, 2., 0.)]),
        PageState::Complete,
    )]));
    let Some(QueryData::Units(rows)) = policy.collect(query, &mut q) else {
        panic!("complete scan")
    };
    assert_eq!(unique_first(rows).unwrap().unit, 99);
    assert!(policy.scans[0].complete.is_some());
}
#[test]
fn expired_selection_discards_its_prefix_without_proposing() {
    let mut policy = controller();
    let query = Query::Units(UnitQuery::own(vec![]));
    let mut q = Pages(VecDeque::from([
        page(
            QueryData::Units(vec![unit(1, 8., 0.)]),
            PageState::UnresolvedTie(QueryCursor { basis: 1, token: 7 }),
        ),
        Err(QueryError::Budget),
    ]));
    assert!(policy.collect(query.clone(), &mut q).is_none());
    let mut q = Pages(VecDeque::from([Err(QueryError::ExpiredBasis)]));
    assert!(policy.collect(query, &mut q).is_none());
    assert!(policy.scans.is_empty());
}
#[test]
fn factory_registration_is_profile_independent_and_rejects_unhandled_config() {
    let mut registry = ControllerRegistry::new();
    register_stock(&mut registry).unwrap();
    assert!(registry.contains(&id("engine:economy")));
    let mut instance = BotControllerDef {
        key: "economy".into(),
        factory: "engine:economy".into(),
        config: BotValue::Null,
        quota: Default::default(),
    };
    let factory = StockFactory(Policy::Economy);
    factory.validate(&profile(), &instance).unwrap();
    instance.config = BotValue::Bool(true);
    assert_eq!(
        factory
            .validate(&profile(), &instance)
            .unwrap_err()
            .field_path,
        "config"
    );
}

#[test]
fn completed_queries_remain_cached_while_a_later_selection_is_incomplete() {
    let mut policy = controller();
    let query = Query::Units(UnitQuery::own(vec![]));
    let mut q = Pages(VecDeque::from([page(
        QueryData::Units(vec![unit(1, 0., 0.)]),
        PageState::Complete,
    )]));
    assert!(policy.collect(query.clone(), &mut q).is_some());
    let mut no_queries = Pages(VecDeque::new());
    assert!(policy.collect(query, &mut no_queries).is_some());
}

#[test]
fn later_query_resumes_its_old_basis_after_accounting_advances() {
    let mut policy = controller();
    let mut q = Pages(VecDeque::from([
        page(
            QueryData::Units(vec![unit(10, 0., 0.)]),
            PageState::Complete,
        ),
        Err(QueryError::Budget),
    ]));
    assert!(run(&mut policy, &profile(), &mut q).is_empty());
    assert_eq!(policy.basis, Some(1));
    // The completed actor page must not be re-read on the newer account basis.
    let mut q = Pages(VecDeque::from([page(
        QueryData::Nodes(vec![node(20, 1., 0.)]),
        PageState::Complete,
    )]));
    let proposals = run_with_metadata(&mut policy, &profile(), &mut q, 2, &[], &[]);
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].source_basis, 1);
    assert!(policy.basis.is_none());
    assert!(policy.scans.is_empty());
}
fn recovery_profile() -> BotDef {
    let mut p = profile();
    p.knowledge.roles.push(BotRole {
        key: "trainers".into(),
        units: vec!["fixture:trainer".into()],
        requires: vec![],
    });
    p.knowledge.orders.push(BotOrderBinding {
        key: "train".into(),
        order: "fixture:produce".into(),
        actor_role: "trainers".into(),
    });
    p.knowledge.recipes.push(BotRecipeBinding {
        key: "replacement".into(),
        recipe: "fixture:worker_recipe".into(),
        actor_role: "trainers".into(),
        order: "train".into(),
        kind: BotRecipeKind::Produce,
    });
    p.knowledge.recovery = vec![BotGoal {
        key: "restore_worker".into(),
        priority: 100,
        action: BotGoalAction::EnsureUnits {
            role: "workers".into(),
            count: 1,
            recipe: "replacement".into(),
        },
    }];
    p
}
#[test]
fn missing_gatherer_emits_an_ordinary_high_priority_replacement() {
    let mut trainer = unit(40, 2., 3.);
    trainer.content = id("fixture:trainer");
    let mut q = Pages(VecDeque::from([page(
        QueryData::Units(vec![trainer]),
        PageState::Complete,
    )]));
    let proposals = run(&mut controller(), &recovery_profile(), &mut q);
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].priority, 100);
    assert_eq!(proposals[0].actors, vec![40]);
    assert_eq!(proposals[0].order.content, id("fixture:produce"));
    assert_eq!(
        proposals[0].order.target,
        BotTarget::Content(id("fixture:worker_recipe"))
    );
}
#[test]
fn a_pending_paid_replacement_prevents_duplicate_recovery_proposals() {
    let mut q = Pages(VecDeque::new());
    let counts = [RoleCount {
        role: "workers".into(),
        completed: 0,
        pending: 1,
    }];
    assert!(
        run_with_metadata(
            &mut controller(),
            &recovery_profile(),
            &mut q,
            1,
            &counts,
            &[]
        )
        .is_empty()
    );
}
#[test]
fn pending_supply_provider_prevents_duplicate_construction_and_loss_replans() {
    let mut p = recovery_profile();
    p.knowledge.recovery[0].action = BotGoalAction::EnsureSupply {
        resource: "fixture:beds".into(),
        amount: 4,
        provider_role: "housing".into(),
        recipe: "replacement".into(),
    };
    p.knowledge.recipes[0].kind = BotRecipeKind::Construct;
    let supply = [SupplyObservation {
        resource: id("fixture:beds"),
        provided: 0,
        consumed: 0,
    }];
    let counts = [RoleCount {
        role: "housing".into(),
        completed: 0,
        pending: 1,
    }];
    let mut q = Pages(VecDeque::new());
    assert!(run_with_metadata(&mut controller(), &p, &mut q, 1, &counts, &supply).is_empty());
    let mut q = Pages(VecDeque::from([
        page(
            QueryData::Units(vec![unit(40, 2., 3.)]),
            PageState::Complete,
        ),
        page(
            QueryData::Placement(Advice::Known(PlacementAdvice::Allowed)),
            PageState::Complete,
        ),
    ]));
    let proposals = run_with_metadata(&mut controller(), &p, &mut q, 1, &[], &supply);
    assert_eq!(proposals.len(), 1);
    assert!(matches!(
        proposals[0].order.target,
        BotTarget::ContentAtPoint { .. }
    ));
}

#[test]
fn actor_quota_never_chooses_one_coincident_source_by_storage_order() {
    for rows in [
        vec![unit(1, 0., 0.), unit(2, 0., 0.), unit(3, 2., 0.)],
        vec![unit(2, 0., 0.), unit(1, 0., 0.), unit(3, 2., 0.)],
    ] {
        let selected = ranked_sources(rows, 1);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].unit, 3);
    }
}
#[test]
fn large_squads_select_within_affected_unit_quota_by_physical_rank() {
    let rows = (0..300).map(|n| unit(300 - n, n as f32, 0.)).collect();
    let selected = ranked_sources(rows, 256);
    assert_eq!(selected.len(), 256);
    assert_eq!(selected.first().unwrap().unit, 300);
    assert_eq!(selected.last().unwrap().unit, 45);
}

fn squad_profile() -> BotDef {
    let mut p = profile();
    p.knowledge.roles.push(BotRole {
        key: "home".into(),
        units: vec!["fixture:home".into()],
        requires: vec![],
    });
    p.knowledge.orders.extend([
        BotOrderBinding {
            key: "attack".into(),
            order: "fixture:attack".into(),
            actor_role: "workers".into(),
        },
        BotOrderBinding {
            key: "move".into(),
            order: "fixture:move".into(),
            actor_role: "workers".into(),
        },
    ]);
    p.knowledge.squads = vec![turts_mod_api::BotSquadRule {
        key: "main".into(),
        role: "workers".into(),
        min_units: 1,
        anchor_role: "home".into(),
        attack_order: "attack".into(),
        move_order: "move".into(),
        defend_radius: 18.0,
        assemble_radius: 8.0,
        target_roles: vec![],
        priority: 50,
    }];
    p.controllers = vec![BotControllerDef {
        key: "policy".into(),
        factory: "engine:squads".into(),
        config: BotValue::Null,
        quota: turts_mod_api::BotQuota {
            queries: 8,
            candidates: 1024,
            rows: 256,
            proposals: 8,
        },
    }];
    p
}
fn squad_pages(targets: Vec<UnitObservation>) -> Pages {
    let mut home = unit(11, 54., 24.);
    home.content = id("fixture:home");
    Pages(VecDeque::from([
        page(
            QueryData::Units(vec![unit(10, 50., 28.)]),
            PageState::Complete,
        ),
        page(QueryData::Units(vec![home]), PageState::Complete),
        page(QueryData::Units(targets), PageState::Complete),
    ]))
}
#[test]
fn equidistant_squad_targets_use_geometry_after_id_and_registration_inversion() {
    let p = squad_profile();
    let actor = BotActor {
        slot: 0,
        profile: id("fixture:opponent"),
    };
    for inverted_registry in [false, true] {
        for (lower_id, upper_id) in [(1, 99), (99, 1)] {
            for reverse_page in [false, true] {
                let mut registry = ControllerRegistry::new();
                let mut factories = [("squads", Policy::Squads), ("economy", Policy::Economy)];
                if inverted_registry {
                    factories.reverse();
                }
                for (name, policy) in factories {
                    registry
                        .register(
                            id(&format!("engine:{name}")),
                            std::sync::Arc::new(StockFactory(policy)),
                        )
                        .unwrap();
                }
                let mut controllers = registry
                    .activate(
                        &actor,
                        &p,
                        42,
                        turts_mod_api::BotQuota {
                            queries: 32,
                            candidates: 4096,
                            rows: 1024,
                            proposals: 32,
                        },
                    )
                    .unwrap();
                let mut lower = unit(lower_id, 14., 20.);
                lower.owner_slot = 1;
                lower.own = None;
                let mut upper = unit(upper_id, 14., 28.);
                upper.owner_slot = 1;
                upper.own = None;
                let mut rows = vec![lower, upper];
                if reverse_page {
                    rows.reverse();
                }
                let proposals = run_with_metadata(
                    controllers[0].controller.as_mut(),
                    &p,
                    &mut squad_pages(rows),
                    1,
                    &[],
                    &[],
                );
                assert_eq!(
                    proposals.len(),
                    1,
                    "a distinct equal-distance target must not freeze the squad"
                );
                assert_eq!(proposals[0].order.target, BotTarget::Unit(lower_id));
            }
        }
    }
}
#[test]
fn complete_physical_target_ties_remain_refused_with_inverted_ids() {
    for ids in [[1, 99], [99, 1]] {
        let rows = ids
            .into_iter()
            .map(|id| {
                let mut target = unit(id, 14., 20.);
                target.owner_slot = 1;
                target.own = None;
                target
            })
            .collect();
        let mut controller = controller();
        controller.policy = Policy::Squads;
        assert!(run(&mut controller, &squad_profile(), &mut squad_pages(rows)).is_empty());
    }
}

#[test]
fn squad_scouts_complete_remembered_pages_as_points_and_never_reuses_stale_ids() {
    fn memory(unit: u32, x: f32) -> turts_mod_api::RememberedUnit {
        turts_mod_api::RememberedUnit {
            unit,
            content: id("fixture:enemy"),
            owner: 1,
            position: WorldPos { x, y: 24.0 },
            facing: None,
            site: false,
            last_seen_tick: 1,
        }
    }
    for invert in [false, true] {
        let mut controller = controller();
        controller.policy = Policy::Squads;
        let mut queries = squad_pages(vec![]);
        let first = if invert {
            memory(99, 60.0)
        } else {
            memory(1, 60.0)
        };
        let second = if invert {
            memory(1, 70.0)
        } else {
            memory(99, 70.0)
        };
        queries.0.extend([
            page(
                QueryData::RememberedUnits(RememberedRows {
                    rows: vec![first],
                    retention_truncated: true,
                }),
                PageState::More(QueryCursor { basis: 1, token: 1 }),
            ),
            page(
                QueryData::RememberedUnits(RememberedRows {
                    rows: vec![second],
                    retention_truncated: true,
                }),
                PageState::Complete,
            ),
        ]);
        let proposals = run(&mut controller, &squad_profile(), &mut queries);
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].order.content, id("fixture:move"));
        assert_eq!(
            proposals[0].order.target,
            BotTarget::Point(WorldPos { x: 60.0, y: 24.0 })
        );
    }
    let mut controller = controller();
    controller.policy = Policy::Squads;
    let mut queries = squad_pages(vec![]);
    queries.0.extend([
        page(
            QueryData::RememberedUnits(RememberedRows {
                rows: vec![memory(1, 60.0)],
                retention_truncated: false,
            }),
            PageState::More(QueryCursor { basis: 1, token: 1 }),
        ),
        Err(QueryError::Budget),
    ]);
    assert!(
        run(&mut controller, &squad_profile(), &mut queries).is_empty(),
        "an incomplete memory prefix cannot choose a scout destination"
    );
}
