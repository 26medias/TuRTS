//! Match outcomes through the real skirmish package and the public simulation commands.
use bevy_app::{App, FixedPostUpdate, FixedPreUpdate, FixedUpdate};
use std::path::Path;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::outcome::{MatchOutcomes, Outcome};
use turts_sim::{Command, CommandKind, SlotId, Tick, apply_command, completed_world, headless_app};

fn game() -> App {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let mut app = headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    app
}

#[test]
fn victory_does_not_infer_the_api_seat_count_from_initial_units() {
    fn files(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_str().unwrap().into(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let mut content = std::collections::BTreeMap::new();
    files(&root, &root, &mut content);
    let path = content
        .keys()
        .find(|path| path.starts_with("maps/"))
        .unwrap()
        .clone();
    let mut map: turts_mod_api::MapDef = ron::de::from_bytes(&content[&path]).unwrap();
    map.placements.retain(|unit| unit.owner_slot == 0);
    content.insert(path, ron::ser::to_string(&map).unwrap().into_bytes());
    let archive =
        PackageArchive::parse(&turts_mod_runtime::pack::pack_files(&content).unwrap()).unwrap();
    let mut app = headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    assert_eq!(
        app.world().resource::<turts_sim::Slots>().count(),
        turts_mod_api::SLOT_COUNT
    );
    assert_eq!(
        app.world()
            .resource::<turts_sim::outcome::MatchRules>()
            .participants,
        (0..turts_mod_api::SLOT_COUNT).collect::<Vec<_>>()
    );
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedPreUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedPostUpdate);
}

fn remove_base(app: &mut App, slot: u8) {
    let id = completed_world(app.world())
        .units
        .iter()
        .find(|unit| unit.owner_slot == slot && unit.content_id == "skirmish:base")
        .unwrap()
        .unit_id;
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(slot),
            seq: 1,
            kind: CommandKind::Remove { unit_id: id },
        },
    );
    assert!(result.outcome.is_ok(), "{result:?}");
}

#[test]
fn both_teams_keep_playing_while_their_bases_survive() {
    let mut app = game();
    for _ in 0..5 {
        tick(&mut app);
    }
    let state = app.world().resource::<MatchOutcomes>();
    assert!(state.slots.iter().all(Option::is_none));
    assert_eq!(state.end_tick, None);
}

#[test]
fn losing_the_enemy_or_own_base_gives_the_server_result_and_freezes_the_match() {
    for defeated in [0, 1] {
        let mut app = game();
        tick(&mut app);
        remove_base(&mut app, defeated);
        tick(&mut app);
        let state = app.world().resource::<MatchOutcomes>();
        assert_eq!(
            state.slots[usize::from(defeated)].unwrap().outcome,
            Outcome::Defeat
        );
        assert_eq!(
            state.slots[usize::from(1 - defeated)].unwrap().outcome,
            Outcome::Victory
        );
        let end_tick = state.end_tick.unwrap();
        let before = turts_sim::audit_state(app.world_mut());
        for _ in 0..5 {
            tick(&mut app);
        }
        assert_eq!(
            app.world().resource::<Tick>().0,
            end_tick,
            "the match clock continued after its result"
        );
        assert_eq!(
            turts_sim::audit_state(app.world_mut()),
            before,
            "the ended simulation changed"
        );
        let denied = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(1 - defeated),
                seq: 2,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![3],
                    target: turts_sim::Vec2::new(30.0, 20.0),
                },
            },
        );
        assert_eq!(denied.outcome, Err(turts_sim::RejectReason::MatchOver));
    }
}

#[test]
fn losing_both_bases_in_one_tick_is_a_draw_in_either_command_order() {
    for order in [[0, 1], [1, 0]] {
        let mut app = game();
        for slot in order {
            remove_base(&mut app, slot);
        }
        tick(&mut app);
        let state = app.world().resource::<MatchOutcomes>();
        assert_eq!(state.slots[0].unwrap().outcome, Outcome::Draw);
        assert_eq!(state.slots[1].unwrap().outcome, Outcome::Draw);
    }
}
