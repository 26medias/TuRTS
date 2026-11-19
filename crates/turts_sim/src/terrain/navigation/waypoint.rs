//! Bounded continuation for an order's temporary route or yield goal.
//! A physical-prefix retry is never promoted to the intended local goal.
use super::*;
use crate::movement::{
    MoveProgress, MovementActivity, MovementFrameOutput, MovementFrameUnit, PhysicalEscape,
    PlainMoveProgress,
};
#[derive(Clone)]
pub(super) struct WaypointMotion {
    pub(super) target: Vec2,
    pub(super) parking: bool,
    spares: [Vec2; 6],
    spare_len: usize,
    pub(super) group: Option<MoveProgress>,
    pub(super) plain: Option<PlainMoveProgress>,
    pub(super) escape: Option<PhysicalEscape>,
}
impl WaypointMotion {
    pub(super) fn plain_progress(&self) -> Option<PlainMoveProgress> {
        self.plain.as_ref().map(|p| {
            let mut p = p.clone();
            if self.spare_len > 0 {
                p.pool_slots = self.spares[..self.spare_len].to_vec();
            }
            p
        })
    }
    fn pack(&mut self) {
        if let Some(p) = self.plain.as_mut() {
            let values = std::mem::take(&mut p.pool_slots);
            self.spare_len = values.len();
            assert!(self.spare_len <= 6, "bounded local spare slots");
            self.spares[..values.len()].copy_from_slice(&values);
        }
    }
    pub(super) fn bytes(&self) -> usize {
        self.plain
            .as_ref()
            .map_or(0, |p| p.pool_slots.capacity() * size_of::<Vec2>())
    }
}
type Counter = Option<(Vec2, f32, u16)>;
pub(in crate::terrain::navigation) struct PreparedMotion {
    epoch: u64,
    intent_position: Vec2,
    arrival_slot: Option<Vec2>,
    previous_group: Counter,
    previous_plain: Counter,
    group_recovered: bool,
    plain_recovered: bool,
    speed: f32,
    motion: WaypointMotion,
}
// Existing per-viewer scratch reservations cover this one common transient
// buffer. It has one row per admitted member and at most six spare-slot values.
const _: () = assert!(
    MAX_MEMBERS * VIEWERS * (size_of::<(usize, PreparedMotion)>() + 6 * size_of::<Vec2>() + 128)
        <= 4 * 1024 * 1024
);
impl TerrainNavigation {
    /// Certified local recovery belongs to the connector cache, while the ECS
    /// progress still belongs to the original order. Let that local refusal
    /// request the existing own-body detour without changing either endpoint.
    pub(in crate::terrain::navigation) fn waypoint_needs_detour(
        &self,
        unit: u32,
        viewer: u8,
        stalled_limit: u16,
    ) -> bool {
        let Some(motion) = self
            .viewers
            .get(usize::from(viewer))
            .and_then(|book| book.members.get(&unit))
            .filter(|member| {
                member.viewer == viewer
                    && member.live_target
                    && !member.cancelled
                    && !member.owner_fence
                    && !member.owner_released
            })
            .and_then(|member| member.motion.as_ref())
            .filter(|motion| !motion.parking)
        else {
            return false;
        };
        motion.plain.as_ref().is_some_and(|p| {
            p.activity == MovementActivity::Blocked || p.stalled_ticks >= stalled_limit.max(1)
        }) || motion.group.as_ref().is_some_and(|p| {
            p.activity == MovementActivity::Blocked || p.stalled_ticks >= stalled_limit.max(1)
        })
    }
    pub(in crate::terrain::navigation) fn load_waypoint_motion(
        &mut self,
        original: &MovementFrameUnit,
        proposed: &mut MovementFrameUnit,
        parking: bool,
    ) {
        self.load_motion(original, proposed, parking, false);
    }
    pub(in crate::terrain::navigation) fn load_certified_waypoint_motion(
        &mut self,
        original: &MovementFrameUnit,
        proposed: &mut MovementFrameUnit,
    ) {
        self.load_motion(original, proposed, false, true);
    }
    fn load_motion(
        &mut self,
        original: &MovementFrameUnit,
        proposed: &mut MovementFrameUnit,
        parking: bool,
        certified: bool,
    ) {
        let temporary = proposed.target != super::super::requested_target(original)
            || proposed.speed != original.speed;
        // Clear fenced/unadmitted inputs too. A cache miss cannot resurrect old
        // original-point movement through a stopped guidance row.
        if temporary || certified {
            proposed.progress = None;
            proposed.plain_progress = None;
            proposed.physical_escape = None;
        }
        let Some(book) = self.viewers.get_mut(usize::from(original.owner_slot)) else {
            return;
        };
        let Some(member) = book
            .members
            .get_mut(&original.unit_id)
            .filter(|m| !m.cancelled && !m.owner_fence && m.viewer == original.owner_slot)
        else {
            return;
        };
        let before = member.bytes();
        if !temporary || certified {
            member.motion = None;
        } else if let Some(target) = proposed.target.filter(|_| proposed.speed.is_some()) {
            if let Some(motion) = member
                .motion
                .as_ref()
                .filter(|m| m.target == target && m.parking == parking)
            {
                proposed.progress = motion.group.clone();
                proposed.plain_progress = motion.plain_progress();
                // A route connector owns no terminal destination. Its previous
                // local search may have exhausted while another mover occupied
                // the corner. Retry ordinary bounded steering, rather than
                // caching that terminal refusal for the rest of the order.
                if !parking
                    && proposed
                        .plain_progress
                        .as_ref()
                        .is_some_and(|p| p.activity == MovementActivity::Blocked)
                {
                    proposed.plain_progress = None;
                }
                proposed.physical_escape = motion.escape.clone();
            } else {
                member.motion = None;
            }
        }
        book.member_buffer_bytes = book.member_buffer_bytes.saturating_sub(before) + member.bytes();
    }
    pub(in crate::terrain::navigation) fn prepare_waypoint_motion(
        &self,
        original: &MovementFrameUnit,
        guide: &MovementFrameUnit,
        intent: &MovementFrameOutput,
        parking: bool,
    ) -> Option<PreparedMotion> {
        let target = guide.target.filter(|_| guide.speed.is_some())?;
        let member = self
            .viewers
            .get(usize::from(original.owner_slot))?
            .members
            .get(&original.unit_id)
            .filter(|m| !m.cancelled && !m.owner_fence && m.viewer == original.owner_slot)?;
        if target == member.endpoint && guide.speed == original.speed {
            return None;
        }
        if intent
            .plain_progress
            .as_ref()
            .is_some_and(|p| p.pool_slots.len() > 6)
        {
            return None;
        }
        Some(PreparedMotion {
            epoch: member.epoch,
            intent_position: intent.position,
            arrival_slot: intent.arrival_slot,
            speed: guide.speed.unwrap_or(0.0),
            group_recovered: guide
                .progress
                .as_ref()
                .zip(intent.progress.as_ref())
                .is_some_and(|(old, new)| {
                    old.pass_mode != new.pass_mode
                        || old.claim_count != new.claim_count
                        || old.slot != new.slot
                }),
            plain_recovered: guide
                .plain_progress
                .as_ref()
                .zip(intent.plain_progress.as_ref())
                .is_some_and(|(old, new)| {
                    old.claim_count != new.claim_count || old.slot != new.slot
                }),
            previous_group: guide
                .progress
                .as_ref()
                .map(|p| (p.slot, p.best_distance, p.stalled_ticks)),
            previous_plain: guide
                .plain_progress
                .as_ref()
                .map(|p| (p.slot, p.best_distance, p.stalled_ticks)),
            motion: WaypointMotion {
                target,
                parking,
                spares: [Vec2::ZERO; 6],
                spare_len: 0,
                group: intent.progress.clone(),
                plain: intent.plain_progress.clone(),
                escape: intent.physical_escape.clone(),
            },
        })
    }
    pub(in crate::terrain::navigation) fn commit_waypoint_motion(
        &mut self,
        original: &MovementFrameUnit,
        prepared: PreparedMotion,
        accepted: &MovementFrameOutput,
        dt: f32,
    ) -> bool {
        let Some(book) = self.viewers.get_mut(usize::from(original.owner_slot)) else {
            return true;
        };
        let Some(member) = book.members.get_mut(&original.unit_id).filter(|m| {
            !m.cancelled
                && !m.owner_fence
                && m.viewer == original.owner_slot
                && m.epoch == prepared.epoch
        }) else {
            return true;
        };
        let before = member.bytes();
        let mut motion = prepared.motion;
        // Slot/side intent can survive rejection, but earned progress must use
        // only the final body- and terrain-certified position of this tick.
        let synthesized =
            motion.plain.is_none() && motion.group.is_none() && accepted.position != motion.target;
        if synthesized {
            let parked = motion.parking && prepared.arrival_slot == Some(accepted.position);
            let slot = if parked {
                accepted.position
            } else {
                motion.target
            };
            motion.plain = Some(PlainMoveProgress {
                arrival_yield: false,
                original_point: motion.target,
                slot,
                pool_slots: Vec::new(),
                gather_goal: None,
                activity: if parked {
                    MovementActivity::Resting
                } else {
                    MovementActivity::Moving
                },
                stalled_ticks: 0,
                best_distance: original.position.distance(slot),
                claim_count: u8::from(parked),
                failure_recorded: false,
            });
        }
        if accepted.position != prepared.intent_position || synthesized {
            let step = prepared.speed * dt;
            if let Some(p) = motion.group.as_mut() {
                (p.best_distance, p.stalled_ticks) = if prepared.group_recovered {
                    (accepted.position.distance(p.slot), 0)
                } else {
                    crate::movement::sweep::progress(
                        prepared.previous_group,
                        original.position,
                        accepted.position,
                        p.slot,
                        step,
                    )
                };
                if p.activity == MovementActivity::Resting && accepted.position != p.slot {
                    p.activity = MovementActivity::Moving;
                }
            }
            if let Some(p) = motion.plain.as_mut() {
                (p.best_distance, p.stalled_ticks) = if prepared.plain_recovered {
                    (accepted.position.distance(p.slot), 0)
                } else {
                    crate::movement::sweep::progress(
                        prepared.previous_plain,
                        original.position,
                        accepted.position,
                        p.slot,
                        step,
                    )
                };
                if p.activity == MovementActivity::Resting && accepted.position != p.slot {
                    p.activity = MovementActivity::Moving;
                }
            }
        }
        // Route connectors must remain active when a collision fallback stops
        // short. Traffic parking may keep its legal nearby resting position.
        if !motion.parking && accepted.position != motion.target {
            if let Some(p) = motion
                .group
                .as_mut()
                .filter(|p| p.activity == MovementActivity::Resting)
            {
                p.activity = MovementActivity::Moving;
                p.slot = motion.target;
                p.best_distance = accepted.position.distance(motion.target);
                p.stalled_ticks = 0;
            }
            if let Some(p) = motion
                .plain
                .as_mut()
                .filter(|p| p.activity == MovementActivity::Resting)
            {
                p.activity = MovementActivity::Moving;
                p.slot = motion.target;
                p.original_point = motion.target;
                p.best_distance = accepted.position.distance(motion.target);
                p.stalled_ticks = 0;
            }
        }
        let stalled = motion
            .group
            .as_ref()
            .is_some_and(|p| p.stalled_ticks > 0 || p.activity != MovementActivity::Moving)
            || motion
                .plain
                .as_ref()
                .is_some_and(|p| p.stalled_ticks > 0 || p.activity != MovementActivity::Moving)
            || accepted.position == original.position;
        // Inline spares are paid for at member admission. No ID-ordered
        // commit-time memory contest can choose whose continuation survives.
        motion.pack();
        member.motion = Some(motion);
        book.member_buffer_bytes = book.member_buffer_bytes.saturating_sub(before) + member.bytes();
        stalled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unit() -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 1,
            position: Vec2::new(2., 2.),
            radius: 0.4,
            speed: Some(2.),
            target: Some(Vec2::new(20., 2.)),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    fn plain(target: Vec2, best: f32, stalled: u16) -> PlainMoveProgress {
        PlainMoveProgress {
            arrival_yield: false,
            original_point: target,
            slot: target,
            pool_slots: vec![target + Vec2::Y],
            gather_goal: None,
            activity: MovementActivity::Moving,
            stalled_ticks: stalled,
            best_distance: best,
            claim_count: 0,
            failure_recorded: false,
        }
    }
    fn output(position: Vec2, target: Vec2) -> MovementFrameOutput {
        MovementFrameOutput {
            recovery: Default::default(),
            unit_id: 1,
            position,
            target: Some(target),
            progress: None,
            plain_progress: Some(plain(target, position.distance(target), 0)),
            physical_escape: None,
            failed_blocked: false,
            arrival_slot: None,
            #[cfg(feature = "test-probes")]
            probe: Default::default(),
        }
    }
    fn setup() -> (TerrainNavigation, MovementFrameUnit, MovementFrameUnit) {
        let mut navigation = TerrainNavigation::default();
        let original = unit();
        let mut member = Member::stamp(0, 1, 7, 0);
        member.cancelled = false;
        member.live_target = true;
        member.endpoint = original.target.unwrap();
        navigation.viewers[0].members.insert(1, member);
        navigation.viewers[0].member_ids.push(1);
        let mut guide = original.clone();
        guide.target = Some(Vec2::new(8., 2.));
        (navigation, original, guide)
    }
    #[test]
    fn captured_gm1_private_blocked_corner_can_recover_without_evicting_holders() {
        // GM1-wide Quarry, ef619c27, tick 2400. All 36 captured poses,
        // progress and escape records are retained; profile radii/speeds are
        // the authored small (.4/3.5) and large (1.1/2.5) values. Only unit26's
        // private cache is reconstructed separately from its real order state.
        let rows: serde_json::Value =
            serde_json::from_str(include_str!("gm1_late_state.json")).unwrap();
        for reverse in [false, true] {
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
                        MovementFrameUnit {
                            unit_id: row["unit"].as_u64().unwrap() as u32,
                            owner_slot: 0,
                            unit_type: if large { 2 } else { 1 },
                            position: serde_json::from_value(row["position"].clone()).unwrap(),
                            radius: if large { 1.1 } else { 0.4 },
                            speed: Some(if large { 2.5 } else { 3.5 }),
                            target: serde_json::from_value(row["target"].clone()).unwrap(),
                            progress: serde_json::from_value(row["group_progress"].clone())
                                .unwrap(),
                            plain_progress: serde_json::from_value(row["plain_progress"].clone())
                                .unwrap(),
                            physical_escape: serde_json::from_value(row["physical_escape"].clone())
                                .unwrap(),
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
            let id = if reverse { 974 } else { 26 };
            let index = frame.units.iter().position(|u| u.unit_id == id).unwrap();
            let original = frame.units[index].clone();
            let corner = Vec2::new(47.145386, 27.13149);
            let mut navigation = TerrainNavigation::default();
            let mut member = Member::stamp(0, 1, 8, 0);
            member.cancelled = false;
            member.live_target = true;
            member.endpoint = original.target.unwrap();
            let mut blocked = plain(corner, 1.5693549, 251);
            blocked.activity = MovementActivity::Blocked;
            blocked.pool_slots.clear();
            member.motion = Some(WaypointMotion {
                target: corner,
                parking: false,
                spares: [Vec2::ZERO; 6],
                spare_len: 0,
                group: None,
                plain: Some(blocked),
                escape: None,
            });
            navigation.viewers[0].members.insert(id, member);
            frame.units[index].target = Some(corner);
            navigation.load_waypoint_motion(&original, &mut frame.units[index], false);
            struct PrivateGuide {
                id: u32,
                endpoint: Vec2,
            }
            impl crate::movement::motion::MotionGuide for PrivateGuide {
                fn kind(&self, id: u32) -> crate::movement::motion::MotionKind {
                    if id == self.id {
                        crate::movement::motion::MotionKind::Waypoint
                    } else {
                        crate::movement::motion::MotionKind::Destination
                    }
                }
                fn arrival_destination(&self, id: u32) -> Option<Vec2> {
                    (id == self.id).then_some(self.endpoint)
                }
                fn next_position(
                    &mut self,
                    r: crate::movement::motion::MotionRequest,
                    _: &mut crate::movement::sweep::SweepBudget,
                ) -> Option<Vec2> {
                    (r.unit_id == self.id).then(|| r.preferred_position())
                }
            }
            let mut budget = crate::movement::sweep::SweepBudget::new(2401);
            let out = crate::movement::step_frame_with_motion_guide(
                &frame,
                0.05,
                &BTreeSet::new(),
                &mut budget,
                &mut PrivateGuide {
                    id,
                    endpoint: original.target.unwrap(),
                },
            );
            let result = out.iter().find(|u| u.unit_id == id).unwrap();

            assert_ne!(
                result.position, original.position,
                "a terminal private waypoint must allow fresh bounded recovery"
            );
            assert!(result.position.distance(original.position) <= 2.5 * 0.05 + 0.00001);
            for (i, a) in frame.units.iter().enumerate() {
                for b in &frame.units[i + 1..] {
                    let ao = out.iter().find(|o| o.unit_id == a.unit_id).unwrap();
                    let bo = out.iter().find(|o| o.unit_id == b.unit_id).unwrap();
                    assert!(
                        crate::movement::sweep::pair_clear(
                            a.position,
                            ao.position,
                            a.radius,
                            b.position,
                            bo.position,
                            b.radius
                        ),
                        "captured pair {} / {}",
                        a.unit_id,
                        b.unit_id
                    );
                }
            }
            for holder in frame.units.iter().filter(|u| u.target.is_none()) {
                assert_eq!(
                    out.iter()
                        .find(|o| o.unit_id == holder.unit_id)
                        .unwrap()
                        .position,
                    holder.position
                );
            }
        }
    }
    #[test]
    fn cached_connector_stalls_request_own_detours_without_destination_or_parking_leaks() {
        let (mut n, original, guide) = setup();
        let target = guide.target.unwrap();
        let intent = output(original.position, target);
        let prepared = n
            .prepare_waypoint_motion(&original, &guide, &intent, false)
            .unwrap();
        n.commit_waypoint_motion(&original, prepared, &intent, 0.05);
        let member = n.viewers[0].members.get_mut(&1).unwrap();
        member
            .motion
            .as_mut()
            .unwrap()
            .plain
            .as_mut()
            .unwrap()
            .stalled_ticks = 20;
        assert!(original.plain_progress.is_none());
        assert!(n.waypoint_needs_detour(1, 0, 20));
        assert!(!n.waypoint_needs_detour(1, 1, 20));
        assert!(!n.waypoint_needs_detour(1, 0, 21));
        n.viewers[0]
            .members
            .get_mut(&1)
            .unwrap()
            .motion
            .as_mut()
            .unwrap()
            .parking = true;
        assert!(!n.waypoint_needs_detour(1, 0, 20));
        let member = n.viewers[0].members.get_mut(&1).unwrap();
        member.motion.as_mut().unwrap().parking = false;
        member
            .motion
            .as_mut()
            .unwrap()
            .plain
            .as_mut()
            .unwrap()
            .activity = MovementActivity::Blocked;
        assert!(n.waypoint_needs_detour(1, 0, u16::MAX));
        n.viewers[0].members.get_mut(&1).unwrap().cancelled = true;
        assert!(!n.waypoint_needs_detour(1, 0, 20));
    }
    #[test]
    fn fresh_complete_connector_cannot_reload_a_cached_local_escape() {
        let (mut n, original, mut guide) = setup();
        let target = guide.target.unwrap();
        let escape = crate::movement::PhysicalEscape {
            target: Some(target + Vec2::Y),
        };
        let mut progress = plain(target, 3., 41);
        progress.slot = target + Vec2::Y;
        n.viewers[0].members.get_mut(&1).unwrap().motion = Some(WaypointMotion {
            target,
            parking: false,
            spares: [Vec2::ZERO; 6],
            spare_len: 0,
            group: None,
            plain: Some(progress),
            escape: Some(escape.clone()),
        });
        let mut ordinary = guide.clone();
        n.load_waypoint_motion(&original, &mut ordinary, false);
        assert_eq!(ordinary.physical_escape, Some(escape));
        n.load_certified_waypoint_motion(&original, &mut guide);
        assert_eq!(guide.target, Some(target));
        assert!(
            guide.progress.is_none()
                && guide.plain_progress.is_none()
                && guide.physical_escape.is_none(),
            "fresh complete leg owns its motion, without changing the original order"
        );
        assert!(n.viewers[0].members[&1].motion.is_none());
    }

    #[test]
    fn only_terminal_private_connector_progress_is_retried() {
        for (parking, activity, temporary) in [
            (true, MovementActivity::Blocked, true),
            (false, MovementActivity::Moving, true),
            (false, MovementActivity::Blocked, false),
        ] {
            let (mut navigation, original, mut guide) = setup();
            if !temporary {
                guide.target = original.target;
            }
            let target = guide.target.unwrap();
            let mut p = plain(target, 3., 41);
            p.activity = activity;
            navigation.viewers[0].members.get_mut(&1).unwrap().motion = Some(WaypointMotion {
                target,
                parking,
                spares: [Vec2::ZERO; 6],
                spare_len: 0,
                group: None,
                plain: Some(p.clone()),
                escape: None,
            });
            if !temporary {
                guide.plain_progress = Some(p.clone());
            }
            navigation.load_waypoint_motion(&original, &mut guide, parking);
            assert_eq!(guide.plain_progress, Some(p));
        }
    }
    #[test]
    fn prefix_retry_retains_intended_waypoint_and_only_credits_accepted_motion() {
        let (mut n, mut original, mut guide) = setup();
        let target = guide.target.unwrap();
        let intent = output(Vec2::new(2.1, 2.), target);
        let mut accepted = intent.clone();
        accepted.position = original.position;
        accepted.target = Some(Vec2::new(2.02, 2.));
        let prepared = n
            .prepare_waypoint_motion(&original, &guide, &intent, false)
            .unwrap();
        assert!(
            n.commit_waypoint_motion(&original, prepared, &accepted, 0.05),
            "certified local stall must publish an owner wait"
        );
        n.load_waypoint_motion(&original, &mut guide, false);
        let saved = guide.plain_progress.as_ref().unwrap();
        assert_eq!(saved.original_point, target);
        assert_eq!(saved.slot, target);
        assert_eq!((saved.best_distance, saved.stalled_ticks), (6., 1));
        assert_eq!(
            original.target,
            Some(Vec2::new(20., 2.)),
            "the player endpoint is separate"
        );
        let intent = output(Vec2::new(2.1, 2.), target);
        let mut accepted = intent.clone();
        accepted.position = Vec2::new(2.05, 2.);
        let prepared = n
            .prepare_waypoint_motion(&original, &guide, &intent, false)
            .unwrap();
        assert!(
            !n.commit_waypoint_motion(&original, prepared, &accepted, 0.05),
            "actual forward progress clears the local wait"
        );
        original.position = accepted.position;
        n.load_waypoint_motion(&original, &mut guide, false);
        assert_eq!(
            guide.plain_progress.as_ref().unwrap().best_distance,
            accepted.position.distance(target)
        );
        assert_eq!(guide.plain_progress.as_ref().unwrap().stalled_ticks, 0);
    }
    #[test]
    fn an_uncommitted_waypoint_completion_does_not_mark_the_local_goal_reached() {
        let (mut n, original, mut guide) = setup();
        let target = guide.target.unwrap();
        let mut intent = output(target, target);
        intent.target = None;
        intent.plain_progress = None;
        let mut accepted = intent.clone();
        accepted.position = original.position;
        let prepared = n
            .prepare_waypoint_motion(&original, &guide, &intent, false)
            .unwrap();
        n.commit_waypoint_motion(&original, prepared, &accepted, 0.05);
        n.load_waypoint_motion(&original, &mut guide, false);
        let saved = guide.plain_progress.unwrap();
        assert_eq!(saved.activity, MovementActivity::Moving);
        assert_eq!(saved.slot, target);
        assert_eq!(saved.stalled_ticks, 1);
    }
    #[test]
    fn temporary_hold_resumes_but_changed_goal_epoch_and_owner_cannot_restore_it() {
        let (mut n, original, mut guide) = setup();
        let target = guide.target.unwrap();
        let intent = output(Vec2::new(2.1, 2.), target);
        let prepared = n
            .prepare_waypoint_motion(&original, &guide, &intent, false)
            .unwrap();
        n.commit_waypoint_motion(&original, prepared, &intent, 0.05);
        let mut hold = guide.clone();
        hold.target = None;
        hold.speed = None;
        n.load_waypoint_motion(&original, &mut hold, false);
        assert!(hold.plain_progress.is_none());
        n.load_waypoint_motion(&original, &mut guide, false);
        assert!(guide.plain_progress.is_some());
        let mut foreign = original.clone();
        foreign.owner_slot = 1;
        let mut fenced = guide.clone();
        fenced.target = None;
        fenced.speed = None;
        n.load_waypoint_motion(&foreign, &mut fenced, false);
        assert!(fenced.plain_progress.is_none());
        let mut changed = guide.clone();
        changed.target = Some(target + Vec2::Y);
        n.load_waypoint_motion(&original, &mut changed, false);
        assert!(
            changed.plain_progress.is_none(),
            "a different local goal cannot reuse an old slot"
        );
        assert!(n.viewers[0].members.get(&1).unwrap().motion.is_none());
        n.load_waypoint_motion(&original, &mut guide, false);
        let prepared = n
            .prepare_waypoint_motion(&original, &guide, &intent, false)
            .unwrap();
        n.viewers[0].cancel(1, 10, Some(0));
        assert!(
            n.viewers[0].members.get(&1).unwrap().motion.is_none(),
            "Stop/Replace clears local continuation immediately"
        );
        n.commit_waypoint_motion(&original, prepared, &intent, 0.05);
        assert!(n.viewers[0].members.get(&1).unwrap().motion.is_none());
        guide.target = Some(target + Vec2::Y);
        n.load_waypoint_motion(&original, &mut guide, false);
        assert!(guide.plain_progress.is_none());
    }
    #[test]
    fn private_motion_storage_rejects_more_than_the_existing_six_spare_slots() {
        let (n, original, guide) = setup();
        let mut intent = output(Vec2::new(2.1, 2.), guide.target.unwrap());
        intent.plain_progress.as_mut().unwrap().pool_slots = vec![Vec2::ZERO; 7];
        assert!(
            n.prepare_waypoint_motion(&original, &guide, &intent, false)
                .is_none()
        );
    }
    #[test]
    fn retired_waypoint_motion_returns_its_actual_buffer_credit() {
        let (mut n, original, guide) = setup();
        let intent = output(Vec2::new(2.1, 2.), guide.target.unwrap());
        let prepared = n
            .prepare_waypoint_motion(&original, &guide, &intent, false)
            .unwrap();
        n.commit_waypoint_motion(&original, prepared, &intent, 0.05);
        assert_eq!(
            n.viewers[0].member_buffer_bytes, 0,
            "spares use already reserved inline storage"
        );
        assert_eq!(
            n.viewers[0]
                .members
                .get(&1)
                .unwrap()
                .motion
                .as_ref()
                .unwrap()
                .bytes(),
            0
        );
        n.viewers[0].reclaim(original.unit_id);
        assert_eq!(n.viewers[0].member_buffer_bytes, 0);
        assert!(n.viewers[0].members.is_empty());
    }
    #[test]
    fn clipped_recovery_preserves_reset_and_inline_spares_under_pressure() {
        let (mut n, original, mut guide) = setup();
        let target = guide.target.unwrap();
        guide.plain_progress = Some(plain(target, 6., 80));
        let mut intent = output(Vec2::new(2.1, 2.), target);
        let p = intent.plain_progress.as_mut().unwrap();
        p.claim_count = 1;
        p.slot = target + Vec2::Y;
        p.pool_slots = (1..=6).map(|i| target + Vec2::Y * i as f32).collect();
        let spares = p.pool_slots.clone();
        // Fixed member storage was reserved at admission; no dynamic credit is
        // needed to retain this recovery when the book has no spare capacity.
        n.viewers[0].member_buffer_bytes = super::super::MAX_BYTES;
        let prepared = n
            .prepare_waypoint_motion(&original, &guide, &intent, false)
            .unwrap();
        let accepted = output(original.position, target);
        n.commit_waypoint_motion(&original, prepared, &accepted, 0.05);
        n.load_waypoint_motion(&original, &mut guide, false);
        let saved = guide.plain_progress.as_ref().unwrap();
        assert_eq!(saved.claim_count, 1);
        assert_eq!(saved.stalled_ticks, 0);
        assert_eq!(saved.best_distance, original.position.distance(saved.slot));
        assert_eq!(saved.pool_slots, spares);
    }
    #[test]
    fn nearby_route_fallback_retries_but_parking_rest_keeps_the_original_order_active() {
        for parking in [false, true] {
            let (mut n, original, mut guide) = setup();
            let target = guide.target.unwrap();
            let mut intent = output(Vec2::new(7., 2.), target);
            intent.target = None;
            let p = intent.plain_progress.as_mut().unwrap();
            p.slot = intent.position;
            p.activity = MovementActivity::Resting;
            let prepared = n
                .prepare_waypoint_motion(&original, &guide, &intent, parking)
                .unwrap();
            n.commit_waypoint_motion(&original, prepared, &intent, 0.05);
            n.load_waypoint_motion(&original, &mut guide, parking);
            let saved = guide.plain_progress.as_ref().unwrap();
            if parking {
                assert_eq!(saved.activity, MovementActivity::Resting);
                assert_eq!(saved.slot, intent.position);
            } else {
                assert_eq!(saved.activity, MovementActivity::Moving);
                assert_eq!(saved.slot, target);
            }
            assert_eq!(
                original.target,
                Some(Vec2::new(20., 2.)),
                "neither local completion changes the player goal"
            );
        }
    }
    #[test]
    fn body_guard_clipped_completion_still_accumulates_real_local_recovery_progress() {
        let (mut n, original, mut guide) = setup();
        let target = guide.target.unwrap();
        let mut intent = output(original.position, target);
        intent.plain_progress = None;
        let prepared = n
            .prepare_waypoint_motion(&original, &guide, &intent, false)
            .unwrap();
        assert!(n.commit_waypoint_motion(&original, prepared, &intent, 0.05));
        n.load_waypoint_motion(&original, &mut guide, false);
        let p = guide.plain_progress.unwrap();
        assert_eq!(p.slot, target);
        assert_eq!(p.best_distance, 6.);
        assert_eq!(p.stalled_ticks, 1);
    }
}
