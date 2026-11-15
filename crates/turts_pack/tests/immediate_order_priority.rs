//! A grouped ordinary order must not choose a visible funding winner by allocated unit ID.
use std::path::Path;

use turts_mod_api::{
    BalanceAmount, ContentId, CostSource, PaymentPolicy, Placement, ResourceDef, ResourceKind,
    SlotEconomyInit, SourceDef, WorldPos,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    Command, CommandKind, OrderCommand, OrderPolicyAudit, OrderTargetSim, QueueMode, RejectReason,
    SlotId,
};

fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("cedar:{local}")).unwrap()
}

#[derive(Debug, PartialEq)]
struct Outcome {
    producer_ids_by_position: (u32, u32),
    winning_source_x: f32,
    spawned_output_x: f32,
}

fn placement(local: &str, owner_slot: u8, x: f32) -> Placement {
    Placement {
        unit: id(local).to_string(),
        owner_slot,
        pos: WorldPos { x, y: 8.0 },
        contents: vec![],
        facing: None,
        start_order: None,
    }
}
fn prepare(
    payment: PaymentPolicy,
    construction: bool,
    placements: Vec<Placement>,
) -> bevy_app::App {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
    let mut catalog = archive.content().unwrap();
    catalog.resources.insert(
        id("credits"),
        ResourceDef {
            id: "credits".into(),
            name: "Credits".into(),
            unit_name: "credit".into(),
            kind: ResourceKind::Balance,
        },
    );
    let recipe = catalog.production.get_mut(&id("hire")).unwrap();
    recipe.duration_ms = 50;
    recipe.payment = payment;
    recipe.costs[0].source = CostSource::SlotBalance;
    recipe.costs[0].resource = id("credits");
    recipe.costs[0].amount = 12;
    if construction {
        use turts_mod_api::{
            Behaviour, ConstructProgress, Ratio, ResolvedConstructor, SitePlacementDef, TargetKind,
        };
        catalog.units.get_mut(&id("depot")).unwrap().constructor = Some(ResolvedConstructor {
            buildings: vec![id("hire")],
            concurrent_sites: 8,
            queue_capacity: 8,
            progress: ConstructProgress::Autonomous,
            when_short: vec![],
        });
        catalog.units.get_mut(&id("worker")).unwrap().placement = Some(SitePlacementDef {
            gap: 0.0,
            node_clearance: 0.0,
            in_build_area: false,
            site_health: Ratio {
                numerator: 1,
                denominator: 1,
            },
        });
        let mut order = catalog.orders[&id("order_produce")].clone();
        order.id = "order_construct".into();
        order.behaviour = Behaviour::Construct;
        order.target = TargetKind::ContentAtPoint;
        catalog.orders.insert(id("order_construct"), order);
    }
    let mut map = archive.maps().unwrap().remove(0).1;
    map.placements = placements;
    map.nodes.clear();
    map.slot_economy = vec![SlotEconomyInit {
        slot: 0,
        balances: vec![BalanceAmount {
            balance: id("credits").to_string(),
            amount: 12,
        }],
    }];
    let mut app = turts_sim::headless_app(20);
    let staged = turts_sim::stage_package(
        app.world(),
        catalog,
        SourceDef {
            file: "immediate-order-priority/scene.ron".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    turts_sim::install_package(app.world_mut(), staged);
    app.insert_resource(turts_sim::economy::construction::ConstructionLimit(8));
    app
}

fn issue(app: &mut bevy_app::App, units: Vec<u32>, construction: bool) -> turts_sim::CommandResult {
    let before = turts_sim::audit_state(app.world_mut());
    let definition = before
        .catalog
        .as_ref()
        .unwrap()
        .order_definitions
        .iter()
        .find(|order| {
            order.id
                == id(if construction {
                    "order_construct"
                } else {
                    "order_produce"
                })
        })
        .unwrap();
    turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: definition.id.clone(),
                behaviour: definition.behaviour,
                target_kind: definition.target,
                policy: Some(OrderPolicyAudit {
                    id: definition.id.clone(),
                    behaviour: definition.behaviour,
                    target: definition.target,
                    fire: definition.fire,
                    engage: definition.engage,
                    units: definition.units.clone(),
                    queue_depth: definition.queue_depth,
                    unit_capabilities: definition.unit_capabilities.clone(),
                }),
                units,
                target: if construction {
                    OrderTargetSim::ContentAtPoint {
                        content: id("hire"),
                        point: turts_sim::Vec2::new(24.0, 24.0),
                    }
                } else {
                    OrderTargetSim::Content(id("hire"))
                },
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: definition.units.clone(),
                queue_depth: definition.queue_depth,
            })),
        },
    )
}
fn run_scene(reverse_placements: bool) -> Outcome {
    run_scene_policy(reverse_placements, PaymentPolicy::ChargeOnQueue)
}
fn run_scene_policy(reverse_placements: bool, payment: PaymentPolicy) -> Outcome {
    let mut placements = vec![placement("depot", 0, 8.0), placement("depot", 0, 40.0)];
    if reverse_placements {
        placements.reverse();
    }
    let mut app = prepare(payment, false, placements);
    let before = turts_sim::audit_state(app.world_mut());
    let source_at = |x| {
        before
            .raw_units
            .iter()
            .find(|unit| unit.pos.x == x)
            .unwrap()
            .unit_id
    };
    let (left, right) = (source_at(8.0), source_at(40.0));
    assert_eq!(
        left < right,
        !reverse_placements,
        "only real placement allocation inverts the IDs"
    );
    let result = issue(&mut app, vec![left, right], false);
    assert!(result.outcome.is_ok());
    assert_eq!(
        result.units.accepted.len(),
        1,
        "shared funds can buy exactly one job"
    );
    assert_eq!(result.units.refused.len(), 1);
    assert_eq!(result.units.refused[0].1, RejectReason::InsufficientBalance);
    let winner = result.units.accepted[0];
    let winning_source_x = before
        .raw_units
        .iter()
        .find(|unit| unit.unit_id == winner)
        .unwrap()
        .pos
        .x;
    let after = turts_sim::world_view(app.world_mut());
    assert_eq!(
        after
            .balances
            .iter()
            .find(|balance| balance.resource == id("credits"))
            .unwrap()
            .on_hand,
        if payment == PaymentPolicy::ReserveThenCharge {
            12
        } else {
            0
        }
    );
    assert_eq!(
        after
            .queues
            .iter()
            .map(|queue| queue.jobs.len())
            .sum::<usize>(),
        1
    );
    for _ in 0..4 {
        app.update();
    }
    let completed = turts_sim::completed_world(app.world());
    let outputs = completed
        .units
        .iter()
        .filter(|unit| unit.content_id == id("worker").to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        outputs.len(),
        1,
        "the funded producer must really create its unit"
    );
    Outcome {
        producer_ids_by_position: (left, right),
        winning_source_x,
        spawned_output_x: outputs[0].pos.x,
    }
}

