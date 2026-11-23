use super::{
    diagnostics,
    fixture::{Fixture, VARIANTS},
};
use serde_json::json;
use std::collections::BTreeMap;
use turts_net_protocol::{OrderTarget, Pos, QueueMode};

const PACKAGES: [&str; 2] = ["quarry", "terrain_stockyard"];
const MOVE_BOUND: u32 = 2400;
fn point(x: f32, y: f32) -> OrderTarget {
    OrderTarget::Point(Pos { x, y })
}
fn require_crossed(f: &Fixture, labels: &[String], right: bool) {
    f.require_complete(labels);
    for label in labels {
        let actor = f.actor(label);
        let x = f.position(label).x;
        assert!(
            if right {
                x - actor.profile.radius >= 34.0
            } else {
                x + actor.profile.radius <= 30.0
            },
            "{} tick{} {label} completed on the wrong plateau at{x}",
            f.scene_name,
            f.tick
        );
    }
}
fn step(f: &mut Fixture) {
    f.step();
    let start = f.timer_start();
    diagnostics::bounded(f);
    f.record_diagnostics_time(start);
}
fn until(f: &mut Fixture, tick: u32) {
    while f.tick < tick {
        step(f);
    }
}

#[test]
fn gm1_wide_both_public_packages_complete_and_reversals_preserve_every_physical_tick() {
    let mut compared_dirs = Vec::with_capacity(PACKAGES.len() * VARIANTS.len());
    for package in PACKAGES {
        // Functional worlds are independent; keep packages serial and spawn only
        // the four frozen variants. Construct/drop each App on its own worker.
        let traces = std::thread::scope(|scope| {
            let workers = VARIANTS.map(|variant| {
                scope.spawn(move || {
                    let mut f = Fixture::new(package, "GM1-wide", variant);
                    assert_eq!(f.actors.len(), 36);
                    let labels = f.labels(|_| true);
                    until(&mut f, MOVE_BOUND);
                    require_crossed(&f, &labels, true);
                    f.write_result(json!({"row":"GM1","variant":format!("{variant:?}"),"complete":36,"swept_checks":"every committed tick","reversal":"pending parent comparison"}));
                    (variant, std::mem::take(&mut f.trace), f.evidence_dir().to_owned())
                })
            });
            workers.map(|worker| match worker.join() {
                Ok(trace) => trace,
                Err(panic) => std::panic::resume_unwind(panic),
            })
        });
        // Joining in VARIANTS order preserves the canonical reference, regardless
        // of worker completion order. Compare every exact normalized tick.
        let reference = &traces[0].1;
        for (variant, trace, _) in &traces[1..] {
            assert_eq!(
                reference, trace,
                "{package} physical trajectory differs under{variant:?}"
            );
        }
        compared_dirs.extend(traces.into_iter().map(|(_, _, dir)| dir));
    }
    // Publish reversal success only after every package's comparisons passed.
    for dir in compared_dirs {
        let path = dir.join("native-result.json");
        let mut result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        result["checks"]["reversal"] = json!("exact physical rows");
        std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    }
}
#[test]
fn gm2_narrow_keeps_large_original_order_then_ordinary_excavation_resumes_it() {
    for package in PACKAGES {
        // Four independent Apps; packages remain serial. Join in frozen variant order.
        std::thread::scope(|scope| {
            let workers = VARIANTS.map(|variant| {
                scope.spawn(move || {
                    let mut f = Fixture::new(package, "GM2-narrow", variant);
                    let small = f.labels(|a| a.kind == "small");
                    let large = f.labels(|a| a.kind == "large");
                    step(&mut f);
                    let original: BTreeMap<_, _> = large
                        .iter()
                        .map(|l| (l.clone(), f.order(l).expect("accepted original large move")))
                        .collect();
                    until(&mut f, MOVE_BOUND);
                    require_crossed(&f, &small, true);
                    for label in &large {
                        assert_eq!(
                            f.order(label).as_ref(),
                            Some(&original[label]),
                            "blocked move was replaced"
                        );
                        assert!(
                            f.position(label).x + f.actor(label).profile.radius <= 30.0,
                            "large footprint entered narrow opening"
                        );
                        assert!(
                            !f.complete(label),
                            "narrow refusal falsely completed original move"
                        );
                        let row = diagnostics::member(&f, label);
                        assert!(
                            matches!(
                                row.state,
                                turts_sim::terrain::NavigationMemberState::Blocked
                                    | turts_sim::terrain::NavigationMemberState::Pending
                                    | turts_sim::terrain::NavigationMemberState::Searching
                            ),
                            "large actor must retain truthful waiting route: {row:?}"
                        );
                    }
                    // Tick2401 is the frozen ordinary work envelope. No large move is sent again.
                    // The guard bounds fixture completion; the movement deadline starts at actual opening.
                    let mut opened = None;
                    for _ in 0..=MOVE_BOUND {
                        step(&mut f);
                        for label in &large {
                            if !f.complete(label) {
                                assert_eq!(f.order(label).as_ref(), Some(&original[label]));
                            }
                        }
                        if f.surface.rectangle_is([30, 20], [34, 28], 0) {
                            opened = Some(f.tick);
                            break;
                        }
                    }
                    let opened = opened
                        .expect("ordinary editor did not expose full eight-cell-wide passage");
                    until(&mut f, opened + MOVE_BOUND);
                    require_crossed(&f, &large, true);
                    f.write_result(
                json!({"row":"GM2","opening_tick":opened,"large_completed_without_reissue":12}),
            );
                })
            });
            workers.map(|worker| match worker.join() {
                Ok(()) => (),
                Err(panic) => std::panic::resume_unwind(panic),
            })
        });
    }
}
#[test]
fn gm3_feasible_opposing_shared_unique_and_movable_obstruction_complete() {
    let mut compared_dirs = Vec::with_capacity(PACKAGES.len() * 3 * VARIANTS.len());
    for package in PACKAGES {
        for scene in [
            "GM3-opposing-large",
            "GM3-movable-blocker",
            "GM3-shared-and-unique",
        ] {
            // Only the four independent variants run concurrently. Each worker
            // owns its App; packages and scenes retain their original order.
            let traces = std::thread::scope(|scope| {
                let workers = VARIANTS.map(|variant| {
                    scope.spawn(move || {
                        let mut f = Fixture::new(package, scene, variant);
                        let deadline = f.scene["deadline_ticks"].as_u64().unwrap() as u32;
                        until(&mut f, deadline);
                        f.require_complete(&f.labels(|_| true));
                        for actor in &f.actors {
                            if actor.label == "blocker" || actor.label == "rear.small.0" {
                                continue;
                            }
                            if scene == "GM3-shared-and-unique" || actor.slot == 0 {
                                require_crossed(&f, std::slice::from_ref(&actor.label), true);
                            } else {
                                require_crossed(&f, std::slice::from_ref(&actor.label), false);
                            }
                        }
                        if scene == "GM3-shared-and-unique" {
                            assert!(
                                (f.position("rear.small.0") - turts_sim::Vec2::new(20.0, 6.0)).length()
                                    <= f.actor("rear.small.0").profile.radius,
                                "unique replacement did not reach its actual endpoint"
                            );
                        }
                        f.write_result(json!({"row":"GM3","feasible_complete":true,"variant":format!("{variant:?}"),"reversal":"pending parent comparison"}));
                        (variant, std::mem::take(&mut f.trace), f.evidence_dir().to_owned())
                    })
                });
                workers.map(|worker| match worker.join() {
                    Ok(trace) => trace,
                    Err(panic) => std::panic::resume_unwind(panic),
                })
            });
            // VARIANTS order fixes the canonical trace despite completion order.
            let reference = &traces[0].1;
            for (variant, trace, _) in &traces[1..] {
                assert_eq!(
                    reference, trace,
                    "{package}/{scene} physical reversal{variant:?}"
                );
            }
            compared_dirs.extend(traces.into_iter().map(|(_, _, dir)| dir));
        }
    }
    // No receipt claims reversal success until all packages and scenes compare.
    for dir in compared_dirs {
        let path = dir.join("native-result.json");
        let mut result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        result["checks"]["reversal"] = json!("exact physical rows");
        std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    }
}
#[test]
fn gm3_temporary_blocker_far_from_goal_does_not_complete_the_move() {
    // One 1.4-radius mover blocks the 3-cell corridor for eight seconds.
    // Clearing it is an ordinary player order; the trailing actor receives no replacement.
    for variant in [VARIANTS[0], VARIANTS[1]] {
        let mut f = Fixture::new("quarry", "GM3-temporary-obstruction", variant);
        while f.tick < 160 {
            step(&mut f);
            if f.tick > 10 {
                assert!(
                    !f.complete("mover"),
                    "temporary obstruction falsely completed a distant move at tick {} and {:?}",
                    f.tick,
                    f.position("mover")
                );
            }
        }
        let before_clear = f.position("mover");
        until(&mut f, 1200);
        assert!(
            f.complete("mover"),
            "mover did not resume after the blocker moved"
        );
        let endpoint = turts_sim::Vec2::new(48.0, 23.5);
        assert!(
            (f.position("mover") - endpoint).length() <= 0.05,
            "mover stopped away from its original endpoint: {:?}",
            f.position("mover")
        );
        assert!(f.position("mover").x > before_clear.x + 4.0);
    }
}

