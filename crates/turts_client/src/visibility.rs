//! Client knowledge validation. Staged coverage never changes the active view.
use std::collections::HashSet;

use turts_mod_api::{
    KnowledgeDelta, KnowledgeSnapshot, MAX_REMEMBERED_SUBJECTS, MAX_VISIBILITY_CELLS,
    ObservationPhase, WorldPos,
};

/// The same three states drive ground, minimap, and browser observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogCell {
    Unexplored,
    Remembered,
    Observed,
}

pub fn cell_state(knowledge: &KnowledgeSnapshot, cell: u32) -> FogCell {
    if bit(&knowledge.observed, cell) {
        FogCell::Observed
    } else if bit(&knowledge.explored, cell) {
        FogCell::Remembered
    } else {
        FogCell::Unexplored
    }
}

pub fn position_state(knowledge: &KnowledgeSnapshot, position: WorldPos) -> Option<FogCell> {
    let grid = &knowledge.grid;
    if !position.x.is_finite()
        || !position.y.is_finite()
        || position.x < 0.0
        || position.y < 0.0
        || position.x > grid.size.x
        || position.y > grid.size.y
    {
        return None;
    }
    let x = ((position.x / grid.cell_size).floor() as u32).min(grid.columns - 1);
    let y = ((position.y / grid.cell_size).floor() as u32).min(grid.rows - 1);
    Some(cell_state(knowledge, y * grid.columns + x))
}

pub fn valid_snapshot(knowledge: &KnowledgeSnapshot) -> bool {
    let grid = &knowledge.grid;
    let cells = u64::from(grid.columns) * u64::from(grid.rows);
    if !grid.size.x.is_finite()
        || !grid.size.y.is_finite()
        || grid.size.x <= 0.0
        || grid.size.y <= 0.0
        || !grid.cell_size.is_finite()
        || grid.cell_size <= 0.0
        || cells == 0
        || cells > MAX_VISIBILITY_CELLS
        || (f64::from(grid.size.x) / f64::from(grid.cell_size)).ceil() != f64::from(grid.columns)
        || (f64::from(grid.size.y) / f64::from(grid.cell_size)).ceil() != f64::from(grid.rows)
        || knowledge.basis.phase != ObservationPhase::Completed
    {
        return false;
    }
    let bytes = cells.div_ceil(8) as usize;
    if knowledge.observed.len() != bytes
        || knowledge.explored.len() != bytes
        || knowledge
            .observed
            .iter()
            .zip(&knowledge.explored)
            .any(|(a, b)| a & !b != 0)
        || (cells % 8 != 0
            && (knowledge.observed[bytes - 1] | knowledge.explored[bytes - 1]) >> (cells % 8) != 0)
        || knowledge.remembered_units.len() + knowledge.remembered_nodes.len()
            > MAX_REMEMBERED_SUBJECTS as usize
    {
        return false;
    }
    let mut units = HashSet::new();
    let mut nodes = HashSet::new();
    knowledge.remembered_units.iter().all(|row| {
        units.insert(row.unit)
            && row.last_seen_tick <= knowledge.basis.tick
            && position_state(knowledge, row.position)
                .is_some_and(|state| state != FogCell::Observed)
    }) && knowledge.remembered_nodes.iter().all(|row| {
        nodes.insert(row.node)
            && row.last_seen_tick <= knowledge.basis.tick
            && position_state(knowledge, row.position)
                .is_some_and(|state| state != FogCell::Observed)
    })
}

/// Replace coverage and stale rows on a private candidate only.
pub fn stage_delta(
    current: &KnowledgeSnapshot,
    delta: &KnowledgeDelta,
) -> Option<KnowledgeSnapshot> {
    if delta.viewer != current.viewer
        || delta.basis.match_id != current.basis.match_id
        || delta.basis.map != current.basis.map
        || delta.basis.tick <= current.basis.tick
        || delta.basis.phase != ObservationPhase::Completed
        || (current.retention_truncated && !delta.retention_truncated)
    {
        return None;
    }
    for ids in [
        &delta.hidden_units,
        &delta.hidden_nodes,
        &delta.revealed_units,
        &delta.revealed_nodes,
        &delta.forgotten_units,
        &delta.forgotten_nodes,
    ] {
        if ids.iter().copied().collect::<HashSet<_>>().len() != ids.len() {
            return None;
        }
    }
    if delta
        .hidden_units
        .iter()
        .any(|id| delta.revealed_units.contains(id))
        || delta
            .hidden_nodes
            .iter()
            .any(|id| delta.revealed_nodes.contains(id))
    {
        return None;
    }
    let mut next = current.clone();
    next.basis = delta.basis.clone();
    next.retention_truncated = delta.retention_truncated;
    let cells = next.grid.columns * next.grid.rows;
    let mut end = 0;
    for run in &delta.coverage {
        let stop = run.first_cell.checked_add(run.cell_count)?;
        if run.cell_count == 0
            || run.first_cell < end
            || stop > cells
            || (run.observed && !run.explored)
        {
            return None;
        }
        for cell in run.first_cell..stop {
            if bit(&next.explored, cell) && !run.explored {
                return None;
            }
            set_bit(&mut next.observed, cell, run.observed);
            set_bit(&mut next.explored, cell, run.explored);
        }
        end = stop;
    }
    next.remembered_units.retain(|row| {
        !delta.forgotten_units.contains(&row.unit)
            && !delta.revealed_units.contains(&row.unit)
            && !delta
                .remembered_units
                .iter()
                .any(|changed| changed.unit == row.unit)
    });
    next.remembered_nodes.retain(|row| {
        !delta.forgotten_nodes.contains(&row.node)
            && !delta.revealed_nodes.contains(&row.node)
            && !delta
                .remembered_nodes
                .iter()
                .any(|changed| changed.node == row.node)
    });
    next.remembered_units
        .extend(delta.remembered_units.iter().cloned());
    next.remembered_nodes
        .extend(delta.remembered_nodes.iter().cloned());
    valid_snapshot(&next).then_some(next)
}

