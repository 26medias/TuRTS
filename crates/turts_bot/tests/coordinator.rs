//! These checks use the public framework as an external host/controller would.
use std::collections::BTreeMap;
use turts_bot::*;
use turts_mod_api::{ContentId, PaymentPolicy, WorldPos};
fn cid(name: &str) -> ContentId {
    ContentId::parse(&format!("test:{name}")).unwrap()
}
fn actor() -> BotActor {
    BotActor {
        slot: 0,
        profile: cid("policy"),
    }
}
fn mark(tick: u32) -> Watermark {
    Watermark {
        completed_tick: tick,
        revision: 0,
    }
}
fn task(key: &str) -> TaskKey {
    TaskKey::new("controller", key, "0")
}
fn money() -> SpendKey {
    SpendKey {
        endpoint: SourceEndpoint::SlotBalance(0),
        resource: cid("credits"),
    }
}
fn inventory(unit: u32) -> SpendKey {
    SpendKey {
        endpoint: SourceEndpoint::Inventory {
            unit,
            port: InventoryPort::Main,
        },
        resource: cid("ore"),
    }
}
fn amount(account: SpendKey, amount: u64) -> AccountAmount {
    AccountAmount { account, amount }
}
fn balance(account: SpendKey, on_hand: u64, reserved: u64) -> AccountObservation {
    AccountObservation {
        account,
        on_hand,
        reserved,
    }
}
fn proposal(key: &str, actors: Vec<u32>) -> Proposal {
    Proposal {
        source_basis: 0,
        task: task(key),
        actors,
        order: BotOrder {
            content: cid("produce"),
            target: BotTarget::Content(cid("train")),
            queue: BotQueueMode::Replace,
            group: 0,
        },
        priority: 1,
        goal_order: 0,
    }
}
struct Verifier(BTreeMap<u32, VerifiedMember>);
impl ProposalVerifier for Verifier {
    fn verify(
        &self,
        _: BasisId,
        proposal: &Proposal,
    ) -> Result<VerifiedProposal, ProposalRejection> {
        Ok(VerifiedProposal {
            fulfillment: Vec::new(),
            members: proposal
                .actors
                .iter()
                .map(|id| {
                    self.0
                        .get(id)
                        .cloned()
                        .ok_or(ProposalRejection::UnknownActor)
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}
fn member(unit: u32, x: f32, costs: Vec<AccountAmount>) -> VerifiedMember {
    VerifiedMember {
        unit,
        anchor: WorldPos { x, y: 4. },
        costs,
        payment: Some(PaymentPolicy::ChargeOverTime),
    }
}
fn verifier(members: Vec<VerifiedMember>) -> Verifier {
    Verifier(members.into_iter().map(|m| (m.unit, m)).collect())
}
fn batch(
    tick: u32,
    accounts: Vec<AccountObservation>,
    included: Vec<ObservedRequestEffect>,
    unapplied: Vec<RequestKey>,
) -> ReconcileBatch {
    ReconcileBatch {
        watermark: mark(tick),
        accounts,
        included,
        unapplied,
        complete: true,
    }
}
fn ready(accounts: Vec<AccountObservation>) -> BotCoordinator {
    let mut c = BotCoordinator::new(actor(), CoordinatorLimits::default());
    c.reconcile(0, &batch(0, accounts, vec![], vec![])).unwrap();
    c
}
fn request(sequence: u32, unit: u32) -> RequestKey {
    RequestKey {
        actor: actor(),
        sequence,
        unit,
    }
}
fn effect(
    key: RequestKey,
    paid: Vec<AccountAmount>,
    done: u64,
    terminal: bool,
) -> ObservedRequestEffect {
    let work = WorkObservation {
        identity: WorkIdentity {
            unit: key.unit,
            id: 70 + u64::from(key.unit),
            kind: WorkKind::Production,
        },
        actor_unit: key.unit,
        content: Some(cid("train")),
        payment: Some(PaymentPolicy::ChargeOverTime),
        done,
        total: 10,
        paid: paid.clone(),
        reserved: vec![],
        state: if terminal {
            WorkState::Finished
        } else {
            WorkState::Running
        },
    };
    ObservedRequestEffect {
        request: key,
        effect_watermark: mark(1),
        refused: false,
        pending_placement: false,
        paid,
        reserved: vec![],
        work: Some(work),
        terminal,
    }
}
#[test]
fn separate_inventory_sources_can_fund_one_group_without_collapsing_their_item_keys() {
    let mut c = ready(vec![
        balance(inventory(1), 10, 0),
        balance(inventory(2), 10, 0),
    ]);
    let v = verifier(vec![
        member(1, 8., vec![amount(inventory(1), 10)]),
        member(2, 30., vec![amount(inventory(2), 10)]),
    ]);
    let out = c.select(0, vec![proposal("train", vec![2, 1])], &v);
    assert_eq!(out.commands.len(), 1);
    assert!(out.refused.is_empty());
    assert_eq!(out.commands[0].units, vec![1, 2]);
    assert_eq!(c.pending().len(), 2);
}
#[test]
fn an_early_result_never_releases_old_funds_and_a_new_basis_never_counts_debits_twice() {
    let mut c = ready(vec![balance(money(), 20, 0)]);
    let v = verifier(vec![
        member(1, 8., vec![amount(money(), 10)]),
        member(2, 30., vec![amount(money(), 11)]),
    ]);
    assert_eq!(
        c.select(0, vec![proposal("first", vec![1])], &v)
            .commands
            .len(),
        1
    );
    c.observe_result(ActorResult {
        request: request(0, 1),
        effect_watermark: mark(1),
        accepted: true,
        work: Some(WorkIdentity {
            unit: 1,
            id: 71,
            kind: WorkKind::Production,
        }),
        reason: None,
    })
    .unwrap();
    c.reconcile(
        0,
        &batch(
            0,
            vec![balance(money(), 20, 0)],
            vec![],
            vec![request(0, 1)],
        ),
    )
    .unwrap();
    assert!(matches!(
        c.select(0, vec![proposal("second", vec![2])], &v).refused[0].reason,
        ProposalRejection::InsufficientResources(_)
    ));
    c.reconcile(
        1,
        &batch(
            1,
            vec![balance(money(), 16, 0)],
            vec![effect(request(0, 1), vec![amount(money(), 4)], 4, false)],
            vec![],
        ),
    )
    .unwrap();
    assert_eq!(c.pending()[0].unpaid, vec![amount(money(), 6)]);
    let v = verifier(vec![member(2, 30., vec![amount(money(), 10)])]);
    assert_eq!(
        c.select(1, vec![proposal("second", vec![2])], &v)
            .commands
            .len(),
        1,
        "16 observed minus6 unpaid leaves10, not6"
    );
}
#[test]
fn basis_before_feedback_reconciles_partial_group_members_and_keeps_the_other_inventory_independent()
 {
    let mut c = ready(vec![
        balance(inventory(1), 10, 0),
        balance(inventory(2), 10, 0),
    ]);
    let v = verifier(vec![
        member(1, 8., vec![amount(inventory(1), 10)]),
        member(2, 30., vec![amount(inventory(2), 10)]),
    ]);
    c.select(0, vec![proposal("group", vec![1, 2])], &v);
    let refused = ObservedRequestEffect {
        request: request(0, 2),
        effect_watermark: mark(1),
        refused: true,
        pending_placement: false,
        paid: vec![],
        reserved: vec![],
        work: None,
        terminal: true,
    };
    let report = c
        .reconcile(
            1,
            &batch(
                1,
                vec![balance(inventory(1), 6, 0), balance(inventory(2), 10, 0)],
                vec![
                    effect(request(0, 1), vec![amount(inventory(1), 4)], 4, false),
                    refused,
                ],
                vec![],
            ),
        )
        .unwrap();
    assert_eq!(report.retired, vec![request(0, 2)]);
    assert_eq!(c.pending().len(), 1);
    c.observe_result(ActorResult {
        request: request(0, 2),
        effect_watermark: mark(1),
        accepted: false,
        work: None,
        reason: Some("blocked".into()),
    })
    .unwrap();
    assert_eq!(
        c.select(1, vec![proposal("retry", vec![2])], &v)
            .commands
            .len(),
        1
    );
}
#[test]
fn an_incomplete_or_missing_effect_batch_advances_neither_accounts_nor_commitments() {
    let mut c = ready(vec![balance(money(), 20, 0)]);
    let v = verifier(vec![member(1, 8., vec![amount(money(), 10)])]);
    c.select(0, vec![proposal("first", vec![1])], &v);
    let old = c.pending();
    assert_eq!(
        c.reconcile(1, &batch(1, vec![balance(money(), 16, 0)], vec![], vec![])),
        Err(ReconcileError::Incomplete)
    );
    assert_eq!(c.pending(), old);
    assert_eq!(
        c.select(0, vec![proposal("unsafe", vec![1])], &v).refused[0].reason,
        ProposalRejection::AccountingUnresolved
    );
    let bad = effect(request(0, 1), vec![amount(money(), 5)], 4, false);
    assert_eq!(
        c.reconcile(
            1,
            &batch(1, vec![balance(money(), 15, 0)], vec![bad], vec![])
        ),
        Err(ReconcileError::InvalidEffect)
    );
    assert_eq!(c.pending(), old);
    c.reconcile(
        1,
        &batch(
            1,
            vec![balance(money(), 16, 0)],
            vec![effect(request(0, 1), vec![amount(money(), 4)], 4, false)],
            vec![],
        ),
    )
    .unwrap();
}
#[test]
fn held_placement_keeps_its_obligation_until_the_actual_site_is_observed() {
    let mut c = ready(vec![balance(money(), 20, 0)]);
    let v = verifier(vec![member(1, 8., vec![amount(money(), 10)])]);
    c.select(0, vec![proposal("build", vec![1])], &v);
    let held = ObservedRequestEffect {
        request: request(0, 1),
        effect_watermark: mark(1),
        refused: false,
        pending_placement: true,
        paid: vec![],
        reserved: vec![],
        work: None,
        terminal: false,
    };
    c.reconcile(
        1,
        &batch(1, vec![balance(money(), 20, 0)], vec![held], vec![]),
    )
    .unwrap();
    assert_eq!(c.pending()[0].unpaid, vec![amount(money(), 10)]);
    let mut site = effect(request(0, 1), vec![amount(money(), 2)], 2, false);
    let work = site.work.as_mut().unwrap();
    work.identity = WorkIdentity {
        unit: 99,
        id: 71,
        kind: WorkKind::Construction,
    };
    c.reconcile(
        2,
        &batch(2, vec![balance(money(), 18, 0)], vec![site], vec![]),
    )
    .unwrap();
    assert_eq!(c.pending()[0].unpaid, vec![amount(money(), 8)]);
    assert_eq!(c.pending()[0].work.as_ref().unwrap().unit, 99);
}
#[test]
fn authoritative_reservations_replace_the_local_commitment_instead_of_subtracting_both() {
    let mut c = ready(vec![balance(money(), 20, 0)]);
    let mut first = member(1, 8., vec![amount(money(), 10)]);
    first.payment = Some(PaymentPolicy::ReserveThenCharge);
    let v = verifier(vec![first, member(2, 30., vec![amount(money(), 10)])]);
    c.select(0, vec![proposal("reserved", vec![1])], &v);
    let mut observed = effect(request(0, 1), vec![], 0, false);
    observed.reserved = vec![amount(money(), 10)];
    let work = observed.work.as_mut().unwrap();
    work.payment = Some(PaymentPolicy::ReserveThenCharge);
    work.reserved = observed.reserved.clone();
    c.reconcile(
        1,
        &batch(1, vec![balance(money(), 20, 10)], vec![observed], vec![]),
    )
    .unwrap();
    assert!(c.pending().is_empty());
    assert_eq!(
        c.select(1, vec![proposal("next", vec![2])], &v)
            .commands
            .len(),
        1
    );
}
#[test]
fn positive_and_negative_zero_are_one_physical_rank_and_refuse_both() {
    let mut c = ready(vec![]);
    let v = verifier(vec![member(1, -0., vec![]), member(900, 0., vec![])]);
    let out = c.select(
        0,
        vec![proposal("a", vec![1]), proposal("b", vec![900])],
        &v,
    );
    assert!(out.commands.is_empty());
    assert_eq!(out.refused.len(), 2);
    assert!(
        out.refused
            .iter()
            .all(|row| row.reason == ProposalRejection::EqualRank)
    );
    assert!(c.pending().is_empty());
}
#[test]
fn physical_priority_ignores_identifiers_and_sequence_exhaustion_never_wraps() {
    for (near, far) in [(1, 900), (900, 1)] {
        let mut c =
            BotCoordinator::with_sequence(actor(), CoordinatorLimits::default(), Some(u32::MAX));
        c.reconcile(0, &batch(0, vec![], vec![], vec![])).unwrap();
        let v = verifier(vec![member(near, 8., vec![]), member(far, 30., vec![])]);
        let out = c.select(
            0,
            vec![proposal("far", vec![far]), proposal("near", vec![near])],
            &v,
        );
        assert_eq!(out.commands.len(), 1);
        assert_eq!(out.commands[0].sequence, u32::MAX);
        assert_eq!(out.commands[0].units, vec![near]);
        assert_eq!(out.refused[0].reason, ProposalRejection::SequenceExhausted);
        assert_eq!(c.next_sequence(), None);
    }
}
#[test]
fn malformed_verified_members_have_no_partial_pending_or_sequence_writes() {
    let mut c = ready(vec![balance(money(), 20, 0)]);
    let mut bad = member(1, f32::NAN, vec![amount(money(), 10)]);
    let v = verifier(vec![bad.clone()]);
    assert_eq!(
        c.select(0, vec![proposal("bad", vec![1])], &v).refused[0].reason,
        ProposalRejection::InvalidAnchor
    );
    bad.anchor.x = 8.;
    bad.costs[0].account.endpoint = SourceEndpoint::SlotBalance(1);
    let v = verifier(vec![bad]);
    assert_eq!(
        c.select(0, vec![proposal("bad", vec![1])], &v).refused[0].reason,
        ProposalRejection::InvalidCosts
    );
    assert_eq!(c.next_sequence(), Some(0));
    assert!(c.pending().is_empty());
}

#[test]
fn the_pending_account_row_limit_counts_each_members_obligation_not_only_unique_resources() {
    let mut limits = CoordinatorLimits::default();
    limits.pending_accounts = 1;
    let mut c = BotCoordinator::new(actor(), limits);
    c.reconcile(0, &batch(0, vec![balance(money(), 20, 0)], vec![], vec![]))
        .unwrap();
    let v = verifier(vec![
        member(1, 8., vec![amount(money(), 10)]),
        member(2, 30., vec![amount(money(), 10)]),
    ]);
    let out = c.select(0, vec![proposal("group", vec![1, 2])], &v);
    assert_eq!(out.refused[0].reason, ProposalRejection::Budget);
    assert!(c.tracked_requests().is_empty());
    assert_eq!(c.next_sequence(), Some(0));
}

struct ClaimsVerifier {
    members: Verifier,
    claims: BTreeMap<String, Vec<FulfillmentClaim>>,
}
impl ProposalVerifier for ClaimsVerifier {
    fn verify(
        &self,
        basis: BasisId,
        proposal: &Proposal,
    ) -> Result<VerifiedProposal, ProposalRejection> {
        let mut verified = self.members.verify(basis, proposal)?;
        verified.fulfillment = self
            .claims
            .get(&proposal.task.goal)
            .cloned()
            .unwrap_or_default();
        Ok(verified)
    }
}
fn role(name: &str, observed: u64, cap: Option<u64>) -> FulfillmentClaim {
    FulfillmentClaim {
        key: FulfillmentKey::Role(name.into()),
        observed,
        cap,
        contribution: 1,
    }
}
#[test]
fn opening_and_recovery_do_not_build_the_same_deficit_from_different_builders() {
    for reversed in [false, true] {
        let mut c = ready(vec![balance(money(), 100, 0)]);
        let v = ClaimsVerifier {
            members: verifier(vec![
                member(1, 8., vec![amount(money(), 10)]),
                member(900, 30., vec![amount(money(), 10)]),
            ]),
            claims: [
                ("opening".into(), vec![role("barracks", 0, Some(1))]),
                ("recovery".into(), vec![role("barracks", 0, Some(1))]),
            ]
            .into(),
        };
        let mut opening = proposal("opening", vec![1]);
        opening.task.controller = "opening-controller".into();
        opening.priority = 10;
        let mut recovery = proposal("recovery", vec![900]);
        recovery.task.controller = "recovery-controller".into();
        recovery.priority = 100;
        let mut proposals = vec![opening, recovery];
        if reversed {
            proposals.reverse();
        }
        let out = c.select(0, proposals, &v);
        assert_eq!(out.commands.len(), 1);
        assert_eq!(out.commands[0].task.goal, "recovery");
        assert_eq!(
            out.refused[0].reason,
            ProposalRejection::FulfillmentSatisfied(FulfillmentKey::Role("barracks".into()))
        );
        assert_eq!(c.tracked_requests().len(), 1);
    }
}
#[test]
fn incidental_role_contributions_prevent_a_second_copy_for_an_overlapping_role() {
    let mut c = ready(vec![balance(money(), 100, 0)]);
    let v = ClaimsVerifier {
        members: verifier(vec![
            member(1, 8., vec![amount(money(), 10)]),
            member(2, 30., vec![amount(money(), 10)]),
        ]),
        claims: [
            (
                "workers".into(),
                vec![role("worker", 0, Some(1)), role("builder", 0, None)],
            ),
            (
                "builders".into(),
                vec![role("worker", 0, None), role("builder", 0, Some(1))],
            ),
        ]
        .into(),
    };
    let out = c.select(
        0,
        vec![proposal("builders", vec![2]), proposal("workers", vec![1])],
        &v,
    );
    assert_eq!(out.commands.len(), 1);
    assert_eq!(out.commands[0].task.goal, "workers");
    assert_eq!(
        out.refused[0].reason,
        ProposalRejection::FulfillmentSatisfied(FulfillmentKey::Role("builder".into()))
    );
}
#[test]
fn incidental_roles_have_no_implicit_cap_and_pending_unapplied_work_still_consumes_its_goal() {
    let mut c = ready(vec![balance(money(), 100, 0)]);
    let v = ClaimsVerifier {
        members: verifier(vec![
            member(1, 8., vec![amount(money(), 10)]),
            member(2, 30., vec![amount(money(), 10)]),
        ]),
        claims: [
            (
                "first".into(),
                vec![role("worker", 0, Some(1)), role("civilian", 5, None)],
            ),
            (
                "second".into(),
                vec![role("worker", 0, Some(1)), role("civilian", 5, None)],
            ),
        ]
        .into(),
    };
    assert_eq!(
        c.select(0, vec![proposal("first", vec![1])], &v)
            .commands
            .len(),
        1
    );
    c.reconcile(
        0,
        &batch(
            0,
            vec![balance(money(), 100, 0)],
            vec![],
            vec![request(0, 1)],
        ),
    )
    .unwrap();
    let out = c.select(0, vec![proposal("second", vec![2])], &v);
    assert!(out.commands.is_empty());
    assert_eq!(
        out.refused[0].reason,
        ProposalRejection::FulfillmentSatisfied(FulfillmentKey::Role("worker".into()))
    );
}
#[test]
fn inconsistent_observed_fulfillment_refuses_both_baselines_instead_of_preferring_first() {
    let mut c = ready(vec![]);
    let v = ClaimsVerifier {
        members: verifier(vec![member(1, 8., vec![]), member(2, 30., vec![])]),
        claims: [
            ("a".into(), vec![role("worker", 0, Some(2))]),
            ("b".into(), vec![role("worker", 1, Some(2))]),
        ]
        .into(),
    };
    let out = c.select(0, vec![proposal("a", vec![1]), proposal("b", vec![2])], &v);
    assert!(out.commands.is_empty());
    assert_eq!(out.refused.len(), 2);
    assert!(
        out.refused
            .iter()
            .all(|row| matches!(row.reason, ProposalRejection::InconsistentFulfillment(_)))
    );
    assert!(c.tracked_requests().is_empty());
    assert_eq!(c.next_sequence(), Some(0));
}
#[test]
fn fulfillment_arithmetic_overflow_has_no_partial_allocation() {
    let mut c = ready(vec![]);
    let v = ClaimsVerifier {
        members: verifier(vec![member(1, 8., vec![])]),
        claims: [("a".into(), vec![role("worker", u64::MAX, None)])].into(),
    };
    let out = c.select(0, vec![proposal("a", vec![1])], &v);
    assert_eq!(out.refused[0].reason, ProposalRejection::ArithmeticOverflow);
    assert!(c.tracked_requests().is_empty());
    assert_eq!(c.next_sequence(), Some(0));
}
