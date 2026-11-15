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

fn run_scene(reverse_placements: bool) -> Outcome {
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
    recipe.payment = PaymentPolicy::ChargeOnQueue;
    recipe.costs[0].source = CostSource::SlotBalance;
    recipe.costs[0].resource = id("credits");
    recipe.costs[0].amount = 12;
    let mut map = archive.maps().unwrap().remove(0).1;
    map.placements = [8.0, 40.0]
        .into_iter()
        .map(|x| Placement {
            unit: id("depot").to_string(),
            owner_slot: 0,
            pos: WorldPos { x, y: 8.0 },
            contents: vec![],
            facing: None,
            start_order: None,
        })
        .collect();
    if reverse_placements {
        map.placements.reverse();
    }
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
    let definition = before
        .catalog
        .as_ref()
        .unwrap()
        .order_definitions
        .iter()
        .find(|order| order.id == id("order_produce"))
        .unwrap();
    let result = turts_sim::apply_command(
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
                // Keep the command's physical subject order identical in both cases.
                units: vec![left, right],
                target: OrderTargetSim::Content(id("hire")),
                queue: QueueMode::Replace,
                group: 0,
                allowed_units: definition.units.clone(),
                queue_depth: definition.queue_depth,
            })),
        },
    );
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
        0
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
