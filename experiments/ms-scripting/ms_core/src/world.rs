//! The mock world: units on a flat map, a uniform grid, seeded events, and request application.
//! This is the Rust engine work of a tick (W0). Positions are integers in milli-cells.

use crate::rng::Pcg32;

pub const PLAYERS: usize = 8;
pub const TYPES: usize = 4;
/// Map side in milli-cells (256 cells).
pub const MAP: i32 = 256_000;
/// Grid bucket side in milli-cells (8 cells).
pub const BUCKET: i32 = 8_000;
pub const GRID: i32 = MAP / BUCKET;
/// Movement for each tick in milli-cells.
pub const STEP: i32 = 150;
/// Ints in one event record: `[kind, player, unit_type, unit, extra, x, y]`.
pub const EVENT_INTS: usize = 7;

pub const EV_DIED: i64 = 1;
pub const EV_PRODUCED: i64 = 2;
pub const EV_RESEARCHED: i64 = 3;
pub const EV_TIMER: i64 = 4;

#[derive(Clone, Copy)]
pub struct Unit {
    pub x: i32,
    pub y: i32,
    pub tx: i32,
    pub ty: i32,
    pub owner: u8,
    pub utype: u8,
    pub stance: u8,
}

/// A request that a script made. The host applies it after the script call, with validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Grant { player: i64, amount: i64 },
    MoveTo { unit: i64, x: i64, y: i64 },
    MoveGroup { units: Vec<i64>, x: i64, y: i64 },
    Stance { unit: i64, stance: i64 },
    Alert { player: i64, code: i64 },
}

/// Event rate: a fixed count, or `min + below(spread)`.
#[derive(Clone, Copy)]
pub struct EventRate {
    pub min: u32,
    pub spread: u32,
}

