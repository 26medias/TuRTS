//! Plan Task 1: every candidate script gives the `rust_ref` output hash for W1 and W2, under
//! the calibrated limits. The mutated-script case shows the check can go red.

use ms_core::bench::Workload;
use ms_core::cand::{rules_source, Kind};
use ms_core::Bench;

fn hashes(kind: Kind, source: Option<&str>) -> (u64, u64, u64) {
    let mut b = match source {
        Some(s) => Bench::with_source(kind, s, None),
        None => Bench::new(kind, None),
    }
    .unwrap_or_else(|e| panic!("{kind:?}: {}", e.message));
    for w in [Workload::W1, Workload::W2] {
        b.select(w);
        for _ in 0..300 {
            b.sim();
            b.script(w);
            b.apply();
        }
    }
    let h = (b.hash(Workload::W1), b.hash(Workload::W2), b.errors);
    h
}

fn candidates() -> Vec<Kind> {
    ms_core::compiled_candidates()
        .iter()
        .filter_map(|s| Kind::parse(s))
        .collect()
}

#[test]
fn scripts_match_rust_ref() {
    let want = hashes(Kind::Rust, None);
    assert_eq!(want.2, 0);
    for k in candidates() {
        // Each kind runs on its own thread: the host state is thread-local.
        let got = std::thread::spawn(move || hashes(k, None)).join().unwrap();
        assert_eq!(got, want, "{k:?}");
    }
}

#[test]
fn changed_rule_changes_hash() {
    let want = hashes(Kind::Rust, None);
    for k in candidates() {
        let src = rules_source(k).replacen("400", "401", 1);
        assert_ne!(src, rules_source(k));
        let got = std::thread::spawn(move || hashes(k, Some(&src))).join().unwrap();
        assert_ne!((got.0, got.1), (want.0, want.1), "{k:?}: the test cannot go red");
    }
}