#[test]
fn gm3_settled_blocker_never_moves_and_impossible_traffic_waits_stably() {
    for package in PACKAGES {
        // Each variant has its own fixture, server app, and evidence directory.
        std::thread::scope(|scope| {
            let workers = VARIANTS.map(|variant| {
                scope.spawn(move || {
                    let mut f = Fixture::new(package, "GM3-settled-blocker", variant);
                    let blocker = f.position("blocker").to_array().map(f32::to_bits);
                    let deadline = f.scene["deadline_ticks"].as_u64().unwrap() as u32;
                    let mut stable = None;
                    while f.tick < deadline {
                        step(&mut f);
                        assert_eq!(
                            f.position("blocker").to_array().map(f32::to_bits),
                            blocker,
                            "later traffic displaced settled blocker"
                        );
                        if f.tick == deadline - 200 {
                            stable = Some(f.frame());
                        }
                        if f.tick > deadline - 200 {
                            assert_eq!(
                                Some(f.frame()).as_ref(),
                                stable.as_ref(),
                                "impossible traffic oscillates instead of waiting stably"
                            );
                        }
                    }
                    assert!(f.complete("blocker"));
                    for actor in f.actors.iter().filter(|a| a.label != "blocker") {
                        assert!(
                            !f.complete(&actor.label),
                            "impossible passage falsely completed a move"
                        );
                        if actor.slot == 0 {
                            assert!(f.position(&actor.label).x + actor.profile.radius <= 30.0);
                        } else {
                            assert!(f.position(&actor.label).x - actor.profile.radius >= 34.0);
                        }
                    }
                    f.write_result(
                        json!({"row":"GM3","settled_point_retained":true,"stable_ticks":200}),
                    );
                })
            });
            workers.map(|worker| match worker.join() {
                Ok(()) => (),
                Err(panic) => std::panic::resume_unwind(panic),
            })
        });
    }
}
#[test]
fn gm4_real_fill_excavation_invalidate_only_relevant_routes() {
    for package in PACKAGES {
        // Four independent Apps; packages remain serial. Join in frozen variant order.
        std::thread::scope(|scope| {
            let workers = VARIANTS.map(|variant| scope.spawn(move || {
            let mut f = Fixture::new(package, "GM4-edit-during-approach", variant);
            until(&mut f, 19);
            let distant_label = "large.2.3";
            let distant = diagnostics::member(&f, distant_label);
            assert!(!f.complete(distant_label));
            let original = f.order(distant_label).unwrap();
            let mut closed = false;
            let mut reopened = false;
            let mut moving_distant = false;
            let start = f.position(distant_label);
            let deadline = f.scene["deadline_ticks"].as_u64().unwrap() as u32;
            while f.tick < deadline {
                step(&mut f);
                closed |= f.surface.rectangle_is([31, 20], [32, 28], 4);
                if closed && f.tick > 240 {
                    reopened |= f.surface.rectangle_is([31, 20], [32, 28], 0);
                }
                if f.tick <= 340 {
                    diagnostics::same_lease(&distant, &diagnostics::member(&f, distant_label));
                    assert_eq!(f.order(distant_label), Some(original.clone()));
                    moving_distant |= f.position(distant_label) != start;
                }
            }
            assert!(closed, "frozen ordinary fill never closed full passage");
            assert!(reopened, "ordinary excavation never reopened passage");
            assert!(
                moving_distant,
                "distant unchanged route made no physical progress"
            );
            let main = f.labels(|a| a.label != "small.0.0" && a.label != distant_label);
            require_crossed(&f, &main, true);
            f.write_result(json!({"row":"GM4","closed":closed,"reopened":reopened,"distant_lease_unchanged_through":340}));
            }));
            workers.map(|worker| match worker.join() {
                Ok(()) => (),
                Err(panic) => std::panic::resume_unwind(panic),
            })
        });
    }
}
#[test]
fn exact_geometry_legal_cases_complete_and_refused_boundaries_do_not_tunnel() {
    for package in PACKAGES {
        for (scene, legal) in [
            ("GM-legal-step", true),
            ("GM-step-refusal", false),
            ("GM-grade-refusal", false),
            ("GM-exact-tangency", true),
            ("GM-next-radius-refusal", false),
            ("GM-diagonal-corner", true),
            ("GM-fast-wall", false),
        ] {
            for variant in [VARIANTS[0], VARIANTS[3]] {
                let mut f = Fixture::new(package, scene, variant);
                until(&mut f, MOVE_BOUND);
                if legal {
                    f.require_complete(&["probe".into()]);
                    let target = f.scene["commands"][0]["point"].as_array().unwrap();
                    let endpoint = turts_sim::Vec2::new(
                        target[0].as_f64().unwrap() as f32,
                        target[1].as_f64().unwrap() as f32,
                    );
                    assert!(
                        (f.position("probe") - endpoint).length()
                            <= f.actor("probe").profile.radius,
                        "legal case never reached endpoint"
                    );
                } else {
                    assert!(
                        !f.complete("probe"),
                        "illegal boundary falsely completed order"
                    );
                    assert!(
                        f.position("probe").x <= 30.0,
                        "illegal footprint crossed authored wall/ledge"
                    );
                }
                f.write_result(json!({"row":"geometry","legal":legal,"continuous_full_body":true}));
            }
        }
    }
}
#[test]
fn gm5_ordinary_append_replace_stop_release_memberships_and_replay_same_physical_ticks() {
    for package in PACKAGES {
        let traces = std::thread::scope(|scope| {
            let workers = VARIANTS.map(|variant| scope.spawn(move || {
            let mut f = Fixture::new(package, "GM1-wide", variant);
            until(&mut f, 40);
            let one = vec!["small.0.0".to_string()];
            let original = f
                .order(&one[0])
                .expect("active original movement before Append");
            let append = f.envelope("order_move", &one, point(12.0, 6.0), QueueMode::Append, 77);
            f.send(0, append);
            step(&mut f);
            assert_eq!(f.order(&one[0]), Some(original));
            let entity = turts_sim::unit_entity(f.app.world(), f.actor(&one[0]).unit).unwrap();
            assert_eq!(
                f.app
                    .world()
                    .get::<turts_sim::UnitOrders>(entity)
                    .unwrap()
                    .queue
                    .len(),
                1
            );
            let replace = f.envelope("order_move", &one, point(16.0, 6.0), QueueMode::Replace, 78);
            f.send(0, replace);
            step(&mut f);
            assert!(
                matches!(f.order(&one[0]).unwrap().target,turts_sim::OrderTargetSim::Point(p) if p==turts_sim::Vec2::new(16.0,6.0))
            );
            for cycle in 0..32 {
                let labels = f.labels(|_| true);
                let stop = f.envelope(
                    "order_stop",
                    &labels,
                    OrderTarget::None,
                    QueueMode::Replace,
                    0,
                );
                f.send(0, stop);
                step(&mut f);
                // Bounded deferred reclamation is permitted; physical cancellation is immediate.
                let stopped = f.frame();
                for _ in 0..16 {
                    step(&mut f);
                    assert_eq!(stopped, f.frame(), "Stop retained physical movement");
                }
                diagnostics::no_members(&f);
                if cycle < 31 {
                    let move_order = f.envelope(
                        "order_move",
                        &labels,
                        point(50.0, 24.0),
                        QueueMode::Replace,
                        100 + cycle,
                    );
                    f.send(0, move_order);
                    for _ in 0..4 {
                        step(&mut f);
                    }
                }
            }
            // Only the canonical variant authors the shared replay capture.
            let capture = (!variant.allocation && !variant.members)
                .then(|| f.write_movement_capture());
            f.write_result(json!({"row":"GM5","append_replace_stop":true,"replacement_cycles":32,"same_seed_trajectory":true,
                "separate_checks":["native health and owner lifecycle","GM6 hidden paired worlds"],"movement_capture_replay_and_forgery_rejection":variant.allocation||variant.members}));
            let trace = std::mem::take(&mut f.trace);
            drop(f);
            let fresh = Fixture::new(package, "GM1-wide", variant);
            diagnostics::no_members(&fresh);
            assert_eq!(fresh.tick, 0, "fresh match inherited old ticks");
            (variant, trace, capture)
            }));
            workers.map(|worker| match worker.join() {
                Ok(trace) => trace,
                Err(panic) => std::panic::resume_unwind(panic),
            })
        });
        // The canonical capture and all replay/forgery checks retain their
        // original order, independent of worker completion order.
        let reference = &traces[0].1;
        let captured = std::fs::read(traces[0].2.as_ref().unwrap()).unwrap();
        for (variant, trace, _) in &traces[1..] {
            assert_eq!(
                reference, trace,
                "same-seed ordinary command replay differs under{variant:?}"
            );
            super::replay_audit::certify(&captured, trace).unwrap();
            super::replay_audit::reject_forged_positions(&captured, trace);
        }
    }
}