pub struct World {
    pub units: Vec<Unit>,
    /// `bucket_start[b]..bucket_start[b + 1]` indexes `order`.
    pub bucket_start: Vec<u32>,
    /// Unit ids sorted by bucket, ascending id inside a bucket.
    pub order: Vec<u32>,
    pub credits: [i64; PLAYERS],
    pub counts: [[i64; TYPES]; PLAYERS],
    pub rng: Pcg32,
    pub tick: u64,
    pub rate: EventRate,
    /// The events of this tick, flat, `EVENT_INTS` for each event.
    pub events: Vec<i64>,
    pub alerts: u64,
    pub rejected: u64,
    pub applied: u64,
    /// FNV-1a over every applied or rejected request, in order, and the final credits.
    pub hash: u64,
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv(h: u64, v: i64) -> u64 {
    let mut h = h;
    for b in v.to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

impl World {
    pub fn new(units: usize, seed: u64, rate: EventRate) -> Self {
        let mut rng = Pcg32::new(seed, 54);
        let mut v = Vec::with_capacity(units);
        let mut counts = [[0i64; TYPES]; PLAYERS];
        for i in 0..units {
            let owner = (i % PLAYERS) as u8;
            let utype = rng.below(TYPES as u32) as u8;
            counts[owner as usize][utype as usize] += 1;
            let x = rng.below(MAP as u32) as i32;
            let y = rng.below(MAP as u32) as i32;
            v.push(Unit {
                x,
                y,
                tx: rng.below(MAP as u32) as i32,
                ty: rng.below(MAP as u32) as i32,
                owner,
                utype,
                stance: 0,
            });
        }
        let mut w = World {
            units: v,
            bucket_start: vec![0; (GRID * GRID + 1) as usize],
            order: vec![0; units],
            credits: [1_000; PLAYERS],
            counts,
            rng,
            tick: 0,
            rate,
            events: Vec::with_capacity(EVENT_INTS * 512),
            alerts: 0,
            rejected: 0,
            applied: 0,
            hash: FNV_OFFSET,
        };
        w.rebuild_grid();
        w
    }

    fn bucket_of(x: i32, y: i32) -> usize {
        let bx = (x / BUCKET).clamp(0, GRID - 1);
        let by = (y / BUCKET).clamp(0, GRID - 1);
        (by * GRID + bx) as usize
    }

    /// Counting sort of unit ids by bucket. Stable, so ids ascend inside a bucket.
    pub fn rebuild_grid(&mut self) {
        let n = self.bucket_start.len();
        for s in self.bucket_start.iter_mut() {
            *s = 0;
        }
        for u in &self.units {
            self.bucket_start[Self::bucket_of(u.x, u.y) + 1] += 1;
        }
        for i in 1..n {
            self.bucket_start[i] += self.bucket_start[i - 1];
        }
        let mut fill: Vec<u32> = self.bucket_start[..n - 1].to_vec();
        for (id, u) in self.units.iter().enumerate() {
            let b = Self::bucket_of(u.x, u.y);
            self.order[fill[b] as usize] = id as u32;
            fill[b] += 1;
        }
    }

    /// The Rust work of a tick: move, rebuild the grid, make this tick's events.
    pub fn sim_step(&mut self) {
        self.tick += 1;
        for i in 0..self.units.len() {
            let u = &mut self.units[i];
            let dx = u.tx - u.x;
            let dy = u.ty - u.y;
            if dx.abs() <= STEP && dy.abs() <= STEP {
                u.x = u.tx;
                u.y = u.ty;
                u.tx = self.rng.below(MAP as u32) as i32;
                u.ty = self.rng.below(MAP as u32) as i32;
            } else {
                u.x += dx.clamp(-STEP, STEP);
                u.y += dy.clamp(-STEP, STEP);
            }
        }
        self.rebuild_grid();
        self.make_events();
    }

    fn make_events(&mut self) {
        self.events.clear();
        let n = self.rate.min
            + if self.rate.spread > 0 {
                self.rng.below(self.rate.spread)
            } else {
                0
            };
        let units = self.units.len() as u32;
        for _ in 0..n {
            let r = self.rng.below(100);
            if r < 48 {
                let id = self.rng.below(units) as usize;
                let u = self.units[id];
                let killer = (u32::from(u.owner) + 1 + self.rng.below(PLAYERS as u32 - 1))
                    % PLAYERS as u32;
                self.events.extend_from_slice(&[
                    EV_DIED,
                    i64::from(u.owner),
                    i64::from(u.utype),
                    id as i64,
                    i64::from(killer),
                    i64::from(u.x),
                    i64::from(u.y),
                ]);
            } else if r < 96 {
                let id = self.rng.below(units) as usize;
                let u = self.units[id];
                self.events.extend_from_slice(&[
                    EV_PRODUCED,
                    i64::from(u.owner),
                    i64::from(u.utype),
                    id as i64,
                    0,
                    i64::from(u.x),
                    i64::from(u.y),
                ]);
            } else {
                let p = self.rng.below(PLAYERS as u32);
                let tech = self.rng.below(16);
                self.events.extend_from_slice(&[
                    EV_RESEARCHED,
                    i64::from(p),
                    0,
                    0,
                    i64::from(tech),
                    0,
                    0,
                ]);
            }
        }
        if self.tick % 20 == 0 {
            self.events
                .extend_from_slice(&[EV_TIMER, 0, 0, 0, (self.tick / 20) as i64, 0, 0]);
        }
    }

    /// Query: unit ids of `player` within `radius` of (x, y), at most `max`, in grid order.
    pub fn units_near(&self, player: i64, x: i64, y: i64, radius: i64, max: i64) -> Vec<i64> {
        let mut out = Vec::new();
        if !(0..PLAYERS as i64).contains(&player) || radius < 0 || max <= 0 {
            return out;
        }
        let radius = radius.min(i64::from(MAP));
        let max = max.min(256) as usize;
        let b = i64::from(BUCKET);
        let g = i64::from(GRID);
        let bx0 = ((x - radius) / b).clamp(0, g - 1);
        let bx1 = ((x + radius) / b).clamp(0, g - 1);
        let by0 = ((y - radius) / b).clamp(0, g - 1);
        let by1 = ((y + radius) / b).clamp(0, g - 1);
        let r2 = radius * radius;
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                let bi = (by * g + bx) as usize;
                let s = self.bucket_start[bi] as usize;
                let e = self.bucket_start[bi + 1] as usize;
                for &id in &self.order[s..e] {
                    let u = &self.units[id as usize];
                    if i64::from(u.owner) != player {
                        continue;
                    }
                    let dx = i64::from(u.x) - x;
                    let dy = i64::from(u.y) - y;
                    if dx * dx + dy * dy <= r2 {
                        out.push(i64::from(id));
                        if out.len() >= max {
                            return out;
                        }
                    }
                }
            }
        }
        out
    }

