//! Equivalence to the former tree-based terrain delta implementation.
use super::*;

fn tree_reference(old: &[(u32, u64)], new: &[(u32, u64)]) -> (Vec<(u32, u64)>, Vec<u32>) {
    let old: BTreeMap<_, _> = old.iter().map(|row| (row.0, *row)).collect();
    let new: BTreeMap<_, _> = new.iter().map(|row| (row.0, *row)).collect();
    (
        new.iter()
            .filter(|(key, row)| old.get(key) != Some(row))
            .map(|(_, row)| *row)
            .collect(),
        old.keys()
            .filter(|key| !new.contains_key(key))
            .copied()
            .collect(),
    )
}

#[test]
fn canonical_delta_merge_matches_old_trees_for_insert_remove_and_replace() {
    let mut state = 0x73e0_2b19_a46d_5901u64;
    for iteration in 0..512 {
        let mut old = Vec::new();
        let mut new = Vec::new();
        for key in 0..128u32 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            if state & 1 != 0 {
                old.push((key, state % 11));
            }
            if state & 2 != 0 {
                new.push((key, (state >> 8) % 11));
            }
        }
        assert_eq!(
            changed(&old, &new, |row| row.0, |a, b| a == b),
            tree_reference(&old, &new),
            "iteration{iteration}"
        );
    }
}

#[test]
fn noncanonical_delta_inputs_keep_old_last_duplicate_and_sort_rules() {
    let cases = [
        vec![],
        vec![(1, 1)],
        vec![(2, 1), (1, 2)],
        vec![(1, 1), (1, 2)],
        vec![(3, 3), (1, 1), (3, 7), (2, 2)],
    ];
    for old in &cases {
        for new in &cases {
            assert_eq!(
                changed(old, new, |row| row.0, |a, b| a == b),
                tree_reference(old, new)
            );
        }
    }
}

#[test]
fn canonical_merge_retains_current_timestamp_suppression_and_stale_updates() {
    let record = |tick, current, elevation| TerrainSurfaceRecord {
        cell: TerrainCell { x: 1, y: 2 },
        elevation,
        material: Some(TerrainMaterialId(0)),
        provenance: Some(turts_mod_api::TerrainProvenance::Natural),
        observed_tick: tick,
        current,
    };
    let base = TerrainSnapshot {
        materials: vec!["example:stone".into()],
        surfaces: vec![record(1, true, 0)],
        anchors: vec![],
        work: vec![],
    };
    let mut next = base.clone();
    next.surfaces[0] = record(2, true, 0);
    assert!(delta(&Some(base.clone()), &Some(next)).is_none());
    let mut stale = base.clone();
    stale.surfaces[0] = record(2, false, 0);
    assert_eq!(
        delta(&Some(base.clone()), &Some(stale)).unwrap().surfaces,
        vec![record(2, false, 0)]
    );
    let mut edited = base.clone();
    edited.surfaces[0] = record(2, true, 1);
    assert_eq!(
        delta(&Some(base), &Some(edited)).unwrap().surfaces,
        vec![record(2, true, 1)]
    );
}
