//! Independent acceptance tests for the public terrain boundary.
use bevy::prelude::*;
use std::{path::PathBuf, sync::Arc};
use turts_client::{
    content::ClientContent,
    replica::{Replica, ReplicaContext},
};
use turts_mod_api::{ContentId, ObservationPhase, TerrainCell};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    visibility_transaction::{
        StateTransactionAssembler, StateTransactionFrame, encode_transaction,
    },
    *,
};
use turts_sim::{CompletedWorld, SlotId as SimSlotId};

struct Fixture {
    app: App,
    archive: PackageArchive,
}
impl Fixture {
    fn new(package: &str, seed: u64) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../mods")
            .join(package);
        let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
        let mut app = turts_sim::headless_app(20);
        turts_pack::load_package_with_terrain_seed(
            app.world_mut(),
            &archive,
            Some(turts_sim::terrain::PrivateTerrainSeed::new(seed)),
        )
        .unwrap();
        turts_sim::visibility::initialize_context(
            app.world_mut(),
            9001,
            &(0..turts_mod_api::SLOT_COUNT).collect::<Vec<_>>(),
        )
        .unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        Self { app, archive }
    }
    fn view(&self) -> &turts_sim::WorldView {
        &self.app.world().resource::<CompletedWorld>().0
    }
    fn baseline(&self) -> crate::export::PermittedBaseline {
        let permissions = crate::permissions::completed(self.app.world());
        crate::export::PermittedBaseline::capture(self.view(), SimSlotId(0), permissions.0.as_ref())
    }
    fn stamp(&self, transaction_id: u64) -> StateTransactionStamp {
        let basis = self.baseline().knowledge.unwrap().basis;
        StateTransactionStamp {
            basis,
            transaction_id,
            parent_transaction_id: None,
        }
    }
}

#[test]
fn real_packages_export_the_complete_public_dictionary_and_only_permitted_samples() {
    for package in ["quarry", "terrain_stockyard"] {
        let fixture = Fixture::new(package, 159357);
        let baseline = fixture.baseline();
        let terrain = baseline
            .state
            .records
            .terrain
            .as_ref()
            .expect("terrain snapshot must exist");
        let catalog = fixture.archive.content().unwrap();
        let expected = catalog
            .terrain_materials
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(
            terrain.materials, expected,
            "dictionary must include public buried material identities"
        );
        let phase = fixture.view().terrain.as_ref().unwrap();
        assert_eq!(phase.observation_phase(), ObservationPhase::Completed);
        let grid = phase.grid();
        let mut expected_cells = 0;
        let mut unknown_cells = 0;
        for y in 0..grid.rows {
            for x in 0..grid.columns {
                let cell = TerrainCell { x, y };
                let sample = phase.surface(0, cell);
                let exported = terrain.surfaces.iter().find(|row| row.cell == cell);
                match sample {
                    None => {
                        unknown_cells += 1;
                        assert!(exported.is_none(), "unknown cell {cell:?} leaked");
                    }
                    Some(sample) => {
                        expected_cells += 1;
                        let row = exported.expect("permitted surface omitted");
                        assert_eq!(row.elevation, sample.elevation);
                        assert_eq!(row.observed_tick, sample.last_seen_tick);
                        assert_eq!(row.current, sample.current);
                        assert_eq!(row.provenance, sample.provenance);
                        assert_eq!(
                            row.material.map(|id| &terrain.materials[id.0 as usize]),
                            sample.material.as_ref().map(|id| id.to_string()).as_ref()
                        );
                    }
                }
            }
        }
        assert!(
            expected_cells > 0 && unknown_cells > 0,
            "fixture must cover mixed knowledge"
        );
        assert_eq!(
            terrain.surfaces.len(),
            expected_cells,
            "duplicate or extra surface rows"
        );
        for unit in &baseline.state.units {
            let anchor = terrain
                .anchors
                .iter()
                .find(|row| row.unit == unit.unit_id)
                .expect("live ground contact omitted");
            let contact = phase.ground_anchor(0, unit.unit_id.0).unwrap();
            assert_eq!(
                (anchor.elevation, anchor.observed_tick),
                (contact.elevation, contact.observed_tick)
            );
        }
    }
}

