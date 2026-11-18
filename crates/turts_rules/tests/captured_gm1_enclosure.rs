#![allow(
    clippy::excessive_precision,
    reason = "Preserve exact captured f32 inputs for the geometry witness."
)]
//! Exact geometry witness from b16 GM1 Quarry tick2400, canonical allocation.
//! This is not a MovementFrame/Guide replay: no missing pool/history is invented.
//! All36 captured poses are unchanged from b979. Runtime source state remains in
//! target/mc-evidence/terrain-occlusion/final-foundation/feedback-combined/.
use turts_rules::terrain::{
    Ratio,
    geometry::{Budget, Cell, Grid, Point, Traversal, swept_disks_clear},
};

// (physical label handle for provenance only, x, y, radius, actually settled).
const BODIES: [(u32, f32, f32, f32, bool); 36] = [
    (1, 46.54999923706055, 18.024425506591797, 0.4, true), // small.0.0
    (2, 47.70000076293945, 20.01628303527832, 0.4, true),  // small.0.1
    (3, 48.849998474121094, 18.024425506591797, 0.4, true), // small.0.2
    (4, 51.150001525878906, 18.024425506591797, 0.4, true), // small.0.3
    (5, 53.45000076293945, 18.024425506591797, 0.4, true), // small.0.4
    (6, 54.599998474121094, 20.01628303527832, 0.4, true), // small.0.5
    (7, 45.400001525878906, 20.01628303527832, 0.4, true), // small.1.0
    (8, 46.54999923706055, 22.008142471313477, 0.4, true), // small.1.1
    (9, 50.0, 20.01628303527832, 0.4, true),               // small.1.2
    (10, 52.29999923706055, 20.01628303527832, 0.4, true), // small.1.3
    (11, 53.45000076293945, 22.008142471313477, 0.4, true), // small.1.4
    (12, 55.75, 22.008142471313477, 0.4, true),            // small.1.5
    (13, 44.25, 22.008142471313477, 0.4, true),            // small.2.0
    (14, 50.0, 24.0, 0.4, true),                           // small.2.1
    (15, 48.849998474121094, 22.008142471313477, 0.4, true), // small.2.2
    (16, 51.150001525878906, 22.008142471313477, 0.4, true), // small.2.3
    (17, 54.599998474121094, 24.0, 0.4, true),             // small.2.4
    (18, 56.900001525878906, 24.0, 0.4, true),             // small.2.5
    (19, 47.70000076293945, 24.0, 0.4, true),              // small.3.0
    (20, 48.849998474121094, 29.975574493408203, 0.4, true), // small.3.1
    (21, 48.849998474121094, 25.991857528686523, 0.4, true), // small.3.2
    (22, 52.29999923706055, 24.0, 0.4, true),              // small.3.3
    (23, 54.599998474121094, 27.98371696472168, 0.4, true), // small.3.4
    (24, 55.75, 25.991857528686523, 0.4, true),            // small.3.5
    (25, 45.400001525878906, 24.0, 1.1, true),             // large.0.0
    (26, 46.06489562988281, 27.235315322875977, 1.1, false), // large.0.1
    (27, 47.86237335205078, 28.503864288330078, 1.1, false), // large.0.2
    (28, 47.36805725097656, 25.462810516357422, 1.1, false), // large.0.3
    (29, 44.25, 25.991857528686523, 1.1, true),            // large.1.0
    (30, 45.74543380737305, 29.894134521484375, 1.1, false), // large.1.1
    (31, 50.0, 27.98371696472168, 1.1, true),              // large.1.2
    (32, 52.29999923706055, 27.98371696472168, 1.1, true), // large.1.3
    (33, 44.12495803833008, 28.272933959960938, 1.1, false), // large.2.0
    (34, 47.73109817504883, 30.974742889404297, 1.1, false), // large.2.1
    (35, 51.150001525878906, 29.975574493408203, 1.1, true), // large.2.2
    (36, 53.45000076293945, 29.975574493408203, 1.1, true), // large.2.3
];
const FROM: [f32; 2] = [47.36805725097656, 25.462810516357422];
const SLOT: [f32; 2] = [53.45000076293945, 25.991857528686523];
const PUBLIC: [f32; 2] = [50., 24.];
const RADIUS: f32 = 1.1;
// A geometric witness, not a claim of globally nearest reachable parking.
const PARKING: [f32; 2] = [47.43, 25.49];
// Fixed physical clockwise/anticlockwise ring; handles never choose an outcome.
const RING: [[f32; 2]; 8] = [
    [52.29999923706055, 24.],
    [54.599998474121094, 24.],
    [55.75, 25.991857528686523],
    [54.599998474121094, 27.98371696472168],
    [52.29999923706055, 27.98371696472168],
    [50., 27.98371696472168],
    [48.849998474121094, 25.991857528686523],
    [50., 24.],
];
fn point(p: [f32; 2]) -> Point {
    Point {
        x: f64::from(p[0]),
        y: f64::from(p[1]),
    }
}
fn exact(v: f32) -> i128 {
    let x = f64::from(v) * 33_554_432.;
    assert_eq!(x.fract(), 0.);
    x as i128
}
fn cross(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> i128 {
    (exact(b[0]) - exact(a[0])) * (exact(c[1]) - exact(a[1]))
        - (exact(b[1]) - exact(a[1])) * (exact(c[0]) - exact(a[0]))
}
fn square_distance(a: [f32; 2], b: [f32; 2]) -> i128 {
    let dx = exact(a[0]) - exact(b[0]);
    let dy = exact(a[1]) - exact(b[1]);
    dx * dx + dy * dy
}
fn clear(from: [f32; 2], to: [f32; 2], radius: f64, centre: [f32; 2], other_radius: f64) -> bool {
    swept_disks_clear(
        point(from),
        point(to),
        radius,
        point(centre),
        point(centre),
        other_radius,
        &mut Budget::new(1),
    )
    .expect("bounded exact witness arithmetic")
}
#[test]
fn captured_free_private_slot_is_enclosed_by_own_settled_body_ring() {
    for reverse in [false, true] {
        let mut bodies = BODIES;
        if reverse {
            bodies.reverse();
            for b in &mut bodies {
                b.0 = 1000 - b.0;
            }
        }
        let find = |p: [f32; 2]| bodies.iter().find(|b| [b.1, b.2] == p).unwrap();
        assert!(
            bodies.iter().filter(|b| b.4).all(|b| clear(
                SLOT,
                SLOT,
                f64::from(RADIUS),
                [b.1, b.2],
                f64::from(b.3)
            )),
            "private slot is locally free, not occupied"
        );
        let mut outside = false;
        for i in 0..RING.len() {
            let a = RING[i];
            let b = RING[(i + 1) % RING.len()];
            let c = RING[(i + 2) % RING.len()];
            assert!(cross(a, b, c) >= 0, "simple convex physical cycle");
            assert!(
                cross(a, b, SLOT) > 0,
                "slot strictly inside all ring halfplanes"
            );
            outside |= cross(a, b, FROM) < 0;
            let aa = find(a);
            let bb = find(b);
            assert!(aa.4 && bb.4, "only actual settled holders contribute");
            let ar = f64::from(RADIUS) + f64::from(aa.3);
            let br = f64::from(RADIUS) + f64::from(bb.3);
            assert!(
                !clear(a, a, ar, b, br),
                "neighbor expanded discs strictly overlap, so their entire centre segment is forbidden"
            );
        }
        assert!(outside, "actor begins outside closed forbidden curve");
        // Convex closed centre cycle lies wholly in the open union of inflated
        // holder discs. Any continuous path from FROM to SLOT crosses that
        // forbidden curve. More waypoints cannot make this private slot reachable.
    }
}
#[test]
fn captured_exterior_parking_has_exact_safe_approach_without_moving_any_body() {
    for reverse in [false, true] {
        let mut bodies = BODIES;
        if reverse {
            bodies.reverse();
            for b in &mut bodies {
                b.0 = 1000 - b.0;
            }
        }
        for b in bodies {
            if [b.1, b.2] == FROM {
                continue;
            }
            assert!(
                clear(FROM, PARKING, f64::from(RADIUS), [b.1, b.2], f64::from(b.3)),
                "exact approach keeps every other captured disc fixed at {:?}",
                [b.1, b.2]
            );
        }
        assert!(square_distance(PARKING, PUBLIC) < square_distance(FROM, PUBLIC));
        let public_holder = BODIES.iter().find(|b| [b.1, b.2] == PUBLIC).unwrap();
        assert!(public_holder.4);
        assert!(
            !clear(
                PUBLIC,
                PUBLIC,
                f64::from(RADIUS),
                PUBLIC,
                f64::from(public_holder.3)
            ),
            "original public goal is actually held"
        );
        // The frozen GM1 map is AllVisible: x30..34 is the only ridge;
        // passage y20..28 is cut to0; all cells at this approach are height0.
        let grid = Grid {
            columns: 64,
            rows: 48,
            length_scale: 1000,
            cell_width: 1000,
            elevation_step: 250,
            floor: -8,
            ceiling: 8,
        };
        let surface = |c: Cell| {
            if c.x < 64 && c.y < 48 {
                Some(if (30..34).contains(&c.x) && !(20..28).contains(&c.y) {
                    4
                } else {
                    0
                })
            } else {
                None
            }
        };
        let result = grid.sweep(
            &surface,
            point(FROM),
            point(PARKING),
            f64::from(RADIUS),
            Traversal {
                max_step: 0,
                max_grade: Ratio {
                    numerator: 0,
                    denominator: 1,
                },
            },
            &mut Budget::new(16384),
        );
        assert!(
            result.result.is_ok(),
            "whole footprint permitted flat-terrain approach: {result:?}"
        );
        // No completion claim: the controller must select/retain an authorized
        // reachable arrival and physically reach it under final exact guards.
    }
}