#[test]
#[ignore = "Short functional cost attribution; not milestone acceptance"]
fn diagnostic_gm1_64_ticks_attribute_engine_and_exact_oracle_cost() {
    let mut f = Fixture::new("quarry", "GM1-wide", Default::default());
    until(&mut f, 64);
    f.write_result(json!({"scope":"64 real ticks with unchanged physical and diagnostic checks","acceptance":false}));
}

#[test]
#[ignore = "Short opposing-traffic regression; not milestone acceptance"]
fn diagnostic_gm3_64_ticks_preserve_authored_speed_and_swept_safety() {
    let mut f = Fixture::new("quarry", "GM3-opposing-large", Default::default());
    until(&mut f, 64);
    f.write_result(json!({"scope":"64 real opposing-traffic ticks with unchanged physical and diagnostic checks","acceptance":false}));
}

#[test]
#[ignore = "Canonical GM1 diagnosis before the full reversal campaign"]
fn diagnostic_gm1_canonical_ordinary_move_completes() {
    let mut f = Fixture::new("quarry", "GM1-wide", Default::default());
    let labels = f.labels(|_| true);
    until(&mut f, MOVE_BOUND);
    require_crossed(&f, &labels, true);
    f.write_result(json!({"scope":"canonical Quarry GM1 ordinary Move, unchanged 2400-tick bound","complete":36,"acceptance":false}));
}

