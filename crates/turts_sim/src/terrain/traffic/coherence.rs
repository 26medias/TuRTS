//! Frozen order-group alignment. Exact guards still decide actual movement.
use bevy_math::{DVec2, Vec2};
use std::cmp::Ordering;

const SMALL_MEAN_SQUARED: f64 = 1.0e-12;

/// The caller precharges every valid contributing row as a complete owner batch.
/// `actor` and `group` are lookup/equality keys, never physical tie breakers.
#[derive(Clone, Copy, Debug)]
pub(super) struct Member {
    pub actor: usize,
    pub owner: u8,
    pub group: u64,
    pub position: Vec2,
    pub radius: f32,
    pub speed: f32,
    pub goal: Vec2,
    pub preferred: DVec2,
    cell: [i64; 2],
    sum: DVec2,
    count: u16,
    located: bool,
}

impl Member {
    #[allow(
        clippy::too_many_arguments,
        reason = "The physical state is passed explicitly."
    )]
    pub fn new(
        actor: usize,
        owner: u8,
        group: u64,
        position: Vec2,
        radius: f32,
        speed: f32,
        goal: Vec2,
        preferred: DVec2,
    ) -> Self {
        Self {
            actor,
            owner,
            group,
            position,
            radius,
            speed,
            goal,
            preferred,
            cell: [0; 2],
            sum: DVec2::ZERO,
            count: 0,
            located: false,
        }
    }
}

fn direction(value: DVec2) -> Option<DVec2> {
    let length = value.length();
    if value.is_finite() && length.is_finite() && length > 0. {
        Some(value / length)
    } else {
        None
    }
}

pub(super) fn contribution(row: &Member) -> Option<DVec2> {
    if !row.position.is_finite()
        || !row.goal.is_finite()
        || !row.radius.is_finite()
        || row.radius <= 0.
        || !row.speed.is_finite()
        || row.speed <= 0.
    {
        return None;
    }
    direction(row.preferred)
}

pub(super) fn physical(a: &Member, b: &Member) -> Ordering {
    a.position
        .x
        .total_cmp(&b.position.x)
        .then(a.position.y.total_cmp(&b.position.y))
        .then(a.radius.total_cmp(&b.radius))
        .then(a.speed.total_cmp(&b.speed))
        .then(a.goal.x.total_cmp(&b.goal.x))
        .then(a.goal.y.total_cmp(&b.goal.y))
        .then(a.preferred.x.total_cmp(&b.preferred.x))
        .then(a.preferred.y.total_cmp(&b.preferred.y))
}

fn subgroup(a: &Member, b: &Member) -> Ordering {
    a.owner
        .cmp(&b.owner)
        .then(a.group.cmp(&b.group))
        .then(a.radius.total_cmp(&b.radius))
}

fn address(a: &Member, b: &Member) -> Ordering {
    subgroup(a, b)
        .then(a.located.cmp(&b.located))
        .then(a.cell.cmp(&b.cell))
}

/// Refuse extreme coordinates rather than alias cells by a saturating cast.
fn cell_coordinate(position: f64, origin: f64, width: f64) -> Option<i64> {
    let value = ((position - origin) / width).floor();
    if value.is_finite() && (0. ..=4_503_599_627_370_494.).contains(&value) {
        Some(value as i64)
    } else {
        None
    }
}

/// Complete cell aggregates reuse the bounded Member allocation.
pub(super) fn prepare_cells(members: &mut [Member]) {
    for row in &mut *members {
        row.located = false;
        row.count = 0;
        row.sum = DVec2::ZERO;
        row.cell = [0; 2];
    }
    if members.len() > super::MEMBERS {
        return;
    }
    members.sort_unstable_by(|a, b| subgroup(a, b).then_with(|| physical(a, b)));
    let mut start = 0;
    while start < members.len() {
        let mut end = start + 1;
        while end < members.len() && subgroup(&members[start], &members[end]).is_eq() {
            end += 1;
        }
        let rows = &mut members[start..end];
        let origin = rows
            .iter()
            .filter(|r| contribution(r).is_some())
            .fold(DVec2::splat(f64::INFINITY), |p, r| {
                p.min(r.position.as_dvec2())
            });
        let width = 4. * f64::from(rows[0].radius);
        let mut complete = true;
        for row in &mut *rows {
            if contribution(row).is_none() {
                continue;
            }
            let Some(x) = cell_coordinate(f64::from(row.position.x), origin.x, width) else {
                complete = false;
                break;
            };
            let Some(y) = cell_coordinate(f64::from(row.position.y), origin.y, width) else {
                complete = false;
                break;
            };
            row.cell = [x, y];
            row.located = true;
        }
        if !complete {
            for row in rows {
                row.located = false;
            }
        }
        start = end;
    }
    members.sort_unstable_by(|a, b| address(a, b).then_with(|| physical(a, b)));
    start = 0;
    while start < members.len() {
        let mut end = start + 1;
        while end < members.len() && address(&members[start], &members[end]).is_eq() {
            end += 1;
        }
        if members[start].located {
            let sum = members[start..end].iter().filter_map(contribution).sum();
            let count = u16::try_from(end - start).expect("bounded cell contributors");
            for row in &mut members[start..end] {
                row.sum = sum;
                row.count = count;
            }
        }
        start = end;
    }
}

