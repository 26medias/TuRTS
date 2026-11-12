//! Throwaway G1 micro-benchmark: `within` before (ceil rings) and after
//! (bounding box). Not committed. Run in release with --nocapture.
#![allow(dead_code)]

#[path = "/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/g1/grid_old.rs"]
mod old;
#[path = "/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/g1/grid_new.rs"]
mod new;

use bevy_math::Vec2;
use std::hint::black_box;
use std::time::Instant;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1_u32 << 24) as f32
    }
}

const UNITS: usize = 5_000;
const SPAN: f32 = 200.0;
const QUERIES: usize = 200_000;
const REPEATS: usize = 5;

macro_rules! bench {
    ($module:ident, $positions:expr, $queries:expr, $cell:expr, $reach:expr) => {{
        let points: Vec<$module::GridPoint> = $positions
            .iter()
            .enumerate()
            .map(|(id, position)| $module::GridPoint {
                id: id as u32,
                position: *position,
            })
            .collect();
        let grid = $module::UniformGrid::new(&points, $cell);
        let mut visited = 0usize;
        let mut found = 0usize;
        for query in $queries.iter() {
            found += grid.within(&points, *query, $reach).len();
            visited += grid.visited_cells();
        }
        let mut best = f64::MAX;
        for _ in 0..REPEATS {
            let start = Instant::now();
            for query in $queries.iter() {
                black_box(grid.within(&points, black_box(*query), black_box($reach)));
            }
            best = best.min(start.elapsed().as_secs_f64());
        }
        (
            visited as f64 / $queries.len() as f64,
            found as f64 / $queries.len() as f64,
            best * 1e9 / $queries.len() as f64,
        )
    }};
}

#[test]
fn zz_g1_within_cost() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let positions: Vec<Vec2> = (0..UNITS)
        .map(|_| Vec2::new(rng.unit() * SPAN, rng.unit() * SPAN))
        .collect();
    let queries: Vec<Vec2> = (0..QUERIES)
        .map(|_| Vec2::new(rng.unit() * SPAN, rng.unit() * SPAN))
        .collect();
    // The usual case: cell 2 x max_radius, reach r + max_radius.
    let max_radius = 0.6_f32;
    let cell = 2.0 * max_radius;
    for ratio in [1.0_f32, 2.0] {
        let reach = ratio * cell;
        let (old_cells, old_found, old_ns) = bench!(old, positions, queries, cell, reach);
        let (new_cells, new_found, new_ns) = bench!(new, positions, queries, cell, reach);
        println!(
            "G1BENCH units {UNITS} span {SPAN} queries {QUERIES} best-of {REPEATS} cell {cell} reach {reach} reach/cell {ratio}: \
             before cells/query {old_cells:.3} found/query {old_found:.4} ns/query {old_ns:.1}; \
             after cells/query {new_cells:.3} found/query {new_found:.4} ns/query {new_ns:.1}"
        );
    }
}
