use super::health::{health_bar_visible, health_fraction, update_health_fill_transform};
use super::markers::marker_is_live_for_orders;
use super::materials::{RingOverlayExtension, configure_ring_overlay_depth};
use super::world_tooltip::tooltip_text;
use super::*;
use bevy::mesh::VertexAttributeValues;
use bevy::render::render_resource::{DepthBiasState, StencilState, TextureFormat};
use std::time::Duration;
use turts_net_protocol::{OrderEnvelope, QueueMode, UnitId};

#[test]
fn ring_overlay_uses_shared_per_slot_colours_and_depth_overlay_state() {
    let mut materials = Assets::<StandardMaterial>::default();
    let palette = [Color::srgb_u8(0, 114, 178), Color::srgb_u8(230, 159, 0)];
    let handles = palette
        .iter()
        .copied()
        .map(|base_color| {
            materials.add(StandardMaterial {
                base_color,
                unlit: true,
                ..default()
            })
        })
        .collect::<Vec<_>>();
    let mut overlays = Assets::<RingOverlayMaterial>::default();
    let overlay_handles = create_ring_overlay_materials(&handles, &materials, &mut overlays)
        .expect("each overlay must correspond to one body palette slot");
    assert_eq!(overlay_handles.len(), palette.len());
    let neutral = create_neutral_ring_overlay_materials(&mut overlays);
    assert_eq!(
        overlays.len(),
        palette.len() + 2,
        "two neutral shared underlays serve every foreign ring"
    );
    for (handle, expected) in overlay_handles.iter().zip(palette) {
        let overlay = overlays.get(handle).expect("shared slot overlay exists");
        assert_eq!(overlay.base.base_color, expected);
        assert!(overlay.base.unlit);
        assert_eq!(overlay.base.alpha_mode, AlphaMode::Blend);
    }
    let neutral_colours = neutral
        .iter()
        .map(|handle| {
            let neutral = overlays
                .get(handle)
                .expect("shared neutral ring underlay exists");
            assert!(neutral.base.unlit);
            assert_eq!(neutral.base.alpha_mode, AlphaMode::Blend);
            neutral.base.base_color
        })
        .collect::<Vec<_>>();
    assert_eq!(
        neutral_colours,
        [Color::srgb(0.96, 0.96, 0.96), Color::BLACK]
    );

    let cases = [
        (Color::WHITE, 1, "white slot gets black contrast"),
        (Color::BLACK, 0, "black slot gets white contrast"),
        (
            Color::srgb_u8(230, 159, 0),
            0,
            "default orange keeps white contrast",
        ),
    ];
    for (slot_colour, expected, message) in cases {
        let selected_backer = neutral_ring_overlay_index(slot_colour);
        assert_eq!(selected_backer, expected, "{message}");
        assert_eq!(
            neutral_colours[selected_backer],
            if expected == 0 {
                Color::srgb(0.96, 0.96, 0.96)
            } else {
                Color::BLACK
            }
        );
    }

    assert_eq!(RingOverlayExtension::alpha_mode(), Some(AlphaMode::Blend));
    assert!(!RingOverlayExtension::enable_prepass());
    assert!(!RingOverlayExtension::enable_shadows());
    let mut depth = DepthStencilState {
        format: TextureFormat::Depth32Float,
        depth_write_enabled: Some(true),
        depth_compare: Some(CompareFunction::Less),
        stencil: StencilState::default(),
        bias: DepthBiasState::default(),
    };
    configure_ring_overlay_depth(&mut depth);
    assert_eq!(depth.depth_compare, Some(CompareFunction::Always));
    assert_eq!(depth.depth_write_enabled, Some(false));
}