#[test]
fn real_permitted_snapshot_crosses_bounded_frames_before_atomic_client_activation() {
    for package in ["quarry", "terrain_stockyard"] {
        let fixture = Fixture::new(package, 271828);
        let baseline = fixture.baseline();
        let stamp = fixture.stamp(1);
        let payload = StateTransactionPayload::Snapshot {
            state: Box::new(baseline.state),
            knowledge: baseline.knowledge,
            event_units: vec![],
        };
        let content = ClientContent::from_archives(&[fixture.archive]).unwrap();
        let mut replica = Replica::new(20);
        replica.set_context(ReplicaContext::new(content, 0));
        let mut assembler =
            StateTransactionAssembler::new(stamp.basis.match_id, stamp.basis.map.clone(), 512);
        let mut encoded = encode_transaction(stamp.clone(), &payload, 512).unwrap();
        let mut chunks = 0;
        let mut completed = false;
        while let Some(frame) = encoded.next_frame() {
            assert!(frame.encoded_bytes().unwrap() <= 512);
            match frame {
                StateTransactionFrame::Begin(begin) => assembler.begin(begin, 0).unwrap(),
                StateTransactionFrame::Chunk(chunk) => {
                    assembler.chunk(&chunk).unwrap();
                    chunks += 1;
                }
                StateTransactionFrame::End(end) => {
                    let staged = assembler.end(&end).unwrap().unwrap();
                    assert!(replica.terrain.is_none(), "staging activated terrain early");
                    assert!(
                        replica.activate_transaction(
                            &staged.stamp,
                            &staged.payload,
                            bevy::platform::time::Instant::now()
                        ),
                        "actual server projection rejected by client"
                    );
                    assembler.activate(&staged.stamp).unwrap();
                    completed = true;
                }
            }
            if !completed {
                assert!(replica.terrain.is_none());
                assert!(replica.units.is_empty());
            }
        }
        assert!(
            chunks > 1,
            "test must exercise interrupted multi-frame state"
        );
        assert!(completed);
        assert_eq!(
            replica.economy,
            match &payload {
                StateTransactionPayload::Snapshot { state, .. } => state.economy.clone(),
                _ => unreachable!(),
            }
        );
        assert!(replica.terrain.is_some());
    }
}

#[test]
fn retained_completed_phase_cannot_follow_later_simulation_ticks() {
    let mut fixture = Fixture::new("quarry", 314159);
    let old_view = fixture.view().clone();
    let old_phase = Arc::clone(old_view.terrain.as_ref().unwrap());
    let grid = old_phase.grid();
    let before_cells = (0..grid.rows)
        .flat_map(|y| (0..grid.columns).map(move |x| TerrainCell { x, y }))
        .map(|cell| old_phase.surface(0, cell))
        .collect::<Vec<_>>();
    for _ in 0..4 {
        fixture.app.update();
    }
    assert_eq!(old_phase.tick(), old_view.tick);
    let after_cells = (0..grid.rows)
        .flat_map(|y| (0..grid.columns).map(move |x| TerrainCell { x, y }))
        .map(|cell| old_phase.surface(0, cell))
        .collect::<Vec<_>>();
    assert_eq!(
        before_cells, after_cells,
        "retained phase changed after live world advanced"
    );
    let knowledge = old_phase.knowledge_basis().unwrap();
    assert_eq!(knowledge.tick, old_view.tick);
}