fn bit(mask: &[u8], cell: u32) -> bool {
    mask.get(cell as usize / 8)
        .is_some_and(|byte| byte & (1 << (cell % 8)) != 0)
}
fn set_bit(mask: &mut [u8], cell: u32, value: bool) {
    let byte = &mut mask[cell as usize / 8];
    let bit = 1 << (cell % 8);
    if value {
        *byte |= bit;
    } else {
        *byte &= !bit;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::{ContentId, CoverageRun, KnowledgeBasis, VisibilityGrid};

    fn knowledge() -> KnowledgeSnapshot {
        KnowledgeSnapshot {
            basis: KnowledgeBasis {
                match_id: 7,
                map: ContentId::parse("test:map").unwrap(),
                tick: 1,
                phase: ObservationPhase::Completed,
            },
            viewer: 0,
            grid: VisibilityGrid {
                size: WorldPos { x: 3.0, y: 2.0 },
                cell_size: 1.0,
                columns: 3,
                rows: 2,
            },
            observed: vec![1],
            explored: vec![3],
            remembered_units: vec![],
            remembered_nodes: vec![],
            retention_truncated: false,
        }
    }
    fn delta() -> KnowledgeDelta {
        KnowledgeDelta {
            basis: KnowledgeBasis {
                tick: 2,
                ..knowledge().basis
            },
            viewer: 0,
            coverage: vec![],
            hidden_units: vec![],
            hidden_nodes: vec![],
            revealed_units: vec![],
            revealed_nodes: vec![],
            remembered_units: vec![],
            remembered_nodes: vec![],
            forgotten_units: vec![],
            forgotten_nodes: vec![],
            retention_truncated: false,
        }
    }
    #[test]
    fn fog_distinguishes_current_explored_and_unknown_at_edges() {
        let k = knowledge();
        assert!(valid_snapshot(&k));
        assert_eq!(cell_state(&k, 0), FogCell::Observed);
        assert_eq!(cell_state(&k, 1), FogCell::Remembered);
        assert_eq!(
            position_state(&k, WorldPos { x: 3.0, y: 2.0 }),
            Some(FogCell::Unexplored)
        );
        assert_eq!(position_state(&k, WorldPos { x: -0.1, y: 0.0 }), None);
    }
    #[test]
    fn invalid_coverage_tail_cannot_commit_valid_prefix() {
        let original = knowledge();
        let mut patch = delta();
        patch.coverage = vec![
            CoverageRun {
                first_cell: 0,
                cell_count: 1,
                observed: false,
                explored: true,
            },
            CoverageRun {
                first_cell: 5,
                cell_count: 2,
                observed: true,
                explored: true,
            },
        ];
        assert!(stage_delta(&original, &patch).is_none());
        assert_eq!(cell_state(&original, 0), FogCell::Observed);
    }
    #[test]
    fn explored_cells_and_retention_flag_cannot_revert() {
        let mut original = knowledge();
        let mut patch = delta();
        patch.coverage = vec![CoverageRun {
            first_cell: 1,
            cell_count: 1,
            observed: false,
            explored: false,
        }];
        assert!(stage_delta(&original, &patch).is_none());
        original.retention_truncated = true;
        assert!(stage_delta(&original, &delta()).is_none());
    }
    #[test]
    fn masks_must_be_bounded_with_zero_padding_and_observed_subset() {
        let mut k = knowledge();
        k.observed[0] = 0x80;
        assert!(!valid_snapshot(&k));
        k.observed = vec![4];
        assert!(!valid_snapshot(&k));
        k.observed.clear();
        assert!(!valid_snapshot(&k));
    }
}