#[test]
fn order_markers_share_range_colour_overlay_material() {
    let mut overlays = Assets::<RingOverlayMaterial>::default();
    let marker_colour = Color::srgb_u8(242, 204, 96);
    let marker = create_marker_overlay_material(marker_colour, &mut overlays);
    let stored = overlays.get(&marker).expect("shared marker overlay exists");

    assert_eq!(
        overlays.len(),
        1,
        "all order markers use one shared material"
    );
    assert_eq!(stored.base.base_color, marker_colour);
    assert!(stored.base.unlit);
    assert_eq!(stored.base.alpha_mode, AlphaMode::Blend);
    assert_eq!(RingOverlayExtension::alpha_mode(), Some(AlphaMode::Blend));
    assert!(!RingOverlayExtension::enable_prepass());
    assert!(!RingOverlayExtension::enable_shadows());
}

#[test]
fn order_marker_gizmos_render_in_front_without_changing_other_gizmos() {
    assert_eq!(order_marker_gizmo_config().depth_bias, -1.0);
    assert_eq!(GizmoConfig::default().depth_bias, 0.0);
}

#[derive(Resource)]
struct HealthTestFraction(f32);

#[derive(Resource, Default)]
struct ChangedTransformCount(usize);

#[derive(Resource)]
struct HealthReconcileTestInput {
    desired: HashMap<VisualId, (Handle<Mesh>, VisualMaterial, Transform)>,
    fractions: HashMap<u32, f32>,
    bar_mesh: Option<Handle<Mesh>>,
    health_material: Option<Handle<StandardMaterial>>,
}

fn update_test_health_fill(
    fraction: Res<HealthTestFraction>,
    mut transforms: Query<&mut Transform>,
) {
    for mut transform in &mut transforms {
        update_health_fill_transform(&mut transform, fraction.0);
    }
}

fn count_changed_transforms(
    transforms: Query<Entity, Changed<Transform>>,
    mut count: ResMut<ChangedTransformCount>,
) {
    count.0 = transforms.iter().count();
}

fn reconcile_health_test_visuals(
    mut commands: Commands,
    mut shown: Query<
        (Entity, &VisualId, &mut Transform, Option<&mut RingMetadata>),
        Without<HealthFill>,
    >,
    input: Res<HealthReconcileTestInput>,
) {
    reconcile_visual_entities(
        &mut commands,
        &mut shown,
        input.desired.clone(),
        &HashMap::new(),
        &input.fractions,
        input.bar_mesh.clone(),
        input.health_material.clone(),
    );
}

fn marker(id: u64, seqs: &[u32]) -> RetainedMarker {
    let order = OrderEnvelope {
        order: "muster:order_move".into(),
        units: vec![UnitId(7)],
        target: OrderTarget::Point(Pos { x: 8.0, y: 9.0 }),
        queue: QueueMode::Replace,
        group: 0,
    };
    RetainedMarker {
        id,
        position: Vec2::new(8.0, 9.0),
        kind: "move".into(),
        order: order.clone(),
        fragments: seqs
            .iter()
            .map(|seq| SentOrderFragment {
                intent_id: id,
                seq: *seq,
                order: order.clone(),
            })
            .collect(),
        results: HashMap::new(),
        seen_results: HashSet::new(),
        started: Instant::now(),
    }
}

fn order_record(activity: Activity) -> UnitOrderRecord {
    UnitOrderRecord {
        unit: UnitId(7),
        activity,
        current: None,
        queued: Vec::new(),
        last_failure: None,
    }
}