fn geological_pair(seed: u64) -> Fixture {
    let mut fixture = Fixture::new("quarry", seed);
    let catalog = fixture.archive.content().unwrap();
    let (file, mut map) = fixture.archive.maps().unwrap().remove(0);
    // Change only the buried band. Both variants retain exactly the same public rules.
    let turts_mod_api::TerrainGenerationDef::Version1 { bands } =
        &mut map.terrain.as_mut().unwrap().generation;
    bands[0]
        .choices
        .push(turts_mod_api::TerrainWeightedMaterial {
            material: "quarry:soil_layer".into(),
            weight: 1,
        });
    let resolved = turts_mod_runtime::terrain::validate_terrain_map(
        &fixture.archive.id(),
        &file,
        &map,
        &catalog,
    )
    .unwrap();
    let map_id = ContentId::parse("quarry:field").unwrap();
    let staged = turts_sim::stage_package_with_terrain(
        fixture.app.world(),
        catalog,
        turts_mod_api::SourceDef { file, value: map },
        20,
        map_id,
        resolved,
        Some(turts_sim::terrain::PrivateTerrainSeed::new(seed)),
    )
    .unwrap();
    turts_sim::install_package(fixture.app.world_mut(), staged);
    turts_sim::visibility::initialize_context(
        fixture.app.world_mut(),
        9001,
        &(0..turts_mod_api::SLOT_COUNT).collect::<Vec<_>>(),
    )
    .unwrap();
    turts_sim::initialize_completed_world(fixture.app.world_mut());
    fixture
}

fn terrain_pages(fixture: &Fixture) -> Vec<turts_bot::QueryPage> {
    use turts_bot::{BotQueries, PageState, Query, TerrainQuery};
    let permissions = crate::permissions::completed(fixture.app.world());
    let projected = super::projection::project(
        fixture.view(),
        0,
        77,
        Arc::new(fixture.archive.content().unwrap()),
        vec![0, 1, 2, 3],
        turts_mod_api::WorldPos { x: 32., y: 24. },
        permissions.0.as_ref(),
    );
    let mut lease = super::observation::CandidateLease::new(projected.basis, 0);
    let mut queries = lease.queries(
        "independent-terrain-boundary",
        turts_mod_api::BotQuota {
            queries: 64,
            candidates: 1024,
            rows: 1024,
            proposals: 0,
        },
    );
    let query = Query::Terrain(TerrainQuery {
        cells: turts_mod_api::TerrainRect {
            min: TerrainCell { x: 0, y: 7 },
            max: TerrainCell { x: 32, y: 9 },
        },
    });
    let mut cursor = None;
    let mut pages = vec![];
    loop {
        let page = queries.query(query.clone(), cursor, 7).unwrap();
        cursor = match &page.state {
            PageState::Complete => None,
            PageState::More(cursor) => Some(cursor.clone()),
            PageState::UnresolvedTie(_) => {
                panic!("coordinate enumeration cannot have identity ties")
            }
        };
        pages.push(page);
        if cursor.is_none() {
            break;
        }
        assert!(
            pages.len() <= 10,
            "pagination failed to make bounded progress"
        );
    }
    pages
}

