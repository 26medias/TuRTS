//! Manual timing of the production movement kernel. Run with an exclusive lock.
use bevy_math::Vec2;
use std::collections::BTreeMap;
use turts_sim::movement::{
    MoveProgress, MovementActivity, MovementFrame, MovementFrameUnit, MovementPool,
    step_frame_at_tick,
};

#[test]
#[ignore = "manual release workload"]
fn frame_contact_density_production_probe() {
    let tiny = std::env::var("DENSITY_RADIUS").as_deref() == Ok("tiny");
    let side: usize = std::env::var("DENSITY_SIDE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(71);
    let radius = if tiny { 0.001_f32 } else { 0.4_f32 };
    let spacing = radius * 2.1;
    let upper = 2.0 + side as f32 * spacing;
    let slots: Vec<_> = (0..100)
        .map(|i| {
            let index = side * side - 100 + i;
            Vec2::new(
                1.0 + (index % side) as f32 * spacing,
                1.0 + (index / side) as f32 * spacing,
            )
        })
        .collect();
    let base = slots[50];
    let units = (0..side * side)
        .map(|i| {
            let position = Vec2::new(
                1.0 + (i % side) as f32 * spacing,
                1.0 + (i / side) as f32 * spacing,
            );
            let moving = i < 100;
            MovementFrameUnit {
                unit_id: i as u32 + 1,
                owner_slot: 0,
                unit_type: 1,
                position,
                radius,
                speed: moving.then_some(1.0),
                target: moving.then(|| slots[i]),
                progress: moving.then(|| MoveProgress {
                    pool_id: 7,
                    slot: slots[i],
                    activity: MovementActivity::Moving,
                    pass_mode: true,
                    stalling_on_group: false,
                    stalled_ticks: std::env::var("DENSITY_STALL")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(60),
                    best_distance: 0.0,
                    claim_count: 0,
                    follow_origin: None,
                }),
                plain_progress: None,
                physical_escape: None,
            }
        })
        .collect();
    let frame = MovementFrame {
        bounds: Vec2::splat(upper + 10.0 * spacing + 2.0),
        tick_hz: 20,
        units,
        pools: BTreeMap::from([(
            7,
            MovementPool {
                pool_id: 7,
                point: base,
                spacing,
                primary_count: 100,
                slots,
            },
        )]),
        recovery: BTreeMap::new(),
    };
    let mut times = Vec::new();
    for sample in 0..6 {
        let start = std::time::Instant::now();
        let outputs = step_frame_at_tick(&frame, 0.05, 1);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(outputs.len(), side * side);
        std::hint::black_box(outputs);
        eprintln!(
            "PRODUCTION tiny={tiny} bodies={} movers=100 sample={sample} ms={ms:.3}",
            side * side
        );
        if sample > 0 {
            times.push(ms);
        }
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "PRODUCTION_SUMMARY tiny={tiny} bodies={} median_ms={:.3} max_ms={:.3}",
        side * side,
        times[times.len() / 2],
        times[times.len() - 1]
    );
}