#[test]
fn clicked_marker_retires_on_authoritative_result_tick_rest() {
    let now = Instant::now();
    let mut clicked = marker(1, &[4]);
    let mut orders = HashMap::from([(7, order_record(Activity::Idle))]);
    assert!(marker_is_live_for_orders(&clicked, &orders, 100, now));
    clicked.results.insert(
        4,
        MarkerOrderResult {
            tick: 101,
            accepted: vec![7],
        },
    );
    assert!(
        marker_is_live_for_orders(&clicked, &orders, 100, now),
        "stale pre-result Idle cannot retire it"
    );
    assert!(
        !marker_is_live_for_orders(&clicked, &orders, 101, now),
        "Idle at the result tick is from the post-command committed replica"
    );
    orders.insert(7, order_record(Activity::Moving));
    assert!(marker_is_live_for_orders(&clicked, &orders, 101, now));
    orders.insert(7, order_record(Activity::Blocked));
    assert!(
        !marker_is_live_for_orders(&clicked, &orders, 101, now),
        "Blocked at the result tick is also post-command terminal evidence"
    );
    orders.insert(7, order_record(Activity::Idle));
    assert!(!marker_is_live_for_orders(&clicked, &orders, 102, now));
    orders.insert(7, order_record(Activity::Blocked));
    assert!(
        !marker_is_live_for_orders(&clicked, &orders, 102, now),
        "accepted units retire the marker when they are Blocked after the result tick"
    );
}

#[test]
fn marker_lifetime_is_capped_at_sixty_seconds() {
    let now = Instant::now();
    let mut pending = marker(5, &[]);
    pending.started = now - Duration::from_secs(60);

    assert!(
        !marker_is_live_for_orders(&pending, &HashMap::new(), 0, now),
        "unacknowledged markers expire at the design's 60 second cap"
    );
}

#[test]
fn split_fragments_and_refusals_retire_only_after_all_results() {
    let now = Instant::now();
    let mut clicked = marker(2, &[10, 11]);
    clicked.results.insert(
        10,
        MarkerOrderResult {
            tick: 5,
            accepted: vec![7],
        },
    );
    let orders = HashMap::from([(7, order_record(Activity::Idle))]);
    assert!(
        marker_is_live_for_orders(&clicked, &orders, 5, now),
        "unresolved split fragment holds marker"
    );
    clicked.results.insert(
        11,
        MarkerOrderResult {
            tick: 6,
            accepted: Vec::new(),
        },
    );
    assert!(
        marker_is_live_for_orders(&clicked, &orders, 4, now),
        "replica before the accepted result tick is stale"
    );
    assert!(
        !marker_is_live_for_orders(&clicked, &orders, 5, now),
        "same-tick authoritative rest may retire after all split-fragment results arrive"
    );
    assert!(!marker_is_live_for_orders(&clicked, &orders, 6, now));

    let mut refused = marker(3, &[12]);
    refused.results.insert(
        12,
        MarkerOrderResult {
            tick: 7,
            accepted: Vec::new(),
        },
    );
    assert!(
        !marker_is_live_for_orders(&refused, &orders, 7, now),
        "refused envelope removes its marker"
    );
}

#[test]
fn replacement_intents_remain_distinct_and_session_reset_keeps_current_frame_click() {
    let now = Instant::now();
    let mut completed = marker(20, &[30]);
    completed.results.insert(
        30,
        MarkerOrderResult {
            tick: 2,
            accepted: vec![7],
        },
    );
    let pending = marker(21, &[31]);
    let orders = HashMap::from([(7, order_record(Activity::Idle))]);
    assert!(!marker_is_live_for_orders(&completed, &orders, 2, now));
    assert!(marker_is_live_for_orders(&pending, &orders, 2, now));
    assert_ne!(completed.id, pending.id);

    let mut visuals = VisualState {
        session: Some(VisualSession {
            match_id: 5,
            connection_epoch: 8,
            local_slot: 0,
        }),
        markers: VecDeque::from([pending]),
        hover_unit: Some(7),
        hover_started: Some(now),
        blocked_since: HashMap::from([(7, now)]),
        blocked_active: HashSet::from([7]),
        ..Default::default()
    };
    let mut feed = OrderMarkerFeed::default();
    feed.intents.push(OrderMarkerIntent {
        id: 21,
        kind: "move".into(),
        order: completed.order.clone(),
    });
    feed.sent.push_back(SentOrderFragment {
        intent_id: 21,
        seq: 31,
        order: completed.order.clone(),
    });
    reset_marker_session(
        &mut visuals,
        &mut feed,
        Some(VisualSession {
            match_id: 5,
            connection_epoch: 8,
            local_slot: 1,
        }),
    );
    assert_eq!(visuals.generation, 1);
    assert!(visuals.markers.is_empty());
    assert!(visuals.hover_unit.is_none());
    assert!(visuals.hover_started.is_none());
    assert!(visuals.blocked_since.is_empty());
    assert!(visuals.blocked_active.is_empty());
    assert_eq!(feed.intents.len(), 1);
    assert_eq!(feed.intents[0].id, 21);
    assert_eq!(feed.sent.len(), 1);
    assert_eq!(feed.sent[0].intent_id, 21);

    visuals.markers.push_back(marker(22, &[32]));
    feed.sent.push_back(SentOrderFragment {
        intent_id: 22,
        seq: 32,
        order: completed.order,
    });
    reset_marker_session(&mut visuals, &mut feed, None);
    assert_eq!(visuals.generation, 2);
    assert!(visuals.markers.is_empty());
    assert!(feed.intents.is_empty());
    assert!(feed.sent.is_empty());
    assert_eq!(visuals.session, None);
}

