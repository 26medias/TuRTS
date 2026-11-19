//! Private raw-motion seam. Final physical certification remains mandatory.
use super::sweep::SweepBudget;
use bevy_math::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MotionKind {
    Destination,
    Waypoint,
    /// Fresh owner-local complete body/terrain leg. Optional steering must
    /// preserve its segment; final physical certification still owns safety.
    CertifiedWaypoint,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MotionRequest {
    /// Lookup only; never use this handle for steering priority.
    pub unit_id: u32,
    pub position: Vec2,
    pub goal: Vec2,
    pub speed: f32,
    pub radius: f32,
    pub bounds: Vec2,
    pub dt: f32,
    pub kind: MotionKind,
}

pub(crate) trait MotionGuide {
    fn kind(&self, unit_id: u32) -> MotionKind;

    /// Immutable order destination before private route/retry targets replaced it.
    /// Arrival ordering may use this for Waypoint rows; it never owns the connector.
    fn arrival_destination(&self, _unit_id: u32) -> Option<Vec2> {
        None
    }

    /// Planning guides charge discovery and predicates to one steering counter.
    /// `legacy_charge` preserves existing real-guard visit accounting.
    fn charge_certificate_visit(
        &self,
        _unit_id: u32,
        owner: u8,
        legacy_charge: bool,
        budget: &mut SweepBudget,
    ) -> bool {
        !legacy_charge || budget.body(owner)
    }

    /// Planning charges every extracted row before membership/envelope filters.
    fn examine_certificate_row(
        &self,
        _unit_id: u32,
        _owner: u8,
        _budget: &mut SweepBudget,
    ) -> bool {
        true
    }

    /// Legacy admission is separate from planning's already charged extraction.
    fn admit_certificate_row(
        &self,
        unit_id: u32,
        owner: u8,
        legacy_charge: bool,
        budget: &mut SweepBudget,
    ) -> bool {
        self.charge_certificate_visit(unit_id, owner, legacy_charge, budget)
    }

    fn charge_certificate_pair(
        &self,
        _unit_id: u32,
        owner: u8,
        spent: &mut usize,
        allowance: usize,
        budget: &mut SweepBudget,
    ) -> bool {
        budget.charge_pair(owner, spent, allowance)
    }

    /// Private planning permission to attempt in-flight support directly.
    /// This grants no paid ordinary admission or physical certificate.
    fn deferred_support_enabled(&self) -> bool {
        false
    }

    /// The real guard keeps its search. A cheap planning baseline can hold.
    fn certificate_search_enabled(&self) -> bool {
        true
    }

    /// True when the frame solver owns pool recovery with no private route state. Then a
    /// stalled mover may exchange its slot claim with a moving peer (`exchange`). A guide
    /// that keeps its own destinations must leave this off.
    fn exchanges_claims(&self) -> bool {
        false
    }

    /// None retains the old path only for motions outside the new scope.
    /// Eligible model refusals must return the bounded preferred next pose.
    fn next_position(&mut self, request: MotionRequest, budget: &mut SweepBudget) -> Option<Vec2>;

    /// Release unused prediction reservations before physical certification.
    /// Spent tickets survive this boundary and all later terrain retries.
    fn finish_prediction(&mut self, _budget: &mut SweepBudget) {}
}

/// Legacy entry points remain bit-for-bit on the original steering path.
pub(crate) struct NoMotionGuide;
impl MotionGuide for NoMotionGuide {
    fn kind(&self, _unit_id: u32) -> MotionKind {
        MotionKind::Destination
    }
    fn exchanges_claims(&self) -> bool {
        true
    }
    fn next_position(
        &mut self,
        _request: MotionRequest,
        _budget: &mut SweepBudget,
    ) -> Option<Vec2> {
        None
    }
}

impl MotionRequest {
    /// Straight authored motion for an eligible model refusal. This does not
    /// inspect neighbors, and still passes through final exact certification.
    pub(crate) fn preferred_position(self) -> Vec2 {
        let delta = self.goal.as_dvec2() - self.position.as_dvec2();
        let distance = delta.length();
        let limit = f64::from(self.speed) * f64::from(self.dt);
        if !distance.is_finite() || !limit.is_finite() || limit <= 0. {
            return self.position;
        }
        let to = super::step(
            &super::MovementInput {
                position: self.position,
                target: self.goal,
                speed: self.speed,
            },
            self.dt,
        )
        .position;
        if !to.is_finite() {
            return self.position;
        }
        if !self.bounds.is_finite() || !self.radius.is_finite() || self.radius <= 0. {
            return self.position;
        }
        Vec2::new(
            to.x.clamp(self.radius, (self.bounds.x - self.radius).max(self.radius)),
            to.y.clamp(self.radius, (self.bounds.y - self.radius).max(self.radius)),
        )
    }
}

pub(super) fn guided_position(
    unit: &super::MovementFrameUnit,
    goal: Vec2,
    bounds: Vec2,
    dt: f32,
    guide: &mut dyn MotionGuide,
    budget: &mut SweepBudget,
) -> Option<Vec2> {
    let request = MotionRequest {
        unit_id: unit.unit_id,
        position: unit.position,
        goal,
        speed: unit.speed.unwrap_or(0.),
        radius: unit.radius,
        bounds,
        dt,
        kind: guide.kind(unit.unit_id),
    };
    let to = guide.next_position(request, budget)?;
    // A malformed modeled result must hold, never revive legacy separation.
    let valid = to.is_finite()
        && request.position.is_finite()
        && request.bounds.is_finite()
        && request.radius.is_finite()
        && request.radius > 0.
        && request.speed.is_finite()
        && request.speed >= 0.
        && dt.is_finite()
        && dt >= 0.
        && to.x >= request.radius
        && to.y >= request.radius
        && to.x <= (bounds.x - request.radius).max(request.radius)
        && to.y <= (bounds.y - request.radius).max(request.radius);
    if !valid {
        return Some(unit.position);
    }
    let ulp = |v: f32| {
        let v = v.abs();
        if v.next_up().is_finite() {
            f64::from(v.next_up()) - f64::from(v)
        } else {
            f64::from(v) - f64::from(v.next_down())
        }
    };
    let rounding = 4. * (ulp(unit.position.x) + ulp(unit.position.y) + ulp(to.x) + ulp(to.y));
    let distance = unit.position.as_dvec2().distance(to.as_dvec2());
    Some(
        if distance <= f64::from(request.speed) * f64::from(dt) + rounding {
            to
        } else {
            unit.position
        },
    )
}

#[cfg(test)]
mod tests;
