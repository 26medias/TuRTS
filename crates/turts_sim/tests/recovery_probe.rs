//! Manual probe: release timing and allocation peak of blocked recovery (stress scene).
use bevy_math::Vec2;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use turts_sim::movement::{
    MoveProgress, MovementActivity, MovementFrame, MovementFrameUnit, MovementPool,
    step_frame_at_tick,
};

struct Counting;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let now = CURRENT.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
        PEAK.fetch_max(now, Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CURRENT.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn body(unit_id: u32, position: Vec2, radius: f32) -> MovementFrameUnit {
    MovementFrameUnit {
        unit_id,
        owner_slot: 0,
        unit_type: 1,
        position,
        radius,
        speed: Some(1.0),
        target: None,
        progress: None,
        plain_progress: None,
        physical_escape: None,
    }
}

fn stress(movers: usize) -> MovementFrame {
    let goal = Vec2::splat(0.5);
    let mut units = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            units.push(body(
                units.len() as u32 + 1,
                Vec2::new(x as f32 + 0.5, y as f32 + 0.5),
                0.6,
            ));
        }
    }
    for i in 0..movers {
        let mut unit = body(
            units.len() as u32 + 1,
            Vec2::new((i % 32) as f32 + 0.5, 20.0 + (i / 32) as f32),
            0.6,
        );
        unit.target = Some(goal);
        unit.progress = Some(MoveProgress {
            pool_id: 7,
            slot: goal,
            activity: MovementActivity::Blocked,
            pass_mode: true,
            stalling_on_group: false,
            stalled_ticks: 60,
            best_distance: 1.0,
            claim_count: 3,
            follow_origin: None,
        });
        units.push(unit);
    }
    MovementFrame {
        bounds: Vec2::splat(16.0),
        tick_hz: 20,
        units,
        pools: BTreeMap::from([(
            7,
            MovementPool {
                pool_id: 7,
                point: goal,
                spacing: 1.1,
                primary_count: movers as u32,
                slots: vec![goal],
            },
        )]),
        recovery: BTreeMap::new(),
    }
}

#[test]
#[ignore = "manual release probe"]
fn recovery_stress_probe() {
    let movers: usize = std::env::var("ACTORS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let ticks: u32 = std::env::var("TICKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40);
    let mut frame = stress(movers);
    let mut times = Vec::new();
    let mut peak_extra = 0usize;
    for tick in 1..=ticks {
        let before = CURRENT.load(Ordering::SeqCst);
        PEAK.store(before, Ordering::SeqCst);
        let start = std::time::Instant::now();
        let outputs = step_frame_at_tick(&frame, 0.05, tick);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        peak_extra = peak_extra.max(PEAK.load(Ordering::SeqCst).saturating_sub(before));
        frame.recovery = outputs
            .iter()
            .filter_map(|o| o.recovery.as_deref().map(|j| (o.unit_id, *j)))
            .collect();
        for (unit, output) in frame.units.iter_mut().zip(&outputs) {
            unit.position = output.position;
            unit.target = output.target;
            unit.progress = output.progress.clone();
            unit.plain_progress = output.plain_progress.clone();
            unit.physical_escape = output.physical_escape.clone();
        }
    }
    let mut sorted = times.clone();
    sorted.sort_by(f64::total_cmp);
    eprintln!(
        "PROBE actors={movers} ticks={ticks} first_ms={:.3} median_ms={:.3} max_ms={:.3} mean_ms={:.3} alloc_peak_extra={peak_extra}",
        times[0],
        sorted[sorted.len() / 2],
        sorted[sorted.len() - 1],
        times.iter().sum::<f64>() / times.len() as f64
    );
}