#[test]
#[ignore = "Short real-dispatch destination-authority regression; not GM1 acceptance"]
fn diagnostic_real_dispatch_retains_destination_region_through_plain_recovery() {
    let mut f = Fixture::new("quarry", "GM1-wide", Default::default());
    until(&mut f, 3);
    let label = "large.0.3".to_string();
    let actor = f.actor(&label).unit;
    let original = f.order(&label).unwrap();
    let before = diagnostics::member(&f, &label);
    let region = before
        .destination_region
        .expect("actual initial ordinary dispatch captures region");
    assert_eq!(region.goal, [50., 24.]);
    assert_eq!(region.cohort_radius, 1.1);
    assert_eq!(region.epoch, before.order_epoch);
    assert!(region.inline_bytes > 0);
    let pools = &f
        .app
        .world()
        .resource::<turts_sim::movement::MovementPools>()
        .0;
    let pool = pools
        .values()
        .find(|p| p.point == turts_sim::Vec2::new(50., 24.) && p.primary_count == 36)
        .unwrap();
    let extent = turts_sim::Vec2::new(
        pool.spacing * 3.,
        pool.spacing * (3.0_f32.sqrt() * 0.5 * 3.),
    );
    let low = pool.point - extent;
    let high = pool.point + extent;
    assert_eq!(region.primary_min, [low.x.next_down(), low.y.next_down()]);
    assert_eq!(region.primary_max, [high.x.next_up(), high.y.next_up()]);
    // This lifecycle probe converts only local controller state after a REAL
    // public dispatch. It invents no old pool, pose, order or navigation scope.
    let entity = turts_sim::unit_entity(f.app.world(), actor).unwrap();
    let target = f
        .app
        .world()
        .get::<turts_sim::world::MoveTarget>(entity)
        .unwrap()
        .0;
    f.app
        .world_mut()
        .entity_mut(entity)
        .remove::<turts_sim::movement::MoveProgress>();
    f.app.world_mut().entity_mut(entity).insert((
        turts_sim::movement::PlainMoveProgress {
            arrival_yield: false,
            original_point: target,
            slot: target,
            pool_slots: vec![],
            gather_goal: None,
            activity: turts_sim::movement::MovementActivity::Blocked,
            stalled_ticks: 1013,
            best_distance: 10.,
            claim_count: 0,
            failure_recorded: false,
        },
        turts_sim::movement::PhysicalEscape { target: None },
    ));
    step(&mut f);
    let recovered = diagnostics::member(&f, &label);
    assert_eq!(recovered.destination_region, Some(region));
    assert_eq!(recovered.order_epoch, before.order_epoch);
    assert_eq!(f.order(&label), Some(original.clone()));
    // Queued intent does not replace the active dispatch authority.
    let one = vec![label.clone()];
    let append = f.envelope("order_move", &one, point(12., 6.), QueueMode::Append, 77);
    f.send(0, append);
    step(&mut f);
    assert_eq!(
        diagnostics::member(&f, &label).destination_region,
        Some(region)
    );
    assert_eq!(f.order(&label), Some(original));
    // Same-coordinate Replace gets a NEW actual singleton dispatch and epoch.
    let replace = f.envelope("order_move", &one, point(50., 24.), QueueMode::Replace, 78);
    f.send(0, replace);
    step(&mut f);
    let replaced = diagnostics::member(&f, &label);
    let new_region = replaced.destination_region.unwrap();
    assert_ne!(replaced.order_epoch, before.order_epoch);
    assert_eq!(new_region.epoch, replaced.order_epoch);
    assert_ne!(new_region.primary_min, region.primary_min);
    let stop = f.envelope(
        "order_stop",
        &one,
        OrderTarget::None,
        QueueMode::Replace,
        79,
    );
    f.send(0, stop);
    step(&mut f);
    assert!(
        turts_sim::terrain::privileged_navigation_audit(f.app.world())
            .members
            .iter()
            .all(|m| m.unit != actor || m.destination_region.is_none())
    );
    f.write_result(json!({"scope":"real initial dispatch, plain recovery, pending Append, same-XY Replace and Stop","acceptance":false,"scope_inline_bytes_per_member":region.inline_bytes,"initial_cohort_members":36,"initial_primary_min":region.primary_min,"initial_primary_max":region.primary_max,"cohort_radius":region.cohort_radius,"scope_epoch_matches_order":true}));
}