#[test]
fn hidden_geology_changes_neither_real_human_payload_nor_paginated_bot_metadata() {
    let left = geological_pair(1234567);
    let right = geological_pair(7654321);
    let left_truth = turts_sim::terrain::privileged_audit(left.app.world()).unwrap();
    let right_truth = turts_sim::terrain::privileged_audit(right.app.world()).unwrap();
    assert_ne!(
        left_truth.columns_state, right_truth.columns_state,
        "paired fixture did not change actual buried geology"
    );
    let baseline = left.baseline();
    let other = right.baseline();
    assert_eq!(
        baseline.state, other.state,
        "human snapshot leaked buried geology"
    );
    assert_eq!(baseline.knowledge, other.knowledge);
    let left_payload = StateTransactionPayload::Snapshot {
        state: Box::new(baseline.state),
        knowledge: baseline.knowledge,
        event_units: vec![],
    };
    let right_payload = StateTransactionPayload::Snapshot {
        state: Box::new(other.state),
        knowledge: other.knowledge,
        event_units: vec![],
    };
    let mut a = encode_transaction(left.stamp(1), &left_payload, 512).unwrap();
    let mut b = encode_transaction(right.stamp(1), &right_payload, 512).unwrap();
    loop {
        let next_a = a.next_frame();
        let next_b = b.next_frame();
        assert_eq!(
            next_a, next_b,
            "wire bytes/shape/count leaked hidden geology"
        );
        if next_a.is_none() {
            break;
        }
    }
    let pages = terrain_pages(&left);
    assert_eq!(
        pages,
        terrain_pages(&right),
        "bot results or public budget/cursor metadata leaked geology"
    );
    let rows = pages
        .iter()
        .flat_map(|page| match &page.data {
            turts_bot::QueryData::Terrain(rows) => rows.iter(),
            _ => panic!("wrong query response"),
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 64, "unknown coordinates must not be filtered");
    assert!(
        rows.iter()
            .any(|row| matches!(row.knowledge, turts_bot::TerrainCellKnowledge::Unknown))
    );
    assert!(
        rows.iter()
            .any(|row| matches!(row.knowledge, turts_bot::TerrainCellKnowledge::Current(_)))
    );
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(
            row.cell,
            TerrainCell {
                x: (index % 32) as u32,
                y: 7 + (index / 32) as u32
            }
        );
    }
    let json = serde_json::to_value(&left_payload).unwrap();
    fn no_private_fields(value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(fields) => {
                for (name, value) in fields {
                    assert!(
                        ![
                            "seed",
                            "private_seed",
                            "columns_state",
                            "natural",
                            "natural_top",
                            "source_revision"
                        ]
                        .contains(&name.as_str()),
                        "private terrain field {name} in human payload"
                    );
                    no_private_fields(value);
                }
            }
            serde_json::Value::Array(rows) => {
                for row in rows {
                    no_private_fields(row);
                }
            }
            _ => {}
        }
    }
    no_private_fields(&json);
}

