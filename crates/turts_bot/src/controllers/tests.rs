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
    let actor = BotActor {
        slot: 0,
        profile: id("fixture:opponent"),
    };
    let batch = ReconcileBatch {
        watermark: Watermark {
            completed_tick: 1,
            revision: 1,
        },
        accounts: vec![],
        included: vec![],
        complete: true,
    };
    let frame = BotFrame {
        basis: 1,
        accounting_basis: 1,
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
        role_counts: &[],
        supply: &[],
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