#[test]
fn first_visual_session_keeps_a_same_frame_click_intent() {
    let mut visuals = VisualState::default();
    let mut feed = OrderMarkerFeed::default();
    let clicked = marker(41, &[51]);
    feed.intents.push(OrderMarkerIntent {
        id: clicked.id,
        kind: clicked.kind.clone(),
        order: clicked.order.clone(),
    });
    feed.sent.extend(clicked.fragments.iter().cloned());

    let changed = reset_marker_session(
        &mut visuals,
        &mut feed,
        Some(VisualSession {
            match_id: 9,
            connection_epoch: 1,
            local_slot: 0,
        }),
    );

    assert!(changed);
    assert_eq!(feed.intents.len(), 1);
    assert_eq!(feed.sent.len(), 1);
}

#[test]
fn old_frame_intent_does_not_reappear_after_session_reset() {
    let old = marker(60, &[70]);
    let current = marker(61, &[71]);
    let mut visuals = VisualState {
        session: Some(VisualSession {
            match_id: 9,
            connection_epoch: 1,
            local_slot: 0,
        }),
        active: false,
        markers: VecDeque::from([old.clone()]),
        ..Default::default()
    };
    let mut feed = OrderMarkerFeed::default();
    feed.intents.push(OrderMarkerIntent {
        id: old.id,
        kind: old.kind.clone(),
        order: old.order.clone(),
    });
    feed.sent.extend(old.fragments.iter().cloned());

    // This is the CoreAdapter frame boundary. A prior-frame click is removed
    // even if this frame later exits before it can call `core.step`.
    feed.begin_interaction_frame();
    feed.intents.push(OrderMarkerIntent {
        id: current.id,
        kind: current.kind.clone(),
        order: current.order.clone(),
    });
    feed.sent.extend(current.fragments.iter().cloned());

    reset_marker_session(
        &mut visuals,
        &mut feed,
        Some(VisualSession {
            match_id: 9,
            connection_epoch: 2,
            local_slot: 1,
        }),
    );

    assert!(visuals.markers.is_empty());
    assert_eq!(feed.intents.len(), 1);
    assert_eq!(feed.intents[0].id, current.id);
    assert_eq!(feed.sent.len(), 1);
    assert_eq!(feed.sent[0].intent_id, current.id);
}

