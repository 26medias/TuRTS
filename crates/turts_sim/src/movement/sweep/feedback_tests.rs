//! A controlled raw-guide seam, not a replay of the missing full guide/pool state.
use super::*;
use crate::movement::{
    MovementFrameUnit,
    motion::{MotionGuide, MotionKind, MotionRequest},
};
use std::collections::{BTreeMap, BTreeSet};
struct Preferred;
impl MotionGuide for Preferred {
    fn kind(&self, _: u32) -> MotionKind {
        MotionKind::Destination
    }
    fn next_position(&mut self, r: MotionRequest, _: &mut SweepBudget) -> Option<Vec2> {
        Some(r.preferred_position())
    }
}
#[test]
fn captured_gm1_direct_body_refusal_keeps_certified_recovery_feedback() {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../terrain/navigation/gm1_b979_late_state.json"
    ))
    .unwrap();
    let mut reference = None;
    for reverse in [false, true] {
        // Preserve all36 captured discs. Hold the other35 to isolate actor28's
        // direct request from unknown pool/model state; this is not a full replay.
        let mut frame = MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::new(64., 48.),
            tick_hz: 20,
            pools: BTreeMap::new(),
            units: rows
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    let large = row["label"].as_str().unwrap().starts_with("large");
                    let id = row["unit"].as_u64().unwrap() as u32;
                    MovementFrameUnit {
                        unit_id: id,
                        owner_slot: 0,
                        unit_type: if large { 2 } else { 1 },
                        position: serde_json::from_value(row["position"].clone()).unwrap(),
                        radius: if large { 1.1 } else { 0.4 },
                        speed: (id == 28).then_some(2.5),
                        target: if id == 28 {
                            serde_json::from_value(row["target"].clone()).unwrap()
                        } else {
                            None
                        },
                        progress: None,
                        plain_progress: None,
                        physical_escape: None,
                    }
                })
                .collect(),
        };
        if reverse {
            frame.units.reverse();
            for u in &mut frame.units {
                u.unit_id = 1000 - u.unit_id;
            }
        }
        let id = if reverse { 972 } else { 28 };
        let mut trace = Vec::new();
        let mut refused = false;
        let mut recovery_requested = false;
        for tick in 2401..2425 {
            let index = frame.units.iter().position(|u| u.unit_id == id).unwrap();
            let from = frame.units[index].position;
            let target = frame.units[index].target.unwrap();
            let raw = MotionRequest {
                unit_id: id,
                position: from,
                goal: target,
                speed: 2.5,
                radius: 1.1,
                bounds: frame.bounds,
                dt: 0.05,
                kind: MotionKind::Destination,
            }
            .preferred_position();
            let out = crate::movement::step_frame_with_motion_guide(
                &frame,
                0.05,
                &BTreeSet::new(),
                &mut SweepBudget::new(tick),
                &mut Preferred,
            );
            let actor = out.iter().find(|u| u.unit_id == id).unwrap();
            if actor.position != raw {
                refused = true;
                assert!(
                    actor.plain_progress.is_some(),
                    "a certified body refusal cannot remain invisible to recovery"
                );
            }
            for peer in frame.units.iter().filter(|u| u.unit_id != id) {
                let result = out.iter().find(|u| u.unit_id == peer.unit_id).unwrap();
                assert_eq!(result.position, peer.position);
                assert!(pair_clear(
                    from,
                    actor.position,
                    1.1,
                    peer.position,
                    peer.position,
                    peer.radius
                ));
            }
            recovery_requested |= actor.plain_progress.as_ref().is_some_and(|p| {
                p.stalled_ticks >= 20
                    || p.activity == MovementActivity::Blocked
                    || p.claim_count > 0
            });
            trace.push(actor.position);
            frame.units[index].position = actor.position;
            frame.units[index].target = actor.target;
            frame.units[index].plain_progress = actor.plain_progress.clone();
            frame.units[index].physical_escape = actor.physical_escape.clone();
            if recovery_requested {
                break;
            }
        }
        assert!(refused);
        assert!(
            recovery_requested,
            "bounded repeated certified refusals must request existing recovery"
        );
        if let Some(old) = &reference {
            assert_eq!(old, &trace);
        } else {
            reference = Some(trace);
        }
    }
}
