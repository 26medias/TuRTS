// Cost of design 6.3 (rev 2): hex pool, reserved-cell skip, banded order-keeping assignment.
use std::time::Instant;
#[derive(Clone, Copy)] struct V(f32, f32);
fn lcg(s: &mut u64) -> f32 { *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((*s >> 40) as f32) / ((1u64 << 24) as f32) }
fn run(n: usize, reserved_n: usize, naive_reserved: bool) -> f64 {
    let r = 0.5f32; let s = 2.0 * r + 0.1; let p = V(2048.0, 2048.0);
    let mut seed = 5u64;
    let units: Vec<V> = (0..n).map(|_| V(500.0 + lcg(&mut seed) * 400.0, 500.0 + lcg(&mut seed) * 400.0)).collect();
    // reserved slots: a resting block of other units next to P
    let reserved: Vec<V> = (0..reserved_n).map(|k| V(p.0 + 10.0 + s * (k % 100) as f32, p.1 + s * (k / 100) as f32)).collect();
    let t = Instant::now();
    // grid of reserved cells (cell size s)
    let key = |q: V| (((q.0 / s) as i64) << 32) ^ ((q.1 / s) as i64 & 0xffff_ffff);
    let mut grid: std::collections::BTreeMap<i64, Vec<V>> = Default::default();
    if !naive_reserved { for q in &reserved { grid.entry(key(*q)).or_default().push(*q); } }
    let want = n + std::cmp::max(6, (n + 3) / 4);
    let mut k = ((want as f32 / 2.6).sqrt() as i32) + 2;
    let mut pool: Vec<V>;
    loop {
        pool = Vec::new();
        for i in -k..=k { for j in -k..=k {
            let q = V(p.0 + s * (i as f32 + 0.5 * j as f32), p.1 + s * 0.8660254 * j as f32);
            let blocked = if naive_reserved { reserved.iter().any(|c| { let dx = c.0 - q.0; let dy = c.1 - q.1; dx * dx + dy * dy < s * s }) } else {
                let (cx, cy) = ((q.0 / s) as i64, (q.1 / s) as i64); let mut b = false;
                for ox in -1..=1 { for oy in -1..=1 { if let Some(v) = grid.get(&(((cx + ox) << 32) ^ ((cy + oy) & 0xffff_ffff))) { if v.iter().any(|c| { let dx = c.0 - q.0; let dy = c.1 - q.1; dx * dx + dy * dy < s * s }) { b = true; } } } } b };
            if !blocked { pool.push(q); }
        } }
        if pool.len() >= want { break; } k += 2;
    }
    let d2 = |q: &V| (q.0 - p.0).powi(2) + (q.1 - p.1).powi(2);
    pool.sort_by(|a, b| d2(a).total_cmp(&d2(b)).then((a.1 - p.1).atan2(a.0 - p.0).total_cmp(&(b.1 - p.1).atan2(b.0 - p.0))));
    let first = &pool[..n];
    let (cx, cy) = units.iter().fold((0.0, 0.0), |a, u| (a.0 + u.0, a.1 + u.1));
    let (dx, dy) = (p.0 - cx / n as f32, p.1 - cy / n as f32); let l = (dx * dx + dy * dy).sqrt(); let (dx, dy) = (dx / l, dy / l);
    let along = |q: &V| q.0 * dx + q.1 * dy; let lat = |q: &V| -q.0 * dy + q.1 * dx;
    let mut su: Vec<usize> = (0..n).collect(); su.sort_by(|a, b| along(&units[*b]).total_cmp(&along(&units[*a])).then(a.cmp(b)));
    let mut ss: Vec<usize> = (0..n).collect(); ss.sort_by(|a, b| along(&first[*b]).total_cmp(&along(&first[*a])).then(a.cmp(b)));
    let w = ((n as f32).sqrt().round() as usize).max(1); let mut out = vec![0usize; n];
    for b in (0..n).step_by(w) { let e = (b + w).min(n);
        let mut us = su[b..e].to_vec(); us.sort_by(|x, y| lat(&units[*x]).total_cmp(&lat(&units[*y])).then(x.cmp(y)));
        let mut ks = ss[b..e].to_vec(); ks.sort_by(|x, y| lat(&first[*x]).total_cmp(&lat(&first[*y])).then(x.cmp(y)));
        for (u, q) in us.iter().zip(ks.iter()) { out[*u] = *q; } }
    std::hint::black_box(&out);
    t.elapsed().as_secs_f64() * 1e3
}
fn main() {
    for n in [40usize, 1000, 4096, 8192, 16384] { println!("banded N={n}: grid-reserved(5000) {:.3} ms; naive-reserved(5000) {:.3} ms", run(n, 5000, false), run(n, 5000, true)); }
}