#[test]
fn owner_session_change_keeps_only_the_current_frame_intent_fragments() {
    let current = marker(42, &[52]);
    let old = marker(41, &[51]);
    let mut visuals = VisualState {
        session: Some(VisualSession {
            match_id: 9,
            connection_epoch: 1,
            local_slot: 0,
        }),
        active: true,
        markers: VecDeque::from([old.clone()]),
        ..Default::default()
    };
    let mut feed = OrderMarkerFeed::default();
    feed.intents.push(OrderMarkerIntent {
        id: current.id,
        kind: current.kind.clone(),
        order: current.order.clone(),
    });
    feed.sent.extend(old.fragments.iter().cloned());
    feed.sent.extend(current.fragments.iter().cloned());

    reset_marker_session(
        &mut visuals,
        &mut feed,
        Some(VisualSession {
            match_id: 9,
            connection_epoch: 2,
            local_slot: 1,
        }),
    );

    assert!(
        visuals.markers.is_empty(),
        "old session markers are cleared"
    );
    assert_eq!(
        feed.intents.len(),
        1,
        "same-frame current intent survives session change"
    );
    assert_eq!(
        feed.sent.len(),
        1,
        "only the current intent sent fragment survives"
    );
    assert_eq!(feed.sent[0].intent_id, current.id);
}

#[test]
fn blocked_mark_reentry_starts_a_fresh_two_second_lifetime() {
    let start = Instant::now();
    let mut active = HashSet::new();
    let mut since = HashMap::new();
    refresh_blocked_marks(HashSet::from([7]), &mut active, &mut since, start);
    refresh_blocked_marks(
        HashSet::new(),
        &mut active,
        &mut since,
        start + Duration::from_millis(1200),
    );
    refresh_blocked_marks(
        HashSet::from([7]),
        &mut active,
        &mut since,
        start + Duration::from_millis(1500),
    );
    assert_eq!(since[&7], start + Duration::from_millis(1500));
    refresh_blocked_marks(
        HashSet::new(),
        &mut active,
        &mut since,
        start + Duration::from_millis(3000),
    );
    assert!(since.contains_key(&7));
    refresh_blocked_marks(
        HashSet::new(),
        &mut active,
        &mut since,
        start + Duration::from_millis(3600),
    );
    assert!(!since.contains_key(&7));
}

#[test]
fn health_modes_fill_fraction_and_transform_guard_match_contract() {
    assert!(!health_bar_visible(HealthMode::Selected, false, 50, 50));
    assert!(health_bar_visible(HealthMode::Selected, true, 50, 50));
    assert!(!health_bar_visible(
        HealthMode::DamagedOrSelected,
        false,
        50,
        50
    ));
    assert!(health_bar_visible(
        HealthMode::DamagedOrSelected,
        false,
        49,
        50
    ));
    assert!(health_bar_visible(HealthMode::Always, false, 50, 50));
    assert_eq!(health_fraction(20, 40), Some(0.5));
    assert_eq!(health_fraction(0, 0), None);
}

#[test]
fn retained_health_fill_marks_transform_changed_only_when_fraction_changes() {
    let mut app = App::new();
    app.insert_resource(HealthTestFraction(1.0));
    app.insert_resource(ChangedTransformCount::default());
    app.world_mut().spawn(Transform::default());
    app.add_systems(
        Update,
        (update_test_health_fill, count_changed_transforms).chain(),
    );

    app.update();
    app.world_mut().resource_mut::<ChangedTransformCount>().0 = 0;
    app.update();
    assert_eq!(
        app.world().resource::<ChangedTransformCount>().0,
        0,
        "retaining the same health fraction must not mark Transform changed"
    );

    app.world_mut().resource_mut::<HealthTestFraction>().0 = 0.5;
    app.update();
    assert_eq!(
        app.world().resource::<ChangedTransformCount>().0,
        1,
        "a changed health fraction must mark the retained Transform changed"
    );
}

