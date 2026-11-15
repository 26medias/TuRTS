//! An external host composes and replaces controllers through the public Rust API only.
use std::sync::Arc;
use turts_bot::*;
use turts_mod_api::{
    BotControllerDef, BotDef, BotKnowledge, BotQuota, BotStartWhen, BotValue, ContentId, WorldPos,
};
fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("external:{local}")).unwrap()
}
fn definition(key: &str, factory: &str) -> BotControllerDef {
    BotControllerDef {
        key: key.into(),
        factory: format!("external:{factory}"),
        config: BotValue::Null,
        quota: BotQuota {
            queries: 1,
            candidates: 1,
            rows: 1,
            proposals: 1,
        },
    }
}
fn profile() -> BotDef {
    BotDef {
        id: "profile".into(),
        name: "External profile".into(),
        decision_ms: 1000,
        start_when: BotStartWhen::MatchRunning,
        knowledge: BotKnowledge::default(),
        controllers: vec![
            definition("economy", "counter"),
            definition("tactics", "counter"),
        ],
    }
}
struct Counter {
    key: String,
    calls: u32,
}
impl BotController for Counter {
    fn step(&mut self, frame: &BotFrame<'_>, _: &mut dyn BotQueries, out: &mut dyn ProposalSink) {
        self.calls += 1;
        out.propose(Proposal {
            source_basis: frame.basis,
            task: TaskKey::new(&self.key, "external-goal", self.calls.to_string()),
            actors: vec![1],
            order: BotOrder {
                content: id("move"),
                target: BotTarget::Point(WorldPos { x: 1., y: 2. }),
                queue: BotQueueMode::Replace,
                group: 0,
            },
            priority: 1,
            goal_order: 0,
        })
        .unwrap();
    }
}
struct Factory;
impl ControllerFactory for Factory {
    fn validate(&self, _: &BotDef, _: &BotControllerDef) -> Result<(), FactoryError> {
        Ok(())
    }
    fn create(&self, init: ControllerInit<'_>) -> Result<Box<dyn BotController>, FactoryError> {
        Ok(Box::new(Counter {
            key: init.instance.key.clone(),
            calls: 0,
        }))
    }
}
struct Empty;
impl BotController for Empty {
    fn step(&mut self, _: &BotFrame<'_>, _: &mut dyn BotQueries, _: &mut dyn ProposalSink) {}
}
struct EmptyFactory;
impl ControllerFactory for EmptyFactory {
    fn validate(&self, _: &BotDef, _: &BotControllerDef) -> Result<(), FactoryError> {
        Ok(())
    }
    fn create(&self, _: ControllerInit<'_>) -> Result<Box<dyn BotController>, FactoryError> {
        Ok(Box::new(Empty))
    }
}
struct Queries;
impl BotQueries for Queries {
    fn pin_basis(&mut self, _: BasisId) -> Result<(), QueryError> {
        Ok(())
    }
    fn release_basis(&mut self, _: BasisId) -> Result<(), QueryError> {
        Ok(())
    }
    fn query(&mut self, _: Query, _: Option<QueryCursor>, _: u32) -> Result<QueryPage, QueryError> {
        Err(QueryError::InvalidQuery)
    }
}
fn step(instance: &mut ControllerInstance, profile: &BotDef, actor: &BotActor) -> Vec<Proposal> {
    let accounting = ReconcileBatch {
        watermark: Watermark {
            completed_tick: 0,
            revision: 0,
        },
        accounts: vec![],
        included: vec![],
        unapplied: vec![],
        complete: true,
    };
    let frame = BotFrame {
        basis: 0,
        accounting_basis: 0,
        completed_tick: 0,
        scheduler_tick: 0,
        active_ticks: 0,
        actor,
        tick_hz: 20,
        phase: MatchPhase::Running,
        profile,
        quota: instance.quota,
        limits: CoordinatorLimits::default(),
        reconciliation: &accounting,
        role_counts: &[],
        supply: &[],
    };
    let mut sink = ProposalBuffer::new(&instance.key, 1, 1);
    instance.controller.step(&frame, &mut Queries, &mut sink);
    sink.into_proposals()
}
#[test]
fn an_external_host_composes_fresh_controllers_and_replaces_the_strategy() {
    let mut registry = ControllerRegistry::new();
    registry.register(id("counter"), Arc::new(Factory)).unwrap();
    registry
        .register(id("empty"), Arc::new(EmptyFactory))
        .unwrap();
    let actor = BotActor {
        slot: 0,
        profile: id("profile"),
    };
    let mut profile = profile();
    let limit = BotQuota {
        queries: 2,
        candidates: 2,
        rows: 2,
        proposals: 2,
    };
    let mut first = registry.activate(&actor, &profile, 7, limit).unwrap();
    let mut second = registry.activate(&actor, &profile, 7, limit).unwrap();
    assert_eq!(step(&mut first[0], &profile, &actor)[0].task.instance, "1");
    assert_eq!(step(&mut first[0], &profile, &actor)[0].task.instance, "2");
    assert_eq!(step(&mut first[1], &profile, &actor)[0].task.instance, "1");
    assert_eq!(step(&mut second[0], &profile, &actor)[0].task.instance, "1");
    let original_seeds = first
        .iter()
        .map(|instance| (instance.key.clone(), instance.seed))
        .collect::<std::collections::BTreeMap<_, _>>();
    profile.controllers.reverse();
    let reordered = registry.activate(&actor, &profile, 7, limit).unwrap();
    assert_eq!(
        original_seeds,
        reordered
            .iter()
            .map(|instance| (instance.key.clone(), instance.seed))
            .collect()
    );
    profile.controllers = vec![definition("replacement", "empty")];
    let mut replacement = registry.activate(&actor, &profile, 7, limit).unwrap();
    assert!(step(&mut replacement[0], &profile, &actor).is_empty());
}
struct NeverCreate;
impl ControllerFactory for NeverCreate {
    fn validate(&self, _: &BotDef, _: &BotControllerDef) -> Result<(), FactoryError> {
        Ok(())
    }
    fn create(&self, _: ControllerInit<'_>) -> Result<Box<dyn BotController>, FactoryError> {
        panic!("activation created state before validating the complete profile")
    }
}
#[test]
fn unknown_later_factory_refuses_before_any_controller_state_is_created() {
    let mut registry = ControllerRegistry::new();
    registry
        .register(id("counter"), Arc::new(NeverCreate))
        .unwrap();
    let actor = BotActor {
        slot: 0,
        profile: id("profile"),
    };
    let mut profile = profile();
    profile.controllers[1].factory = "external:unavailable".into();
    let limit = BotQuota {
        queries: 2,
        candidates: 2,
        rows: 2,
        proposals: 2,
    };
    let error = match registry.activate(&actor, &profile, 7, limit) {
        Ok(_) => panic!("unknown host factory was accepted"),
        Err(error) => error,
    };
    assert_eq!(error.field_path, "controllers[1].factory");
}
#[test]
fn overflowing_authored_quotas_fail_instead_of_wrapping_to_available_budget() {
    let mut registry = ControllerRegistry::new();
    registry
        .register(id("counter"), Arc::new(NeverCreate))
        .unwrap();
    let actor = BotActor {
        slot: 0,
        profile: id("profile"),
    };
    let mut profile = profile();
    profile.controllers[0].quota.queries = u32::MAX;
    let limit = BotQuota {
        queries: u32::MAX,
        candidates: 2,
        rows: 2,
        proposals: 2,
    };
    let error = match registry.activate(&actor, &profile, 7, limit) {
        Ok(_) => panic!("overflowing quota accepted"),
        Err(error) => error,
    };
    assert_eq!(error.field_path, "controllers[1].quota.queries");
}