#[test]
#[ignore = "One unchanged public opposing-large world; focused diagnosis, not milestone acceptance"]
fn gm3_opposing_large_single_public_world_keeps_all_physical_assertions() {
    let mut f = Fixture::new("quarry", "GM3-opposing-large", Default::default());
    let deadline = f.scene["deadline_ticks"].as_u64().unwrap() as u32;
    until(&mut f, deadline);
    f.require_complete(&f.labels(|_| true));
    f.write_result(json!({"scope":"one unchanged ordinary opposing-large world","acceptance":false,"all_actor_completion":true,"physical_and_budget_assertions":"every tick"}));
}

#[test]
#[ignore = "One unchanged public movable-blocker world; focused diagnosis, not milestone acceptance"]
fn gm3_movable_blocker_single_public_world_keeps_all_physical_assertions() {
    let mut f = Fixture::new("quarry", "GM3-movable-blocker", Default::default());
    let deadline = f.scene["deadline_ticks"].as_u64().unwrap() as u32;
    until(&mut f, deadline);
    f.require_complete(&f.labels(|_| true));
    f.write_result(json!({"scope":"one unchanged ordinary movable-blocker world","acceptance":false,"all_actor_completion":true,"physical_and_budget_assertions":"every tick"}));
}

#[test]
#[ignore = "One unchanged public mixed-radius corridor; focused regression, not full acceptance"]
fn gm3_shared_unique_single_public_world_keeps_all_physical_assertions() {
    let mut f = Fixture::new("quarry", "GM3-shared-and-unique", Default::default());
    let deadline = f.scene["deadline_ticks"].as_u64().unwrap() as u32;
    until(&mut f, deadline);
    f.require_complete(&f.labels(|_| true));
    require_crossed(&f, &f.labels(|a| a.label != "rear.small.0"), true);
    assert!(
        (f.position("rear.small.0") - turts_sim::Vec2::new(20., 6.)).length()
            <= f.actor("rear.small.0").profile.radius
    );
    f.write_result(json!({"scope":"one unchanged mixed-radius shared-and-unique corridor", "acceptance":false,"all_actor_completion":true,"physical_and_budget_assertions":"every tick"}));
}