#[test]
fn grouped_production_with_scarce_shared_funds_is_invariant_under_placement_id_inversion() {
    let first = run_scene(false);
    let reversed = run_scene(true);
    assert_ne!(
        first.producer_ids_by_position,
        reversed.producer_ids_by_position
    );
    assert_eq!(first.winning_source_x, first.spawned_output_x);
    assert_eq!(reversed.winning_source_x, reversed.spawned_output_x);
    assert_eq!(
        (first.winning_source_x, first.spawned_output_x),
        (reversed.winning_source_x, reversed.spawned_output_x),
        "numeric IDs must not choose the visible producer or output location; normal placements gave {first:?}, reversed placements gave {reversed:?}"
    );
}

#[test]
fn reserved_shared_funds_also_follow_physical_sources() {
    let first = run_scene_policy(false, PaymentPolicy::ReserveThenCharge);
    let reversed = run_scene_policy(true, PaymentPolicy::ReserveThenCharge);
    assert_ne!(
        first.producer_ids_by_position,
        reversed.producer_ids_by_position
    );
    assert_eq!(
        (first.winning_source_x, first.spawned_output_x),
        (reversed.winning_source_x, reversed.spawned_output_x)
    );
}
#[test]
fn exact_source_ties_refuse_every_eligible_producer_without_spending() {
    let mut app = prepare(
        PaymentPolicy::ChargeOnQueue,
        false,
        vec![placement("depot", 0, 8.0), placement("depot", 0, 8.0)],
    );
    let before = turts_sim::audit_state(app.world_mut());
    let subjects = before
        .raw_units
        .iter()
        .map(|unit| unit.unit_id)
        .collect::<Vec<_>>();
    let result = issue(&mut app, subjects, false);
    assert!(result.units.accepted.is_empty());
    assert_eq!(result.units.refused.len(), 2);
    assert!(
        result
            .units
            .refused
            .iter()
            .all(|(_, reason)| *reason == RejectReason::Blocked)
    );
    let after = turts_sim::audit_state(app.world_mut());
    assert_eq!(before.allocators, after.allocators);
    assert_eq!(before.raw_queues, after.raw_queues);
    assert_eq!(before.economy.balances, after.economy.balances);
}
#[test]
fn foreign_and_incapable_coincident_members_do_not_suppress_a_valid_producer() {
    let mut app = prepare(
        PaymentPolicy::ChargeOnQueue,
        false,
        vec![
            placement("depot", 1, 8.0),
            placement("worker", 0, 8.0),
            placement("depot", 0, 8.0),
        ],
    );
    let before = turts_sim::audit_state(app.world_mut());
    let own = before
        .raw_units
        .iter()
        .find(|unit| {
            unit.owner_slot == 0
                && before.unit_types.iter().any(|kind| {
                    kind.type_id == unit.unit_type && kind.content_id == id("depot").to_string()
                })
        })
        .unwrap()
        .unit_id;
    let result = issue(
        &mut app,
        before.raw_units.iter().map(|unit| unit.unit_id).collect(),
        false,
    );
    assert_eq!(result.units.accepted, vec![own]);
    assert_eq!(
        result
            .units
            .refused
            .iter()
            .map(|(_, reason)| *reason)
            .collect::<Vec<_>>(),
        vec![RejectReason::NotOwner, RejectReason::MissingCapability]
    );
}
#[test]
fn grouped_construction_placement_and_payment_follow_physical_builder_priority() {
    let mut winners = vec![];
    for reverse in [false, true] {
        let mut placements = vec![placement("depot", 0, 8.0), placement("depot", 0, 40.0)];
        if reverse {
            placements.reverse();
        }
        let mut app = prepare(PaymentPolicy::ChargeOnQueue, true, placements);
        let before = turts_sim::audit_state(app.world_mut());
        let mut subjects = before
            .raw_units
            .iter()
            .map(|unit| unit.unit_id)
            .collect::<Vec<_>>();
        subjects.reverse();
        let result = issue(&mut app, subjects, true);
        assert_eq!(result.units.accepted.len(), 1);
        assert_eq!(result.units.refused.len(), 1);
        let after = turts_sim::audit_state(app.world_mut());
        assert_eq!(after.economy.construction.sites.len(), 1);
        let site = after.economy.construction.sites.values().next().unwrap();
        assert_eq!(site.constructor, result.units.accepted[0]);
        assert_eq!(result.outcome.unwrap().job_id, Some(site.job));
        assert_eq!(
            after
                .economy
                .balances
                .iter()
                .find(|balance| balance.resource == id("credits"))
                .unwrap()
                .on_hand,
            0
        );
        winners.push(
            before
                .raw_units
                .iter()
                .find(|unit| unit.unit_id == site.constructor)
                .unwrap()
                .pos
                .x,
        );
    }
    assert_eq!(winners, vec![8.0, 8.0]);
}