#[test]
fn actual_excavation_publishes_height_and_item_credit_in_one_activated_patch() {
    let mut fixture = Fixture::new("quarry", 424242);
    let mut baseline = fixture.baseline();
    let worker = baseline
        .state
        .units
        .iter()
        .find(|unit| unit.owner_slot.0 == 0 && unit.content_id == "quarry:walker")
        .unwrap()
        .unit_id;
    let cell = TerrainCell { x: 15, y: 8 };
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(
        ClientContent::from_archives(&[fixture.archive.clone()]).unwrap(),
        0,
    ));
    let initial_stamp = fixture.stamp(1);
    let initial_payload = StateTransactionPayload::Snapshot {
        state: Box::new(baseline.state.clone()),
        knowledge: baseline.knowledge.clone(),
        event_units: vec![],
    };
    assert!(replica.activate_transaction(
        &initial_stamp,
        &initial_payload,
        bevy::platform::time::Instant::now()
    ));
    let command = turts_sim::Command {
        slot: SimSlotId(0),
        seq: 1,
        kind: turts_sim::CommandKind::Order(Box::new(turts_sim::orders::OrderCommand {
            order: ContentId::parse("quarry:order_work").unwrap(),
            behaviour: turts_mod_api::Behaviour::TerrainWork,
            target_kind: turts_mod_api::TargetKind::TerrainDesignation,
            policy: None,
            units: vec![worker.0],
            target: turts_sim::orders::OrderTargetSim::TerrainWork {
                tool: ContentId::parse("quarry:spade").unwrap(),
                cells: turts_mod_api::TerrainRect {
                    min: cell,
                    max: TerrainCell { x: 16, y: 9 },
                },
                target_elevation: 3,
            },
            queue: turts_sim::orders::QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    };
    assert!(
        turts_sim::apply_command(fixture.app.world_mut(), &command)
            .outcome
            .is_ok()
    );
    let item_count = |economy: &EconomyState| {
        economy
            .inventories
            .iter()
            .filter(|row| row.unit == worker)
            .flat_map(|row| &row.items)
            .filter(|item| item.resource == "quarry:soil")
            .map(|item| item.on_hand)
            .sum::<u64>()
    };
    let mut assembler = StateTransactionAssembler::new(
        initial_stamp.basis.match_id,
        initial_stamp.basis.map.clone(),
        512,
    );
    let mut initial_encoded =
        encode_transaction(initial_stamp.clone(), &initial_payload, 512).unwrap();
    while let Some(frame) = initial_encoded.next_frame() {
        match frame {
            StateTransactionFrame::Begin(begin) => assembler.begin(begin, 0).unwrap(),
            StateTransactionFrame::Chunk(chunk) => assembler.chunk(&chunk).unwrap(),
            StateTransactionFrame::End(end) => {
                assembler.end(&end).unwrap().unwrap();
                assembler.activate(&initial_stamp).unwrap();
            }
        }
    }
    let initial_items = item_count(&replica.economy);
    let mut transaction_id = 1;
    for _ in 0..200 {
        fixture.app.update();
        let next = fixture.baseline();
        let payload = baseline.patch(
            &next,
            fixture.app.world().resource::<turts_sim::CommittedTick>(),
            SimSlotId(0),
            &turts_sim::visibility::event_units(fixture.app.world(), 0),
        );
        transaction_id += 1;
        let mut stamp = fixture.stamp(transaction_id);
        stamp.parent_transaction_id = Some(transaction_id - 1);
        let mut encoded = encode_transaction(stamp.clone(), &payload, 512).unwrap();
        // The active client remains unchanged while the next transaction is incomplete.
        let old_height = replica.terrain.as_ref().unwrap().elevation(cell);
        let old_items = item_count(&replica.economy);
        let mut activated = false;
        while let Some(frame) = encoded.next_frame() {
            match frame {
                StateTransactionFrame::Begin(begin) => assembler.begin(begin, 0).unwrap(),
                StateTransactionFrame::Chunk(chunk) => assembler.chunk(&chunk).unwrap(),
                StateTransactionFrame::End(end) => {
                    let staged = assembler.end(&end).unwrap().unwrap();
                    assert_eq!(
                        replica.terrain.as_ref().unwrap().elevation(cell),
                        old_height
                    );
                    assert_eq!(item_count(&replica.economy), old_items);
                    assert!(
                        replica.activate_transaction(
                            &staged.stamp,
                            &staged.payload,
                            bevy::platform::time::Instant::now()
                        ),
                        "ordinary server work patch rejected"
                    );
                    assembler.activate(&staged.stamp).unwrap();
                    activated = true;
                }
            }
            if !activated {
                assert_eq!(
                    replica.terrain.as_ref().unwrap().elevation(cell),
                    old_height
                );
                assert_eq!(item_count(&replica.economy), old_items);
            }
        }
        assert!(activated);
        let height = replica.terrain.as_ref().unwrap().elevation(cell);
        let items = item_count(&replica.economy);
        assert!(
            matches!((height, items - initial_items), (Some(4), 0) | (Some(3), 1)),
            "height and account activated on different bases: {height:?}, {items}"
        );
        if height == Some(3) {
            return;
        }
        baseline = next;
    }
    panic!("ordinary authored worker failed to complete one observed prism in 200 real ticks");
}

#[test]
fn tiny_all_visible_terrain_uses_atomic_sender_below_the_legacy_frame_limit() {
    let mut fixture = Fixture::new("quarry", 121212);
    let catalog = fixture.archive.content().unwrap();
    let (file, mut map) = fixture.archive.maps().unwrap().remove(0);
    map.size = turts_mod_api::MapSize { x: 2., y: 2. };
    map.visibility = None;
    map.placements.clear();
    map.nodes.clear();
    map.belts.clear();
    let terrain = map.terrain.as_mut().unwrap();
    terrain.columns = 2;
    terrain.rows = 2;
    terrain.chunk_width = 2;
    terrain.initial_heights.clear();
    let resolved = turts_mod_runtime::terrain::validate_terrain_map(
        &fixture.archive.id(),
        &file,
        &map,
        &catalog,
    )
    .unwrap();
    let staged = turts_sim::stage_package_with_terrain(
        fixture.app.world(),
        catalog,
        turts_mod_api::SourceDef { file, value: map },
        20,
        ContentId::parse("quarry:field").unwrap(),
        resolved,
        Some(turts_sim::terrain::PrivateTerrainSeed::new(121212)),
    )
    .unwrap();
    turts_sim::install_package(fixture.app.world_mut(), staged);
    turts_sim::visibility::initialize_context(
        fixture.app.world_mut(),
        9001,
        &(0..turts_mod_api::SLOT_COUNT).collect::<Vec<_>>(),
    )
    .unwrap();
    turts_sim::initialize_completed_world(fixture.app.world_mut());
    fixture.app.insert_resource(crate::stats::MatchId(9001));
    let baseline = fixture.baseline();
    assert!(baseline.knowledge.is_none());
    assert!(baseline.state.records.terrain.is_some());
    assert!(
        postcard::to_allocvec(&baseline.state).unwrap().len() <= 1024,
        "fixture must exercise the tiny legacy-path boundary"
    );
    let peer = fixture.app.world_mut().spawn_empty().id();
    assert!(
        crate::outbound::knowledge::queue_full(fixture.app.world_mut(), peer, SimSlotId(0))
            .unwrap(),
        "terrain cannot enter the legacy snapshot path, even when AllVisible fits one frame"
    );
}

#[test]
fn leaving_observation_retains_the_last_authorized_surface_and_stamp() {
    let mut fixture = Fixture::new("quarry", 333333);
    let cell = TerrainCell { x: 15, y: 8 };
    let old = fixture.baseline();
    let sample = old
        .state
        .records
        .terrain
        .as_ref()
        .unwrap()
        .surfaces
        .iter()
        .find(|row| row.cell == cell)
        .unwrap()
        .clone();
    assert!(sample.current);
    // Move sight sources out of range as test setup. The next real tick must hide this cell.
    {
        let world = fixture.app.world_mut();
        let mut sources = world.query::<(&turts_sim::Owner, &mut turts_sim::Position)>();
        for (owner, mut position) in sources.iter_mut(world) {
            if owner.0 == SimSlotId(0) {
                position.0.x = 2.;
                position.0.y = 22.;
            }
        }
    }
    fixture.app.update();
    let hidden = fixture.baseline();
    let stale = hidden
        .state
        .records
        .terrain
        .as_ref()
        .unwrap()
        .surfaces
        .iter()
        .find(|row| row.cell == cell)
        .unwrap();
    assert!(!stale.current, "out-of-range surface stayed current");
    assert_eq!(
        (stale.elevation, stale.material, stale.provenance),
        (sample.elevation, sample.material, sample.provenance)
    );
    let final_stamp = stale.observed_tick;
    for _ in 0..3 {
        fixture.app.update();
    }
    let later = fixture.baseline();
    let later = later
        .state
        .records
        .terrain
        .as_ref()
        .unwrap()
        .surfaces
        .iter()
        .find(|row| row.cell == cell)
        .unwrap();
    assert!(!later.current);
    assert_eq!(
        later.observed_tick, final_stamp,
        "unobserved terrain refreshed its timestamp"
    );
    assert_eq!(
        (later.elevation, later.material, later.provenance),
        (stale.elevation, stale.material, stale.provenance)
    );
}