#[test]
fn gm3_browser_delayed_rear_group_at_a_held_point_completes() {
    let mut f = Fixture::new("quarry", "GM3-shared-and-unique", VARIANTS[0]);
    // Actual application ticks (relative 29, 244, 326) and projected points of the
    // real-browser failure in gm8-frame-codex/host-2026-11-22/pointer/
    // quarry-GM3-shared-and-unique-a0-m0-normal. The frozen acceptance row is unchanged.
    let shared = point(47.999_996, 23.500_027);
    f.commands[0].tick = 28;
    f.commands[0].envelope.target = shared.clone();
    f.commands[1].tick = 243;
    f.commands[1].envelope.target = shared;
    f.commands[2].tick = 325;
    f.commands[2].envelope.target = point(19.999_996, 6.000_008);
    until(&mut f, 2420);
    let all = f.labels(|_| true);
    for label in &all {
        if !f.complete(label) {
            eprintln!(
                "INCOMPLETE {label} position={:?} order={:?} member={:?}",
                f.position(label),
                f.order(label),
                diagnostics::member(&f, label)
            );
        }
    }
    f.require_complete(&all);
}

/// Guard of the packed stop (operator decision 2026-11-22): a free place nearer the
/// point must be reached. Only ordinary Move orders build the scene: a holder on the
/// point, a wall of three resting friendly bodies on its west side, and a late unit
/// that comes from the west. The other sides of the holder are free.
#[test]
fn gm3_late_unit_reaches_the_free_place_beside_the_holder() {
    use turts_sim::Vec2;
    const POINT: Vec2 = Vec2::new(12., 10.);
    const WALL: [(&str, Vec2); 3] = [
        ("rear.small.1", Vec2::new(-0.85, 0.)),
        ("rear.small.3", Vec2::new(-0.5, 0.8)),
        ("rear.small.4", Vec2::new(-0.5, -0.8)),
    ];
    const HOLDER: &str = "rear.small.0";
    const LATE: &str = "rear.small.2";
    for variant in VARIANTS {
        let mut f = Fixture::new("quarry", "GM3-shared-and-unique", variant);
        f.commands.clear();
        let mut group = 10;
        let mut go = |f: &mut Fixture, label: &str, to: Vec2| {
            group += 1;
            let one = [label.to_string()];
            let envelope = f.envelope(
                "order_move",
                &one,
                point(to.x, to.y),
                QueueMode::Replace,
                group,
            );
            f.send(0, envelope);
        };
        let settle = |f: &mut Fixture, label: &str| {
            let end = f.tick + 400;
            while !(f.tick > 0 && f.complete(label)) {
                assert!(
                    f.tick < end,
                    "{label} did not settle: {:?} order {:?} tick {}",
                    f.position(label),
                    f.order(label),
                    f.tick
                );
                step(f);
            }
        };
        go(&mut f, HOLDER, POINT);
        step(&mut f);
        settle(&mut f, HOLDER);
        for (label, offset) in WALL {
            go(&mut f, label, POINT + offset);
            step(&mut f);
            settle(&mut f, label);
            assert!(f.position(label).distance(POINT + offset) <= 1.0e-4);
        }
        // The late unit leaves its start row only after its neighbours left it.
        go(&mut f, LATE, POINT + Vec2::new(-6., 0.));
        step(&mut f);
        settle(&mut f, LATE);
        assert_eq!(f.position(HOLDER), POINT);
        let radius = f.actor(LATE).profile.radius;
        let contact = 2. * radius + 0.101;
        go(&mut f, LATE, POINT);
        step(&mut f);
        let end = f.tick + 600;
        while !f.complete(LATE) {
            assert!(f.tick < end, "the late unit did not complete");
            step(&mut f);
        }
        let stop = f.position(LATE);
        let d = turts_sim::terrain::privileged_navigation_audit(f.app.world()).diagnostics;
        eprintln!(
            "guard a{} m{}: tick {} stop {:?} distance {} packed_stops {} packed_steps {}",
            variant.allocation as u8,
            variant.members as u8,
            f.tick,
            stop - POINT,
            stop.distance(POINT),
            d.packed_stops,
            d.packed_steps
        );
        // Arrival at the free place: the unit ended in contact with the holder. A stop
        // behind the wall (distance 1.65 or more) is a premature completion.
        assert!(
            stop.distance(POINT) <= contact,
            "a{} m{}: the late unit completed {} from the point, behind the wall",
            variant.allocation as u8,
            variant.members as u8,
            stop.distance(POINT)
        );
        for (label, offset) in WALL {
            assert!(f.position(label).distance(POINT + offset) <= 1.0e-4);
        }
        assert_eq!(f.position(HOLDER), POINT);
    }
}