#[test]
fn health_fill_is_a_child_with_fraction_scale_and_left_aligned_offset() {
    let unit = 17;
    let parent_id = VisualId {
        kind: VisualKind::HealthBackground,
        unit,
        ordinal: 0,
        generation: 1,
        identity: 0,
    };
    let parent_mesh = Handle::<Mesh>::default();
    let bar_mesh = Handle::<Mesh>::default();
    let background = Handle::<StandardMaterial>::default();
    let health = Handle::<StandardMaterial>::default();
    let input = HealthReconcileTestInput {
        desired: HashMap::from([(
            parent_id,
            (parent_mesh, background.into(), Transform::default()),
        )]),
        fractions: HashMap::from([(unit, 0.25)]),
        bar_mesh: Some(bar_mesh),
        health_material: Some(health),
    };
    let mut app = App::new();
    app.insert_resource(input);
    app.add_systems(Update, reconcile_health_test_visuals);
    app.update();

    let mut fills = app
        .world_mut()
        .query::<(&HealthFill, &Transform, &ChildOf)>();
    let fill_data: Vec<_> = fills
        .iter(app.world())
        .map(|(fill, transform, parent)| (fill.0, *transform, parent.parent()))
        .collect();
    assert_eq!(
        fill_data.len(),
        1,
        "one retained bar fill should be attached to its background"
    );
    let (fill_unit, transform, parent) = &fill_data[0];
    assert_eq!(*fill_unit, unit);
    assert_eq!(
        app.world().get::<VisualId>(*parent),
        Some(&parent_id),
        "health fill must remain a child of the retained health background"
    );
    assert!((transform.scale.x - 0.25).abs() <= f32::EPSILON);
    assert!((transform.translation.x + 0.3375).abs() <= f32::EPSILON);
}

#[test]
fn tooltip_names_actual_unit_and_one_based_owner_slot() {
    assert_eq!(
        tooltip_text("Training Target", 1),
        "Training Target · Slot 2"
    );
    assert_eq!(tooltip_text("Trooper", 0), "Trooper · Slot 1");
}

#[test]
fn shared_visual_mesh_inventory_has_two_ring_topologies_and_one_bar() {
    let mut meshes = Assets::<Mesh>::default();
    let mut assets = VisualAssets::default();
    create_shared_meshes(&mut assets, &mut meshes);
    assert_eq!(
        meshes.len(),
        3,
        "all units share exactly two ring meshes and one bar mesh"
    );
    assert!(assets.ring.is_some());
    assert!(assets.foreign_ring.is_some());
    assert!(assets.bar.is_some());
    let solid = meshes.get(assets.ring.as_ref().unwrap()).unwrap();
    let foreign = meshes.get(assets.foreign_ring.as_ref().unwrap()).unwrap();
    let index_count = |mesh: &Mesh| match mesh.indices().unwrap() {
        Indices::U32(indices) => indices.len(),
        _ => panic!("shared ring mesh indices should be u32"),
    };
    assert_eq!(index_count(solid), 192 * 6);
    assert!(index_count(foreign) < index_count(solid));
    let solid_positions = match solid.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
        VertexAttributeValues::Float32x3(positions) => positions,
        _ => panic!("shared ring mesh positions should be float3"),
    };
    let foreign_positions = match foreign.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
        VertexAttributeValues::Float32x3(positions) => positions,
        _ => panic!("foreign ring mesh positions should be float3"),
    };
    for positions in [solid_positions, foreign_positions] {
        assert!((positions[0][0] - 0.88).abs() <= f32::EPSILON);
        assert!((positions[1][0] - 1.0).abs() <= f32::EPSILON);
    }
    let foreign_indices = match foreign.indices().unwrap() {
        Indices::U32(indices) => indices,
        _ => panic!("foreign ring mesh indices should be u32"),
    };
    let has_segment = |segment: u32| {
        foreign_indices
            .as_chunks::<6>()
            .0
            .iter()
            .any(|triangle_pair| triangle_pair[0] == segment * 2)
    };
    for gap in [0, 48, 96, 144] {
        assert!(!has_segment(gap), "foreign ring gap at segment {gap}");
    }
    for solid in [24, 72, 120, 168] {
        assert!(
            has_segment(solid),
            "foreign ring segment {solid} remains drawn"
        );
    }
}
