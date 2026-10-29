// Cost probe: design 6.3 greedy assignment (O(N^2)) and 6.4 uniform-grid separation per tick.
use std::time::Instant;
#[derive(Clone, Copy)] struct V(f32, f32);
fn d2(a: V, b: V) -> f32 { let x = a.0 - b.0; let y = a.1 - b.1; x * x + y * y }
fn lcg(s: &mut u64) -> f32 { *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((*s >> 40) as f32) / ((1u64 << 24) as f32) }
fn spread(n: usize) -> (f64, u64) {
    let r = 0.5f32; let s = 2.0 * r * 1.05; let p = V(512.0, 512.0);
    let k = ((n as f32).sqrt() as i32) + 4;
    let mut slots = Vec::new();
    for i in -k..=k { for j in -k..=k { slots.push(V(p.0 + s * (i as f32 + 0.5 * j as f32), p.1 + s * 0.8660254 * j as f32)); } }
    slots.sort_by(|a, b| d2(*a, p).total_cmp(&d2(*b, p)).then((a.1 - p.1).atan2(a.0 - p.0).total_cmp(&(b.1 - p.1).atan2(b.0 - p.0))));
    slots.truncate(n);
    let mut seed = 7u64;
    let mut units: Vec<(f32, usize, V)> = (0..n).map(|i| { let u = V(100.0 + lcg(&mut seed) * 200.0, 100.0 + lcg(&mut seed) * 200.0); (d2(u, p), i, u) }).collect();
    let t = Instant::now();
    units.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut free = vec![true; n]; let mut sum = 0u64;
    for (_, _, u) in &units {
        let mut best = usize::MAX; let mut bd = f32::INFINITY;
        for (k, q) in slots.iter().enumerate() { if free[k] { let d = d2(*u, *q); if d < bd { bd = d; best = k; } } }
        free[best] = false; sum += best as u64;
    }
    (t.elapsed().as_secs_f64() * 1e3, sum)
}
fn avoid(n: usize, ticks: usize) -> f64 {
    let r = 0.5f32; let cell = 2.0 * r; let side = ((n as f32) * 4.0).sqrt(); // density: 1 unit per 4 m^2
    let mut seed = 11u64;
    let mut pos: Vec<V> = (0..n).map(|_| V(lcg(&mut seed) * side, lcg(&mut seed) * side)).collect();
    let tgt: Vec<V> = (0..n).map(|_| V(lcg(&mut seed) * side, lcg(&mut seed) * side)).collect();
    let g = (side / cell) as usize + 1;
    let step = 3.0f32 / 20.0;
    let t = Instant::now();
    for _ in 0..ticks {
        let mut head = vec![u32::MAX; g * g]; let mut next = vec![u32::MAX; n];
        for i in 0..n { let c = ((pos[i].1 / cell) as usize).min(g - 1) * g + ((pos[i].0 / cell) as usize).min(g - 1); next[i] = head[c]; head[c] = i as u32; }
        for i in 0..n {
            let (dx, dy) = (tgt[i].0 - pos[i].0, tgt[i].1 - pos[i].1); let d = (dx * dx + dy * dy).sqrt().max(1e-6);
            let m = step.min(d); let (mut vx, mut vy) = (dx / d * m, dy / d * m);
            let cx = ((pos[i].0 / cell) as isize).min(g as isize - 1); let cy = ((pos[i].1 / cell) as isize).min(g as isize - 1);
            for oy in -1..=1 { for ox in -1..=1 {
                let (x, y) = (cx + ox, cy + oy); if x < 0 || y < 0 || x >= g as isize || y >= g as isize { continue; }
                let mut j = head[y as usize * g + x as usize];
                while j != u32::MAX { let jj = j as usize; if jj != i { let ex = pos[i].0 - pos[jj].0; let ey = pos[i].1 - pos[jj].1; let e = (ex * ex + ey * ey).sqrt(); let lim = 2.0 * r + 0.1; if e > 1e-9 && e < lim { vx += ex / e * (lim - e); vy += ey / e * (lim - e); } } j = next[jj]; }
            } }
            let v = (vx * vx + vy * vy).sqrt(); if v > step { vx *= step / v; vy *= step / v; }
            pos[i].0 = (pos[i].0 + vx).clamp(0.0, side); pos[i].1 = (pos[i].1 + vy).clamp(0.0, side);
        }
    }
    let s: f32 = pos.iter().map(|p| p.0 + p.1).sum(); std::hint::black_box(s);
    t.elapsed().as_secs_f64() * 1e3 / ticks as f64
}
fn main() {
    for n in [40usize, 1000, 4096, 8192] { let (ms, s) = spread(n); println!("spread N={n}: {ms:.3} ms (checksum {s})"); }
    for n in [1000usize, 5000] { println!("avoid N={n}: {:.3} ms per tick (mean of 200 ticks)", avoid(n, 200)); }
}
