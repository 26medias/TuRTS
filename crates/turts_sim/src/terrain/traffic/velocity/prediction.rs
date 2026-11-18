//! One primary proposal against the captured immutable permitted view.
//! The caller owns admission, scratch, compact caching and unused-credit release.
use super::super::velocity_solver::solve_with_share;
use super::*;

impl Guide<'_> {
    pub(super) fn predict_primary(
        &mut self,
        i: usize,
        request: MotionRequest,
        ticket: &mut Ticket,
        budget: &mut SweepBudget,
        plan: Option<&mut joint::JointPlan>,
    ) -> Option<Vec2> {
        let distance = request
            .goal
            .as_dvec2()
            .distance(request.position.as_dvec2());
        let travel = f64::from(request.speed) * f64::from(request.dt);
        let fallback = request.preferred_position();
        let actor = self.actors[i];
        let fallback = if bits(request.goal) == bits(actor.goal)
            && bits(request.position) == bits(actor.position)
        {
            actor.advice.unwrap_or(fallback)
        } else {
            fallback
        };
        #[cfg(feature = "test-probes")]
        let tracing = std::env::var("TURTS_BODY_TRACE_IDS")
            .ok()
            .is_some_and(|ids| {
                ids.len() <= 4096
                    && ids
                        .split(',')
                        .take(256)
                        .any(|id| id.parse::<u32>().ok() == Some(request.unit_id))
            })
            && std::env::var("TURTS_BODY_TRACE_TICKS")
                .ok()
                .is_none_or(|ticks| {
                    ticks.len() <= 4096
                        && ticks
                            .split(',')
                            .take(256)
                            .any(|tick| tick.parse::<u32>().ok() == Some(budget.epoch))
                });
        #[cfg(feature = "test-probes")]
        if tracing {
            eprintln!(
                "MODEL_BEGIN tick={} unit={} kind={:?} request_kind={:?} from={:?} goal={:?} ticket=primary spent={}",
                budget.epoch,
                request.unit_id,
                actor.kind,
                request.kind,
                request.position,
                request.goal,
                ticket.spent
            );
        }
        macro_rules! trace_advice {
            ($reason:expr, $spent:expr, $advice:expr) => {{
                #[cfg(feature = "test-probes")]
                if tracing {
                    eprintln!(
                        "MODEL_END tick={} unit={} reason={} spent={} advice={:?}",
                        budget.epoch, request.unit_id, $reason, $spent, $advice
                    );
                }
            }};
        }
        // An actual reachable destination retains ordinary arrival/holder logic.
        // A private waypoint remains modeled, including at sub-step distances.
        if actor.kind == MotionKind::Destination
            && request.kind == MotionKind::Destination
            && distance.is_finite()
            && travel.is_finite()
            && distance <= travel
        {
            trace_advice!("short-destination", 0usize, None::<Vec2>);
            return None;
        }

        let Some(view) = &self.views[actor.owner as usize] else {
            budget.model_fallbacks += 1;
            trace_advice!("no-view", ticket.spent, None::<Vec2>);
            return None;
        };
        if actor.kind != request.kind
            || bits(actor.position) != bits(request.position)
            || ticket.refused
        {
            budget.model_fallbacks += 1;
            trace_advice!("identity-or-prior-refusal", ticket.spent, None::<Vec2>);
            return None;
        }
        if !ticket.discovered {
            let mut charge = || spend(ticket, actor.owner, budget);
            // Discover along this bounded preferred chord with the legacy padding.
            let reach = f64::from(request.radius)
                + view.max_radius
                + (2. * f64::from(request.speed) * f64::from(request.dt))
                    .min(2. * f64::from(request.radius));
            let reach2 = reach * reach;
            if !reach.is_finite() || reach < 0. || !reach2.is_finite() {
                ticket.refused = true;
                budget.model_fallbacks += 1;
                trace_advice!("discovery-geometry", ticket.spent, None::<Vec2>);
                return None;
            }
            let mut valid_geometry = true;
            let mut selected = [EMPTY; 8];
            let mut selected_hard = [false; 8];
            let mut count = 0;
            let mut own_velocity = DVec2::ZERO;
            let visits = &mut self.visits[actor.owner as usize];
            let mut admit = || {
                if *visits >= OWNER_VISITS {
                    false
                } else {
                    *visits += 1;
                    true
                }
            };
            let complete = view.query(
                request.position.as_dvec2(),
                fallback.as_dvec2(),
                reach,
                &mut charge,
                &mut admit,
                |row| {
                    if row.unit == request.unit_id {
                        own_velocity = row.agent.velocity;
                        return;
                    }
                    let d = row
                        .agent
                        .position
                        .distance_squared(request.position.as_dvec2());
                    let Some(segment_distance) = segment_distance_squared(
                        row.agent.position,
                        request.position.as_dvec2(),
                        fallback.as_dvec2(),
                    ) else {
                        valid_geometry = false;
                        return;
                    };
                    if !d.is_finite() {
                        valid_geometry = false;
                        return;
                    }
                    if segment_distance > reach2 {
                        return;
                    }
                    let mut at = count.min(8);
                    for k in 0..count.min(8) {
                        let old = selected[k]
                            .position
                            .distance_squared(request.position.as_dvec2());
                        if d.total_cmp(&old)
                            .then_with(|| agent_cmp(row.agent, selected[k]))
                            .then(row.stationary.cmp(&selected_hard[k]))
                            .is_lt()
                        {
                            at = k;
                            break;
                        }
                    }
                    if at < 8 {
                        for k in (at + 1..count.min(7) + 1).rev() {
                            selected[k] = selected[k - 1];
                            selected_hard[k] = selected_hard[k - 1];
                        }
                        selected[at] = row.agent;
                        selected_hard[at] = row.stationary;
                        count = (count + 1).min(8);
                    }
                },
            );
            ticket.neighbors = selected;
            ticket.hard_mask = selected_hard[..count]
                .iter()
                .enumerate()
                .fold(0, |mask, (i, &hard)| mask | (u8::from(hard) << i));
            ticket.velocity = own_velocity;
            ticket.len = count;
            ticket.discovered = true;
            if !complete || !valid_geometry {
                ticket.refused = true;
                budget.model_fallbacks += 1;
                trace_advice!("discovery", ticket.spent, None::<Vec2>);
                return None;
            }
            budget.model_attempts += 1;
        }
        // Refine this actor's completed coherence, never generated peer advice.
        let preferred = (fallback.as_dvec2() - request.position.as_dvec2()) / f64::from(request.dt);
        let own_velocity = ticket.velocity;
        let neighbors = ticket.neighbors;
        let neighbor_count = ticket.len;
        let hard_mask = ticket.hard_mask;
        let agent = Agent {
            position: request.position.as_dvec2(),
            velocity: own_velocity,
            radius: f64::from(request.radius),
        };
        #[cfg(feature = "test-probes")]
        if tracing {
            eprintln!(
                "MODEL_INPUT tick={} unit={} actor={agent:?} preferred={preferred:?} count={} neighbors={:?} hard_mask={} spent={}",
                budget.epoch,
                request.unit_id,
                neighbor_count,
                &neighbors[..neighbor_count],
                ticket.hard_mask,
                ticket.spent
            );
        }
        let mut charge = || spend(ticket, actor.owner, budget);
        #[cfg(feature = "test-probes")]
        let share = if std::env::var("TURTS_TEST_RECIPROCAL_SHARE").is_ok_and(|s| s == "1") {
            0.5
        } else {
            1.
        };
        #[cfg(not(feature = "test-probes"))]
        let share = 1.;
        let solution = solve_with_share(
            agent,
            &neighbors[..neighbor_count],
            hard_mask,
            preferred,
            f64::from(request.speed),
            f64::from(request.dt),
            2. * f64::from(request.dt),
            share,
            &mut charge,
        );
        // The projected compromise resolves conflicting predictions. Re-solving
        // its same planes for cosmetic bias can consume the proof allowance.
        let refined = solution.is_some_and(|value| value.refined);
        let mut velocity = solution.map(|value| value.velocity);
        if !refined
            && neighbor_count > 0
            && velocity.is_some_and(|v| v.dot(preferred) <= 0.01 * preferred.length_squared())
        {
            for side in [1., -1.] {
                let bias = DVec2::new(preferred.y, -preferred.x) * (0.1 * side);
                if let Some(solution) = solve_with_share(
                    agent,
                    &neighbors[..neighbor_count],
                    hard_mask,
                    preferred + bias,
                    f64::from(request.speed),
                    f64::from(request.dt),
                    2. * f64::from(request.dt),
                    share,
                    &mut charge,
                ) {
                    let v = solution.velocity;
                    if v.dot(preferred) >= velocity.unwrap().dot(preferred)
                        && v != velocity.unwrap()
                    {
                        velocity = Some(v);
                        break;
                    }
                }
            }
        }
        let Some(velocity) = velocity else {
            budget.model_fallbacks += 1;
            trace_advice!("solve", ticket.spent, None::<Vec2>);
            return None;
        };
        let next = (request.position.as_dvec2() + velocity * f64::from(request.dt)).as_vec2();
        let mut charge = || spend(ticket, actor.owner, budget);
        if !prove(
            view,
            actor.id,
            request.position,
            request.radius,
            next,
            &mut self.visits[actor.owner as usize],
            &mut charge,
            &mut |from, to| (self.clear)(actor.index, from, to),
        ) {
            budget.model_fallbacks += 1;
            trace_advice!("static-or-terrain", ticket.spent, None::<Vec2>);
            return None;
        }
        if let Some(plan) = plan {
            let mut charge = || spend(ticket, actor.owner, budget);
            if !plan.try_upgrade(
                request.unit_id,
                next,
                &mut self.visits[actor.owner as usize],
                &mut charge,
            ) {
                budget.model_fallbacks += 1;
                trace_advice!("joint-upgrade", ticket.spent, None::<Vec2>);
                return None;
            }
        }
        trace_advice!("modeled", ticket.spent, Some(next));
        Some(next)
    }
}
