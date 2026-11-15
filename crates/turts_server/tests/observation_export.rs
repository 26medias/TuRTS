//! Human wire projections share the bot observation permission boundary.
#[path = "../src/export.rs"]
mod export;
#[path = "../src/permissions.rs"]
mod permissions;

use permissions::{ObservationArea, ObservationPermissions};
use turts_mod_api::{ContentId, WorldPos};
use turts_sim::economy::{
    events::EffectRow,
    views::{
        NodeView, OrderView, UnitOrderActivityView, UnitOrderSummaryView, UnitOrderTargetView,
        UnitOrderView,
    },
    views_m3::{RawHealthAudit, RawWeaponAudit, RawWeaponStatusAudit},
};
use turts_sim::{CommittedTick, SlotId, WorldView, world::UnitState};

struct Restricted;
impl ObservationPermissions for Restricted {
    fn unit_visible(&self, _: u8, id: u32, _: u8) -> bool {
        id != 99
    }
    fn node_visible(&self, _: u8, id: u32) -> bool {
        id != 99
    }
    fn area_observed(&self, _: u8, _: ObservationArea) -> bool {
        false
    }
}
fn id(text: &str) -> ContentId {
    ContentId::parse(text).unwrap()
}
fn unit(unit_id: u32, owner_slot: u8) -> UnitState {
    UnitState {
        unit_id,
        owner_slot,
        content_id: "test:unit".into(),
        facing: None,
        pos: Default::default(),
    }
}
fn view(hidden: bool) -> WorldView {
    let mut v = WorldView {
        tick: 10,
        units: vec![unit(1, 0), unit(2, 1)],
        ..Default::default()
    };
    if hidden {
        v.units.push(unit(99, 1));
    }
    for n in [1, 99] {
        if n == 1 || hidden {
            v.nodes.push(NodeView {
                id: turts_sim::NodeId(n),
                content_id: id("test:node"),
                pos: WorldPos { x: 4.0, y: 4.0 },
                remaining: 500,
            });
        }
    }
    v.m3.raw_healths = v
        .units
        .iter()
        .map(|u| RawHealthAudit {
            unit: u.unit_id,
            current: 90,
        })
        .collect();
    v.m3.raw_weapons.push(RawWeaponAudit {
        unit: 1,
        enabled: true,
        cooldown: 0,
        effective_damage: Ok(5),
        status: RawWeaponStatusAudit::Ready,
        last_target: Some(99),
        ordered_target_fired: None,
    });
    v.economy.orders.push(OrderView {
        unit: 1,
        owner_slot: 0,
        node: Some(turts_sim::NodeId(99)),
        receiver: None,
        stage: turts_sim::economy::gather::GatherStage::Gathering,
    });
    v.unit_orders.push(UnitOrderView {
        unit_id: 1,
        owner_slot: 0,
        rally: None,
        activity: UnitOrderActivityView::Attacking,
        current: Some(UnitOrderSummaryView {
            order: "test:attack".into(),
            target: UnitOrderTargetView::Unit(99),
        }),
        queued: vec![UnitOrderSummaryView {
            order: "test:gather".into(),
            target: UnitOrderTargetView::Node(turts_sim::NodeId(99)),
        }],
        last_failure: None,
    });
    v.economy
        .repairs
        .push(turts_sim::economy::repair::RepairRow {
            repairer: 1,
            target: 99,
            owner_slot: 0,
            state: turts_sim::economy::repair::RepairStatus::Working,
        });
    v
}
fn bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    postcard::to_stdvec(value).unwrap()
}

#[test]
fn restricted_snapshot_hides_geometry_and_every_target_reference() {
    let (s, _) = export::snapshot_for_with_permissions(&view(true), SlotId(0), &Restricted);
    assert_eq!(
        s.units.iter().map(|u| u.unit_id.0).collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(s.nodes.iter().map(|n| n.id.0).collect::<Vec<_>>(), vec![1]);
    assert_eq!(s.healths.len(), 2);
    assert_eq!(s.weapons[0].last_target, None);
    assert_eq!(s.economy.orders[0].node, None);
    assert_eq!(s.unit_orders[0].current, None);
    assert!(s.unit_orders[0].queued.is_empty());
    assert!(s.records.repairs.is_empty());
    let (absent, _) = export::snapshot_for_with_permissions(&view(false), SlotId(0), &Restricted);
    assert_eq!(
        bytes(&s),
        bytes(&absent),
        "hidden presence cannot affect the transmitted snapshot"
    );
}

#[test]
fn own_units_remain_visible_even_when_the_policy_rejects_their_identifier() {
    let mut v = view(true);
    v.units[2].owner_slot = 0;
    let (s, _) = export::snapshot_for_with_permissions(&v, SlotId(0), &Restricted);
    assert!(s.units.iter().any(|u| u.unit_id.0 == 99));
    assert!(s.healths.iter().any(|u| u.unit.0 == 99));
    assert_eq!(
        s.weapons[0].last_target,
        Some(turts_net_protocol::UnitId(99))
    );
}

#[test]
fn restricted_delta_does_not_publish_hidden_changes_death_or_effect_metadata() {
    let before = view(true);
    let (_, history) = export::snapshot_for_with_permissions(&before, SlotId(0), &Restricted);
    let mut after = view(false);
    after.tick = 11;
    let batch = CommittedTick {
        tick: 11,
        removed: vec![99],
        positions: vec![(99, Default::default())],
        healths: vec![RawHealthAudit {
            unit: 99,
            current: 0,
        }],
        effects: vec![
            EffectRow::Destroyed { unit: 99 },
            EffectRow::Fired {
                unit: 2,
                target: 99,
                weapon: 0,
                reduced: false,
            },
        ],
        ..Default::default()
    };
    let (s, _) =
        export::delta_for_with_permissions(&batch, &after, SlotId(0), &Restricted, &history)
            .unwrap();
    let clean = CommittedTick {
        tick: 11,
        ..Default::default()
    };
    let (absent, _) =
        export::delta_for_with_permissions(&clean, &after, SlotId(0), &Restricted, &history)
            .unwrap();
    assert_eq!(bytes(&s), bytes(&absent));
    assert!(s.removed.is_empty());
    assert!(s.records.effects.is_empty());
    assert!(s.positions.is_empty());
    assert!(s.healths.is_empty());
}

#[test]
fn known_permitted_removal_is_an_event_but_visibility_loss_requires_a_snapshot() {
    let before = view(true);
    let (_, history) = export::snapshot_for_with_permissions(&before, SlotId(0), &Restricted);
    let mut after = before.clone();
    after.tick = 11;
    after.units.retain(|u| u.unit_id != 2);
    after.m3.raw_healths.retain(|u| u.unit != 2);
    let batch = CommittedTick {
        tick: 11,
        removed: vec![2, 77],
        effects: vec![
            EffectRow::Destroyed { unit: 2 },
            EffectRow::Destroyed { unit: 77 },
        ],
        ..Default::default()
    };
    let (delta, _) =
        export::delta_for_with_permissions(&batch, &after, SlotId(0), &Restricted, &history)
            .unwrap();
    assert_eq!(delta.removed, vec![turts_net_protocol::UnitId(2)]);
    assert_eq!(
        delta.records.effects,
        vec![turts_net_protocol::Effect::Destroyed {
            unit: turts_net_protocol::UnitId(2)
        }]
    );
    let no_death = CommittedTick {
        tick: 11,
        ..Default::default()
    };
    assert!(
        export::delta_for_with_permissions(&no_death, &after, SlotId(0), &Restricted, &history)
            .is_err()
    );
}
