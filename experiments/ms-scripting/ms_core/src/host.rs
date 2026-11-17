//! The host boundary. Every candidate binds exactly these eight functions (spec 3.1).
//! Queries read the world in Rust. Requests only append to a buffer; the host applies them
//! after the script call (`World::apply`). A script never holds the world.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::world::{Request, World};

/// Test mutants for the red checks of the plan. 0: none (the measured build).
pub const MUTANT_NONE: u8 = 0;
/// Apply each request inside the host call (L8 must go red).
pub const MUTANT_APPLY_NOW: u8 = 1;
/// No request cap (L9a must go red).
pub const MUTANT_NO_REQUEST_CAP: u8 = 2;
/// No per-instance memory accounting (L3 (d) must go red on Rune).
pub const MUTANT_NO_ACCOUNTING: u8 = 3;

static MUTANT: AtomicU8 = AtomicU8::new(MUTANT_NONE);

pub fn set_mutant(m: u8) {
    MUTANT.store(m, Ordering::Relaxed)
}

pub fn mutant() -> u8 {
    MUTANT.load(Ordering::Relaxed)
}

pub fn accounting() -> bool {
    mutant() != MUTANT_NO_ACCOUNTING
}

/// At most this many requests in one script call (spec 3.1).
pub const REQUEST_CAP: usize = 4_096;

pub struct HostState {
    pub world: Option<World>,
    pub requests: Vec<Request>,
    pub calls: u64,
    /// Requests of the current call, counted with the dropped ones.
    pub call_requests: usize,
}

thread_local! {
    static HOST: RefCell<HostState> = const {
        RefCell::new(HostState { world: None, requests: Vec::new(), calls: 0, call_requests: 0 })
    };
}

pub fn with<R>(f: impl FnOnce(&mut HostState) -> R) -> R {
    HOST.with(|h| f(&mut h.borrow_mut()))
}

pub fn install_world(w: World) -> Option<World> {
    with(|h| h.world.replace(w))
}

pub fn take_world() -> Option<World> {
    with(|h| h.world.take())
}

fn world<R>(f: impl FnOnce(&World) -> R) -> R {
    with(|h| {
        h.calls += 1;
        f(h.world.as_ref().expect("no world installed"))
    })
}

fn push(r: Request) {
    with(|h| {
        h.calls += 1;
        h.call_requests += 1;
        let m = mutant();
        if m == MUTANT_APPLY_NOW {
            let mut one = vec![r];
            h.world.as_mut().expect("world").apply(&mut one);
        } else if h.call_requests <= REQUEST_CAP || m == MUTANT_NO_REQUEST_CAP {
            h.requests.push(r)
        }
    })
}

pub fn begin_call() {
    with(|h| h.call_requests = 0)
}

pub fn over_request_cap() -> bool {
    with(|h| h.call_requests > REQUEST_CAP) && mutant() != MUTANT_NO_REQUEST_CAP
}

// --- Queries -------------------------------------------------------------------------------

pub fn count_owned(player: i64, utype: i64) -> i64 {
    world(|w| w.count_owned(player, utype))
}

pub fn credits(player: i64) -> i64 {
    world(|w| w.credits_of(player))
}

pub fn units_near(player: i64, x: i64, y: i64, radius: i64, max: i64) -> Vec<i64> {
    world(|w| w.units_near(player, x, y, radius, max))
}

// --- Requests ------------------------------------------------------------------------------

pub fn grant(player: i64, amount: i64) {
    push(Request::Grant { player, amount })
}

pub fn move_to(unit: i64, x: i64, y: i64) {
    push(Request::MoveTo { unit, x, y })
}

pub fn move_group(units: Vec<i64>, x: i64, y: i64) {
    push(Request::MoveGroup { units, x, y })
}

pub fn set_stance(unit: i64, stance: i64) {
    push(Request::Stance { unit, stance })
}

pub fn alert(player: i64, code: i64) {
    push(Request::Alert { player, code })
}

/// Heap bytes that the host request buffer owns (the buffer and the `move_group` id lists).
/// The counting allocator sees them as live; they are host memory, not script memory, and the
/// host frees them in `apply`, outside the call (spec 3.5 accounting).
pub fn request_heap_bytes() -> usize {
    with(|h| {
        let mut n = h.requests.capacity() * std::mem::size_of::<Request>();
        for r in &h.requests {
            if let Request::MoveGroup { units, .. } = r {
                n += units.capacity() * std::mem::size_of::<i64>();
            }
        }
        n
    })
}

/// Requests count, used to drop the partial requests of a failed call.
pub fn request_len() -> usize {
    with(|h| h.requests.len())
}

pub fn truncate_requests(n: usize) {
    with(|h| h.requests.truncate(n))
}