    pub fn count_owned(&self, player: i64, utype: i64) -> i64 {
        if (0..PLAYERS as i64).contains(&player) && (0..TYPES as i64).contains(&utype) {
            self.counts[player as usize][utype as usize]
        } else {
            0
        }
    }

    pub fn credits_of(&self, player: i64) -> i64 {
        if (0..PLAYERS as i64).contains(&player) {
            self.credits[player as usize]
        } else {
            0
        }
    }

    fn valid_unit(&self, u: i64) -> bool {
        u >= 0 && (u as usize) < self.units.len()
    }

    fn valid_pos(x: i64, y: i64) -> bool {
        (0..i64::from(MAP)).contains(&x) && (0..i64::from(MAP)).contains(&y)
    }

    fn valid_player(p: i64) -> bool {
        (0..PLAYERS as i64).contains(&p)
    }

    /// Validate and apply the requests of one script call, in order.
    pub fn apply(&mut self, requests: &mut Vec<Request>) {
        for r in requests.drain(..) {
            let mut h = self.hash;
            let ok = match &r {
                Request::Grant { player, amount } => {
                    h = fnv(fnv(fnv(h, 1), *player), *amount);
                    if Self::valid_player(*player) && (0..=10_000).contains(amount) {
                        self.credits[*player as usize] += *amount;
                        true
                    } else {
                        false
                    }
                }
                Request::MoveTo { unit, x, y } => {
                    h = fnv(fnv(fnv(fnv(h, 2), *unit), *x), *y);
                    if self.valid_unit(*unit) && Self::valid_pos(*x, *y) {
                        let u = &mut self.units[*unit as usize];
                        u.tx = *x as i32;
                        u.ty = *y as i32;
                        true
                    } else {
                        false
                    }
                }
                Request::MoveGroup { units, x, y } => {
                    h = fnv(fnv(fnv(fnv(h, 3), units.len() as i64), *x), *y);
                    for u in units {
                        h = fnv(h, *u);
                    }
                    if units.len() <= 256
                        && Self::valid_pos(*x, *y)
                        && units.iter().all(|u| self.valid_unit(*u))
                    {
                        for u in units {
                            let u = &mut self.units[*u as usize];
                            u.tx = *x as i32;
                            u.ty = *y as i32;
                        }
                        true
                    } else {
                        false
                    }
                }
                Request::Stance { unit, stance } => {
                    h = fnv(fnv(fnv(h, 4), *unit), *stance);
                    if self.valid_unit(*unit) && (0..3).contains(stance) {
                        self.units[*unit as usize].stance = *stance as u8;
                        true
                    } else {
                        false
                    }
                }
                Request::Alert { player, code } => {
                    h = fnv(fnv(fnv(h, 5), *player), *code);
                    if Self::valid_player(*player) {
                        self.alerts += 1;
                        true
                    } else {
                        false
                    }
                }
            };
            if ok {
                self.applied += 1;
            } else {
                self.rejected += 1;
                h = fnv(h, -1);
            }
            self.hash = h;
        }
    }

    /// The output hash: every request in order, then the credits.
    pub fn output_hash(&self) -> u64 {
        let mut h = self.hash;
        for c in self.credits {
            h = fnv(h, c);
        }
        fnv(h, self.applied as i64)
    }
}
