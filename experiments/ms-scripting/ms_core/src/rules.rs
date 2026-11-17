//! `rust_ref`: the mod rules of spec 3.2 in Rust, behind the same host boundary.
//! Its output hash is the reference that every candidate script must match.

use crate::cand::{Runtime, ScriptError};
use crate::host;
use crate::world::{EVENT_INTS, EV_DIED, EV_PRODUCED, EV_RESEARCHED, EV_TIMER, PLAYERS};

pub const BOUNTY: [i64; 4] = [50, 75, 150, 400];
pub const NEAR_RADIUS: i64 = 8_000;
pub const NEAR_MAX: i64 = 16;
pub const STANCE_CAP: i64 = 30;

pub struct RustRef {
    kills: [i64; PLAYERS],
    rally_x: [i64; PLAYERS],
    rally_y: [i64; PLAYERS],
    leader: i64,
    counter: i64,
}

impl Default for RustRef {
    fn default() -> Self {
        let mut rally_x = [0; PLAYERS];
        for (p, x) in rally_x.iter_mut().enumerate() {
            *x = 16_000 + p as i64 * 28_000;
        }
        RustRef {
            kills: [0; PLAYERS],
            rally_x,
            rally_y: [128_000; PLAYERS],
            leader: -1,
            counter: 0,
        }
    }
}

impl Runtime for RustRef {
    fn on_tick(&mut self, events: &[i64]) -> Result<(), ScriptError> {
        for e in events.chunks_exact(EVENT_INTS) {
            let (kind, player, utype, unit, extra, x, y) =
                (e[0], e[1], e[2], e[3], e[4], e[5], e[6]);
            if kind == EV_DIED {
                let killer = extra;
                self.kills[killer as usize] += 1;
                let mut bounty = BOUNTY[utype as usize];
                if self.kills[killer as usize] % 10 == 0 {
                    bounty *= 2;
                }
                host::grant(killer, bounty);
                let near = host::units_near(player, x, y, NEAR_RADIUS, NEAR_MAX);
                if near.len() < 3 {
                    host::alert(player, 1);
                } else {
                    let p = player as usize;
                    host::move_group(near, self.rally_x[p], self.rally_y[p]);
                }
            } else if kind == EV_PRODUCED {
                let c = host::count_owned(player, utype);
                if c > STANCE_CAP {
                    host::set_stance(unit, 2);
                } else {
                    let p = player as usize;
                    host::move_to(unit, self.rally_x[p], self.rally_y[p]);
                }
            } else if kind == EV_RESEARCHED {
                let p = player as usize;
                self.rally_x[p] += 1_000 * (extra % 3 + 1);
                self.rally_y[p] -= 500;
                if self.rally_x[p] > 250_000 {
                    self.rally_x[p] = 6_000;
                }
                if self.rally_y[p] < 6_000 {
                    self.rally_y[p] = 250_000;
                }
            } else if kind == EV_TIMER {
                let mut best = -1;
                let mut best_c = i64::MIN;
                for p in 0..PLAYERS as i64 {
                    let c = host::credits(p);
                    if c > best_c {
                        best = p;
                        best_c = c;
                    }
                }
                if best != self.leader {
                    self.leader = best;
                    host::alert(best, 2);
                }
            }
        }
        Ok(())
    }

    fn bench_calls(&mut self, n: i64) -> Result<i64, ScriptError> {
        let mut sum = 0;
        for i in 0..n {
            sum += host::count_owned(i % 8, i % 4);
        }
        Ok(sum)
    }

    fn on_unit(&mut self, _id: i64, x: i64, y: i64, _owner: i64) -> Result<(), ScriptError> {
        if x > y {
            self.counter += 1;
        }
        Ok(())
    }
}

/// `none`: the no-script baseline (W0). It does nothing in the script phase.
pub struct NoScript;

impl Runtime for NoScript {
    fn on_tick(&mut self, _events: &[i64]) -> Result<(), ScriptError> {
        Ok(())
    }
    fn bench_calls(&mut self, _n: i64) -> Result<i64, ScriptError> {
        Ok(0)
    }
    fn on_unit(&mut self, _: i64, _: i64, _: i64, _: i64) -> Result<(), ScriptError> {
        Ok(())
    }
}