/// Packed stop behind the holder (operator decision 2026-11-22), in a full simulation.
/// Only ordinary Move orders are used, and the frozen scene is not changed. The large
/// unit `rear.large.2` rests on the point, so it is the holder. The start rows of the
/// small units stand around it as stopped friendly bodies.
/// 1. `rear.small.3` is boxed out in contact with the holder: its move ends there.
/// 2. `rear.small.0` is boxed out in a later row, in contact with a pack body and not
///    with the holder: its move ends there. This is the second-row case.
/// 3. Control: `rear.small.2` has a free way. It reaches the holder by the earlier rules
///    and the packed stop does not run.
///
/// Without the packed stop, units 1 and 2 stay `Blocked` and this test is red.
#[test]
fn gm3_packed_stop_ends_boxed_out_moves_in_the_first_and_in_a_later_row() {
    use turts_sim::Vec2;
    const POINT: Vec2 = Vec2::new(17.3, 27.0);
    const HOLDER: &str = "rear.large.2";
    const DEADLINE: u32 = 400;
    const CONTACT: f32 = 0.101;
    let mut poses = Vec::new();
    for variant in VARIANTS {
        let mut f = Fixture::new("quarry", "GM3-shared-and-unique", variant);
        f.commands.clear();
        let name = format!("a{} m{}", variant.allocation as u8, variant.members as u8);
        let start: BTreeMap<String, Vec2> = f
            .labels(|_| true)
            .into_iter()
            .map(|label| (label.clone(), f.position(&label)))
            .collect();
        let gap = |f: &Fixture, a: &str, b: &str| {
            f.position(a).distance(f.position(b))
                - f.actor(a).profile.radius
                - f.actor(b).profile.radius
        };
        let stops = |f: &Fixture| {
            let d = turts_sim::terrain::privileged_navigation_audit(f.app.world()).diagnostics;
            assert_eq!(d.packed_steps, 0, "{name}: no guard step is expected");
            d.packed_stops
        };
        let mut group = 10;
        let mut order = |f: &mut Fixture, label: &str| {
            group += 1;
            let one = [label.to_string()];
            let envelope = f.envelope(
                "order_move",
                &one,
                point(POINT.x, POINT.y),
                QueueMode::Replace,
                group,
            );
            f.send(0, envelope);
            let end = f.tick + DEADLINE;
            step(f);
            while !f.complete(label) {
                assert!(
                    f.tick < end,
                    "{name} tick{}: {label} did not complete: {:?} at {:?}",
                    f.tick,
                    f.order(label),
                    f.position(label)
                );
                step(f);
            }
        };
        // The holder occupies the point: a small unit cannot stand on it.
        assert!(
            f.position(HOLDER).distance(POINT)
                < f.actor(HOLDER).profile.radius + f.actor("rear.small.3").profile.radius
        );

        // 1. First row: contact with the holder, boxed by a stopped friendly body.
        order(&mut f, "rear.small.3");
        assert_eq!(stops(&f), 1, "{name}: the first stop is a packed stop");
        let first = gap(&f, "rear.small.3", HOLDER);
        assert!((-1.0e-4..=CONTACT).contains(&first), "{name}: gap {first}");

        // 2. Later row: contact with a pack body, no contact with the holder.
        order(&mut f, "rear.small.0");
        assert_eq!(stops(&f), 2, "{name}: the second stop is a packed stop");
        assert!(
            gap(&f, "rear.small.0", HOLDER) > 1.0,
            "{name}: not a first-row stop"
        );
        let behind = gap(&f, "rear.small.0", "rear.small.4");
        assert!(
            (-1.0e-4..=CONTACT).contains(&behind),
            "{name}: gap {behind}"
        );
        assert!(
            f.position("rear.small.0").distance(POINT)
                > f.position("rear.small.3").distance(POINT) + CONTACT
        );

        // 3. Control: a unit with a free way reaches the holder; no packed stop runs.
        order(&mut f, "rear.small.2");
        assert_eq!(
            stops(&f),
            2,
            "{name}: the control must not use the packed stop"
        );
        let free = gap(&f, "rear.small.2", HOLDER);
        assert!((-1.0e-4..=CONTACT).contains(&free), "{name}: gap {free}");

        // The rule moves no stopped body: every other unit is at its start pose.
        for (label, pose) in &start {
            if !["rear.small.3", "rear.small.0", "rear.small.2"].contains(&label.as_str()) {
                assert_eq!(f.position(label), *pose, "{name}: {label} moved");
            }
        }
        poses.push(
            ["rear.small.3", "rear.small.0", "rear.small.2"]
                .map(|label| f.position(label).to_array().map(f32::to_bits)),
        );
    }
    // No unit ID decides a result: the four ID variants give the same poses.
    assert!(poses.windows(2).all(|pair| pair[0] == pair[1]), "{poses:?}");
}