/// Every cell query costs work, including an empty cell. Never return a prefix mean.
pub(super) fn local_mean(
    members: &[Member],
    index: usize,
    mut charge_cell: impl FnMut() -> bool,
) -> Option<DVec2> {
    let row = members.get(index)?;
    if !row.located {
        return None;
    }
    let mut sum = DVec2::ZERO;
    let mut count = 0usize;
    for dx in -1..=1 {
        for dy in -1..=1 {
            let mut key = *row;
            key.cell = [row.cell[0].checked_add(dx)?, row.cell[1].checked_add(dy)?];
            if !charge_cell() {
                return None;
            }
            let i = members.partition_point(|r| address(r, &key).is_lt());
            if let Some(found) = members.get(i).filter(|r| address(r, &key).is_eq()) {
                sum += found.sum;
                count += usize::from(found.count);
            }
        }
    }
    if count < 2 {
        return None;
    }
    let mean = sum / count as f64;
    if mean.length_squared() <= SMALL_MEAN_SQUARED {
        return None;
    }
    direction(mean)
}

/// Equal-weight alignment preserves the actor's preferred displacement length.
pub(super) fn blend(preferred: DVec2, mean: DVec2) -> Option<DVec2> {
    if !mean.length_squared().is_finite() || mean.length_squared() <= SMALL_MEAN_SQUARED {
        return None;
    }
    let own = direction(preferred)?;
    let group = direction(mean)?;
    if own.dot(group) <= 0.5 {
        return None;
    }
    Some(direction(own + group)? * preferred.length())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn member(actor: usize, x: f32, preferred: DVec2) -> Member {
        Member::new(
            actor,
            0,
            7,
            Vec2::new(x, 3.),
            0.3,
            100.,
            Vec2::new(-20., 3.),
            preferred,
        )
    }
    fn results(mut members: Vec<Member>) -> Vec<(u8, u32, Option<[u64; 2]>)> {
        prepare_cells(&mut members);
        let mut values = members
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    r.owner,
                    r.position.x.to_bits(),
                    local_mean(&members, i, || true).map(|m| [m.x.to_bits(), m.y.to_bits()]),
                )
            })
            .collect::<Vec<_>>();
        values.sort_unstable();
        values
    }
    #[test]
    fn canonical_complete_means_ignore_enumeration_and_handle_names() {
        let mut base = vec![
            member(1, 2., DVec2::new(-5., 0.)),
            member(2, 2.3, DVec2::new(-4., -1.)),
            member(3, 2.7, DVec2::new(-3., -0.5)),
        ];
        let mut other = member(4, 4., DVec2::Y);
        other.group = 8;
        base.push(other);
        let mut foreign = member(5, 5., DVec2::Y);
        foreign.owner = 1;
        base.push(foreign);
        foreign.actor = 6;
        foreign.position.x = 5.5;
        base.push(foreign);
        let expected = results(base.clone());
        assert_eq!(expected.iter().filter(|r| r.2.is_some()).count(), 5);
        for rotation in 0..base.len() {
            let mut rows = base.clone();
            rows.rotate_left(rotation);
            rows.reverse();
            for row in &mut rows {
                row.actor = 100 - row.actor;
                row.group += 1000;
            }
            assert_eq!(results(rows), expected);
        }
    }
    #[test]
    fn separated_opposite_flanks_keep_local_turns() {
        let mut rows = vec![
            member(1, 0., DVec2::new(-1., -1.)),
            member(2, 0.5, DVec2::new(-1., -1.)),
            member(3, 20., DVec2::new(-1., 1.)),
            member(4, 20.5, DVec2::new(-1., 1.)),
        ];
        prepare_cells(&mut rows);
        for (i, r) in rows.iter().enumerate() {
            let mean = local_mean(&rows, i, || true).unwrap();
            assert_eq!(mean.y.is_sign_positive(), r.position.x > 10.);
            assert!(mean.x < 0.);
        }
    }
    #[test]
    fn adjacent_cells_share_contributors_and_charge_empty_cells() {
        let mut rows = vec![member(1, 0., DVec2::X), member(2, 1.3, DVec2::Y)];
        prepare_cells(&mut rows);
        assert_ne!(rows[0].cell, rows[1].cell);
        for i in 0..2 {
            let mut spent = 0;
            let mean = local_mean(&rows, i, || {
                spent += 1;
                true
            })
            .unwrap();
            assert_eq!(spent, 9);
            assert!((mean - DVec2::splat(0.5f64.sqrt())).length() < 1e-12);
            for limit in 0..9 {
                let mut spent = 0;
                assert!(
                    local_mean(&rows, i, || {
                        if spent == limit {
                            return false;
                        }
                        spent += 1;
                        true
                    })
                    .is_none()
                );
                assert_eq!(spent, limit);
            }
        }
    }
    #[test]
    fn local_mean_weights_contributors_and_counts_self_once() {
        let mut rows = vec![
            member(1, 0., DVec2::X),
            member(2, 0.1, DVec2::X),
            member(3, 0.2, DVec2::X),
            member(4, 1.3, DVec2::Y),
        ];
        prepare_cells(&mut rows);
        let expected = DVec2::new(3., 1.).normalize();
        for i in 0..rows.len() {
            assert!((local_mean(&rows, i, || true).unwrap() - expected).length() < 1e-12);
        }
        // A single actor visits its own cell once and cannot become a group.
        rows.truncate(1);
        prepare_cells(&mut rows);
        let mut charges = 0;
        assert!(
            local_mean(&rows, 0, || {
                charges += 1;
                true
            })
            .is_none()
        );
        assert_eq!(charges, 9);
    }
    #[test]
    fn radius_owner_group_and_invalid_rows_do_not_supply_alignment() {
        let base = member(1, 0., DVec2::X);
        for variant in 0..7 {
            let mut other = member(2, 0.5, DVec2::Y);
            match variant {
                0 => other.radius = 0.4,
                1 => other.owner = 1,
                2 => other.group = 8,
                3 => other.preferred = DVec2::ZERO,
                4 => other.preferred = DVec2::splat(f64::NAN),
                5 => other.radius = -1.,
                _ => other.speed = 0.,
            }
            assert!(results(vec![base, other]).iter().all(|r| r.2.is_none()));
        }
        assert!(
            results(vec![base, member(2, 0.5, -DVec2::X)])
                .iter()
                .all(|r| r.2.is_none())
        );
        assert!(
            results(vec![base, member(2, 0.5, DVec2::new(-1., 1e-7))])
                .iter()
                .all(|r| r.2.is_none())
        );
    }
    #[test]
    fn complete_minimum_is_translation_invariant_and_extremes_refuse_whole_subgroup() {
        let mut rows = vec![
            member(1, 1.5, DVec2::X),
            member(2, 0., DVec2::Y),
            member(3, 0.5, DVec2::Y),
        ];
        prepare_cells(&mut rows);
        let expected: Vec<_> = (0..rows.len())
            .map(|i| local_mean(&rows, i, || true))
            .collect();
        assert!(expected.iter().all(Option::is_some));
        for row in &mut rows {
            row.position += Vec2::splat(128.);
        }
        prepare_cells(&mut rows);
        assert_eq!(
            (0..rows.len())
                .map(|i| local_mean(&rows, i, || true))
                .collect::<Vec<_>>(),
            expected
        );
        for row in &mut rows {
            row.radius = f32::MIN_POSITIVE;
        }
        prepare_cells(&mut rows);
        assert!(rows.iter().all(|r| !r.located));
        assert!(
            (0..rows.len()).all(|i| local_mean(&rows, i, || panic!("unlocated query")).is_none())
        );
        let max = 4_503_599_627_370_494.;
        assert_eq!(cell_coordinate(max, 0., 1.), Some(max as i64));
        assert_eq!(cell_coordinate(max + 1., 0., 1.), None);
        assert_eq!(cell_coordinate(-1., 0., 1.), None);
        assert_eq!(cell_coordinate(f64::INFINITY, 0., 1.), None);
        let mut boundary = vec![member(1, 0., DVec2::X), member(2, 0., DVec2::Y)];
        prepare_cells(&mut boundary);
        for row in &mut boundary {
            row.cell = [max as i64, max as i64];
        }
        assert!(local_mean(&boundary, 0, || true).is_some());
    }
    #[test]
    fn alignment_gate_and_small_mean_threshold_are_strict() {
        assert!(blend(DVec2::X, DVec2::new(0.5, (3f64.sqrt() * 0.5).next_up())).is_none());
        assert!(blend(DVec2::X, DVec2::new(0.50001, (3f64.sqrt() * 0.5).next_up())).is_some());
        assert!(blend(DVec2::X, DVec2::X * 1.0e-6).is_none());
        assert!(blend(DVec2::X, DVec2::X * 1.01e-6).is_some());
        assert!(blend(DVec2::X, -DVec2::X).is_none());
        assert!(blend(DVec2::ZERO, DVec2::X).is_none());
    }
    #[test]
    fn blending_preserves_short_and_full_preferred_lengths_and_goalward_progress() {
        for distance in [0.001, 0.1, 5.] {
            let preferred = DVec2::X * distance;
            let result = blend(preferred, DVec2::new(1., 0.5)).unwrap();
            assert!((result.length() - distance).abs() <= distance * 1.0e-12);
            assert!(result.x > 0.9 * distance && result.y > 0.);
        }
    }
}
