//! Test-only local profiling. The production binary contains no profiler or clock calls.
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    time::Instant,
};
const MAX_SAMPLES: usize = 100_000;
#[derive(Default)]
struct Stage {
    samples: Vec<u64>,
    discarded: u64,
}
thread_local! {
    // A disabled probe touches only this constant-initialized, destructor-free flag.
    static ACTIVE:Cell<bool>=const{Cell::new(false)};
    static STAGES:RefCell<BTreeMap<&'static str,Stage>>=const{RefCell::new(BTreeMap::new())};
}
/// The benchmark enables capture only inside its exact steady-state sample intervals.
pub(crate) fn activate(active: bool) {
    ACTIVE.set(active)
}
pub(crate) fn reset() {
    activate(false);
    STAGES.with_borrow_mut(BTreeMap::clear)
}
pub(crate) fn start() -> Option<Instant> {
    if ACTIVE.get() {
        Some(Instant::now())
    } else {
        None
    }
}
pub(crate) fn finish(stage: &'static str, start: Option<Instant>) {
    let Some(start) = start else { return };
    let elapsed = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
    STAGES.with_borrow_mut(|stages| {
        let stage = stages.entry(stage).or_default();
        if stage.samples.len() < MAX_SAMPLES {
            stage.samples.push(elapsed)
        } else {
            stage.discarded += 1;
        }
    });
}
/// Nested stages remain separate. Their sums must not be added as disjoint phase costs.
pub(crate) fn summary() -> serde_json::Value {
    STAGES.with_borrow(|stages|{
        let rows=stages.iter().map(|(name,stage)|{
            let mut values=stage.samples.clone();values.sort_unstable();
            let total=values.iter().map(|value|u128::from(*value)).sum::<u128>();
            let percentile=|percent:usize|values[(values.len()*percent).div_ceil(100).saturating_sub(1)];
            ((*name).to_owned(),serde_json::json!({"count":values.len(),"total_ns":u64::try_from(total).unwrap_or(u64::MAX),"mean_ns":u64::try_from(total/values.len() as u128).unwrap_or(u64::MAX),"p50_ns":percentile(50),"p95_ns":percentile(95),"p99_ns":percentile(99),"max_ns":values.last().unwrap(),"discarded_samples":stage.discarded}))
        }).collect::<serde_json::Map<_,_>>();
        serde_json::Value::Object(rows)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_probes_collect_nothing_and_nested_stages_stay_distinct() {
        reset();
        assert!(start().is_none());
        finish("disabled", None);
        assert_eq!(summary(), serde_json::json!({}));
        activate(true);
        let outer = start();
        let inner = start();
        finish("inner", inner);
        finish("outer", outer);
        activate(false);
        let result = summary();
        assert_eq!(result["inner"]["count"], 1);
        assert_eq!(result["outer"]["count"], 1);
        assert!(
            result["outer"]["total_ns"].as_u64().unwrap()
                >= result["inner"]["total_ns"].as_u64().unwrap()
        );
        reset();
    }
}