/// Packed stop, condition 1 (order clock): a late unit whose frame target is a private
/// or spread slot of its order, not the point. `rear.large.2` rests on the point. On
/// the earlier clock the second unit stayed `Blocked` for 3000 ticks or more.
fn packed_stop_ends_the_move_of_two_units_ordered_together(late: [&str; 2], group_order: bool) {
    use turts_sim::Vec2;
    const POINT: Vec2 = Vec2::new(17.3, 27.0);
    const DEADLINE: u32 = 600;
    let mut results = Vec::new();
    for variant in VARIANTS {
        let name = format!("a{} m{}", variant.allocation as u8, variant.members as u8);
        let mut f = Fixture::new("quarry", "GM3-shared-and-unique", variant);
        f.commands.clear();
        let labels = late.map(str::to_string);
        let start: BTreeMap<String, Vec2> = f
            .labels(|actor| !late.contains(&actor.label.as_str()))
            .into_iter()
            .map(|label| (label.clone(), f.position(&label)))
            .collect();
        if group_order {
            let envelope = f.envelope(
                "order_move",
                &labels,
                point(POINT.x, POINT.y),
                QueueMode::Replace,
                90,
            );
            f.send(0, envelope);
        } else {
            for (n, label) in labels.iter().enumerate() {
                let envelope = f.envelope(
                    "order_move",
                    std::slice::from_ref(label),
                    point(POINT.x, POINT.y),
                    QueueMode::Replace,
                    90 + n as u32,
                );
                f.send(0, envelope);
            }
        }
        step(&mut f);
        let mut done = [None; 2];
        while done.iter().any(Option::is_none) {
            for (slot, label) in done.iter_mut().zip(&labels) {
                if slot.is_none() && f.complete(label) {
                    *slot = Some(f.tick);
                }
            }
            if done.iter().all(Option::is_some) {
                break;
            }
            assert!(
                f.tick < DEADLINE,
                "{name} tick{}: not complete: {:?} at {:?}, {:?} at {:?}",
                f.tick,
                f.order(&labels[0]),
                f.position(&labels[0]),
                f.order(&labels[1]),
                f.position(&labels[1])
            );
            step(&mut f);
        }
        let d = turts_sim::terrain::privileged_navigation_audit(f.app.world()).diagnostics;
        eprintln!(
            "{name} {late:?} group_order {group_order}: completion ticks {done:?} poses {:?} {:?} packed_stops {} packed_steps {}",
            f.position(&labels[0]),
            f.position(&labels[1]),
            d.packed_stops,
            d.packed_steps
        );
        assert!(d.packed_stops >= 1, "{name}: no packed stop ran");
        // Each unit ended clear of every other body, and no stopped body moved.
        for label in &labels {
            for other in f.labels(|actor| actor.label != *label) {
                let gap = f.position(label).distance(f.position(&other))
                    - f.actor(label).profile.radius
                    - f.actor(&other).profile.radius;
                assert!(gap >= -1.0e-4, "{name}: {label} overlaps {other}: {gap}");
            }
        }
        for (label, pose) in &start {
            assert_eq!(f.position(label), *pose, "{name}: {label} moved");
        }
        results.push((
            done,
            labels
                .clone()
                .map(|label| f.position(&label).to_array().map(f32::to_bits)),
        ));
    }
    // No unit ID decides a result: the four ID variants are equal.
    assert!(
        results.windows(2).all(|pair| pair[0] == pair[1]),
        "{results:?}"
    );
}

#[test]
fn gm3_packed_stop_ends_two_orders_accepted_in_one_tick() {
    packed_stop_ends_the_move_of_two_units_ordered_together(
        ["rear.small.0", "rear.small.2"],
        false,
    );
}

#[test]
fn gm3_packed_stop_ends_a_group_order_at_a_held_point() {
    packed_stop_ends_the_move_of_two_units_ordered_together(["rear.small.1", "rear.small.2"], true);
}

#[test]
fn gm2_browser_projected_destination_and_delayed_orders_complete() {
    let mut f = Fixture::new("quarry", "GM2-narrow", VARIANTS[0]);
    // These are the actual order application ticks and projected point from
    // the real-browser failure, not a change to the frozen acceptance row.
    f.commands[0].tick = 712;
    f.commands[0].envelope.target = point(50.000_004, 24.000_015);
    f.commands[1].tick = 3219;
    let large = f.labels(|a| a.kind == "large");
    until(&mut f, 3219);
    let mut opened = None;
    for _ in 0..=MOVE_BOUND {
        step(&mut f);
        if f.surface.rectangle_is([30, 20], [34, 28], 0) {
            opened = Some(f.tick);
            break;
        }
    }
    let opened = opened.expect("ordinary work did not open the passage");
    until(&mut f, opened + MOVE_BOUND);
    for label in &large {
        if !f.complete(label) {
            eprintln!(
                "INCOMPLETE {label} position={:?} order={:?} member={:?}",
                f.position(label),
                f.order(label),
                diagnostics::member(&f, label)
            );
        }
    }
    require_crossed(&f, &large, true);
}

#[test]
#[ignore = "GM4 throughput diagnosis: per-tick cost and navigation counters; not acceptance"]
fn diagnostic_gm4_34_unit_order_tick_cost_and_counters() {
    let mut f = Fixture::new("quarry", "GM4-edit-during-approach", VARIANTS[0]);
    // Lena's server-only script: the 34-unit order at tick 200, no terrain edit.
    f.commands.truncate(1);
    f.commands[0].tick = 200;
    let mut rows = Vec::new();
    let group = f.commands[0].envelope.units.clone();
    let labels = f.labels(|actor| group.iter().any(|unit| unit.0 == actor.unit));
    let (mut first_direct, mut last_arrival) = (None, None);
    while f.tick < 2640 && last_arrival.is_none() {
        let start = std::time::Instant::now();
        f.step();
        let ns = start.elapsed().as_nanos() as u64;
        let audit = turts_sim::terrain::privileged_navigation_audit(f.app.world());
        if first_direct.is_none()
            && audit
                .members
                .iter()
                .any(|member| member.state == turts_sim::terrain::NavigationMemberState::Direct)
        {
            first_direct = Some(f.tick);
        }
        if f.tick > 200 && labels.iter().all(|label| f.complete(label)) {
            last_arrival = Some(f.tick);
        }
        rows.push(format!("{} {ns} {:?}", f.tick, audit.diagnostics));
    }
    rows.push(format!(
        "SUMMARY first_direct={first_direct:?} last_arrival={last_arrival:?} members={}",
        labels.len()
    ));
    let path = std::env::var("TURTS_GM4_DIAG_OUT").expect("TURTS_GM4_DIAG_OUT");
    std::fs::write(path, rows.join("\n")).unwrap();
}
