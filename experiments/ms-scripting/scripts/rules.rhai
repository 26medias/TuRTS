// MS mod rules (spec 3.2), Rhai. The host binds `this` to the state that `init` returns.
// Event record: [kind, player, unit_type, unit, extra, x, y]. 1 Died, 2 Produced,
// 3 Researched, 4 Timer.

fn init() {
    let rx = [];
    let ry = [];
    for p in 0..8 {
        rx.push(16000 + p * 28000);
        ry.push(128000);
    }
    #{ kills: [0, 0, 0, 0, 0, 0, 0, 0], rx: rx, ry: ry, leader: -1, counter: 0 }
}

fn on_tick(ev) {
    let bounty = [50, 75, 150, 400];
    let n = ev.len();
    let i = 0;
    while i < n {
        let kind = ev[i];
        let player = ev[i + 1];
        if kind == 1 {
            let killer = ev[i + 4];
            this.kills[killer] += 1;
            let b = bounty[ev[i + 2]];
            if this.kills[killer] % 10 == 0 {
                b *= 2;
            }
            grant(killer, b);
            let near = units_near(player, ev[i + 5], ev[i + 6], 8000, 16);
            if near.len() < 3 {
                alert(player, 1);
            } else {
                move_group(near, this.rx[player], this.ry[player]);
            }
        } else if kind == 2 {
            let c = count_owned(player, ev[i + 2]);
            if c > 30 {
                set_stance(ev[i + 3], 2);
            } else {
                move_to(ev[i + 3], this.rx[player], this.ry[player]);
            }
        } else if kind == 3 {
            this.rx[player] += 1000 * (ev[i + 4] % 3 + 1);
            this.ry[player] -= 500;
            if this.rx[player] > 250000 {
                this.rx[player] = 6000;
            }
            if this.ry[player] < 6000 {
                this.ry[player] = 250000;
            }
        } else if kind == 4 {
            let best = -1;
            let best_c = -1;
            for p in 0..8 {
                let c = credits(p);
                if c > best_c {
                    best = p;
                    best_c = c;
                }
            }
            if best != this.leader {
                this.leader = best;
                alert(best, 2);
            }
        }
        i += 7;
    }
}

fn bench_calls(n) {
    let s = 0;
    let i = 0;
    while i < n {
        s += count_owned(i % 8, i % 4);
        i += 1;
    }
    s
}

fn on_unit(id, x, y, owner) {
    if x > y {
        this.counter += 1;
    }
}
