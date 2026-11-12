use super::*;
use std::collections::BTreeMap;

fn test_tooltip_style() -> turts_mod_api::TooltipStyle {
    turts_mod_api::TooltipStyle {
        background: "#000000".into(),
        colour: "#ffffff".into(),
        font: None,
        size: 12,
    }
}

#[test]
fn authored_minimap_root_clips_marker_extents_to_its_widget_rect() {
    let mut world = World::new();
    let replica = crate::replica::Replica::new(20);
    let content = crate::content::ClientContent::default();
    let selection = Vec::new();
    let groups = BTreeMap::new();
    let alerts = Vec::new();
    let bindings = BTreeMap::new();
    let input_map = turts_mod_api::InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    let guide = Vec::new();
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot: 0,
        interface: None,
        interface_package: Some("test"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &input_map,
        guide: &guide,
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    let widget = Widget {
        id: "minimap".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(200.0),
            h: turts_mod_api::Dimension::Px(200.0),
        },
        layer: 1,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::MinimapView {
            background: None,
            frame: "#ffffff".into(),
            unit_size: 4.0,
            ping: Some("#ffcc00".into()),
        },
    };

    let root = spawn_widget(
        &mut world,
        None,
        &widget,
        Vec2::splat(400.0),
        &context,
        "test",
        &test_tooltip_style(),
        None,
    );

    assert_eq!(
        world.get::<Node>(root).unwrap().overflow,
        Overflow::clip(),
        "the authored minimap root must clip frame, unit, node, and ping pixels to its rectangle"
    );
}

#[test]
fn hidden_widget_ancestor_hides_child_and_removes_its_world_capture() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::mesh::MeshPlugin,
        bevy::transform::TransformPlugin,
        bevy::camera::visibility::VisibilityPlugin,
    ));
    let replica = crate::replica::Replica::new(20);
    let content = crate::content::ClientContent::default();
    let selection = Vec::new();
    let groups = BTreeMap::new();
    let alerts = Vec::new();
    let bindings = BTreeMap::new();
    let input_map = turts_mod_api::InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    let guide = Vec::new();
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot: 0,
        interface: None,
        interface_package: Some("test"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &input_map,
        guide: &guide,
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    let child = Widget {
        id: "guide.close".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(80.0),
            h: turts_mod_api::Dimension::Px(24.0),
        },
        layer: 1,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Button {
            icon: None,
            label: Some(TextValue::Literal("Close".into())),
            action: ButtonAction::Input("toggle_guide".into()),
            hotkey_label: false,
            font: Default::default(),
            size: Default::default(),
        },
    };
    let parent = Widget {
        id: "guide".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(200.0),
            h: turts_mod_api::Dimension::Px(100.0),
        },
        layer: 0,
        visible: vec![Predicate::GuideOpen],
        tooltip: None,
        widget: Primitive::Box {
            fill: None,
            border: None,
            children: vec![child],
        },
    };
    let parent_id = spawn_widget(
        app.world_mut(),
        None,
        &parent,
        Vec2::new(200.0, 100.0),
        &context,
        "test",
        &test_tooltip_style(),
        None,
    );
    let child_id = app
        .world()
        .get::<Children>(parent_id)
        .and_then(|children| children.iter().next())
        .expect("guide parent has its authored close button");

    app.update();

    assert_eq!(
        app.world().get::<InheritedVisibility>(child_id),
        Some(&InheritedVisibility::HIDDEN),
        "a hidden guide child must not remain visible in the hierarchy"
    );
    assert!(
        !app.world().iter_entities().any(|entity| {
            entity.contains::<UiCaptureRoot>()
                && entity
                    .get::<InheritedVisibility>()
                    .is_some_and(|visibility| visibility.get())
        }),
        "a hidden guide close button must not capture world input"
    );

    let open_context = sources::SourceContext {
        guide_open: true,
        ..context
    };
    refresh_bound_widgets(app.world_mut(), &open_context);
    app.update();
    assert_eq!(
        app.world().get::<InheritedVisibility>(child_id),
        Some(&InheritedVisibility::VISIBLE),
        "opening the guide must make its inherited child visible"
    );
    assert!(
        app.world().iter_entities().any(|entity| {
            entity.contains::<UiCaptureRoot>()
                && entity
                    .get::<InheritedVisibility>()
                    .is_some_and(|visibility| visibility.get())
        }),
        "an open guide button must capture world input"
    );

    refresh_bound_widgets(app.world_mut(), &context);
    app.update();
    assert_eq!(
        app.world().get::<InheritedVisibility>(child_id),
        Some(&InheritedVisibility::HIDDEN),
        "closing the guide must hide the inherited child again"
    );
    assert!(
        !app.world().iter_entities().any(|entity| {
            entity.contains::<UiCaptureRoot>()
                && entity
                    .get::<InheritedVisibility>()
                    .is_some_and(|visibility| visibility.get())
        }),
        "a closed guide button must not capture world input"
    );
}

#[test]
fn icon_only_hotkey_button_still_renders_effective_binding_and_tracks_hover() {
    let mut world = World::new();
    let replica = crate::replica::Replica::new(20);
    let content = crate::content::ClientContent::default();
    let selection = Vec::new();
    let groups = Default::default();
    let alerts = Vec::new();
    let bindings = Default::default();
    let input_map = turts_mod_api::InputMapDef {
        actions: vec![turts_mod_api::ActionDef {
            name: "centre_selection".into(),
            default: vec![turts_mod_api::Binding::Key {
                code: "KeyC".into(),
                mods: Vec::new(),
            }],
        }],
        engine_overrides: Vec::new(),
    };
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot: 0,
        interface: None,
        interface_package: Some("test"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &input_map,
        guide: &[],
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    let widget = Widget {
        id: "card/centre".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(40.0),
            h: turts_mod_api::Dimension::Px(40.0),
        },
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Button {
            icon: None,
            label: None,
            action: ButtonAction::Input("centre_selection".into()),
            hotkey_label: true,
            font: Default::default(),
            size: Default::default(),
        },
    };
    let entity = spawn_widget(
        &mut world,
        None,
        &widget,
        Vec2::new(100.0, 100.0),
        &context,
        "test",
        &test_tooltip_style(),
        None,
    );
    assert_eq!(world.get::<Text>(entity).unwrap().0, " [C]");
    assert_eq!(
        world.get::<bevy::picking::hover::Hovered>(entity),
        Some(&bevy::picking::hover::Hovered(false))
    );
}

#[test]
fn dynamic_list_values_refresh_without_replacing_button_entities() {
    let mut world = World::new();
    let mut replica = crate::replica::Replica::new(20);
    replica.units.insert(
        1,
        crate::replica::UnitReplica {
            content_id: "test:scout".into(),
            owner_slot: 0,
            facing: None,
            samples: std::collections::VecDeque::from([(
                1,
                turts_net_protocol::Pos { x: 1.0, y: 1.0 },
            )]),
        },
    );
    let mut content = crate::content::ClientContent::default();
    let id = turts_mod_api::ContentId::parse("test:scout").unwrap();
    content.catalog.units.insert(
        id.clone(),
        turts_mod_api::ResolvedUnit {
            id,
            name: "Scout".into(),
            footprint: turts_mod_api::Footprint::Circle { radius: 1.0 },
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
        },
    );
    let initial_content = content.clone();
    let selection = [1];
    let groups = Default::default();
    let alerts = Vec::new();
    let bindings = Default::default();
    let input_map = turts_mod_api::InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &initial_content,
        selection: &selection,
        owner_slot: 0,
        interface: None,
        interface_package: Some("test"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &input_map,
        guide: &[],
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    assert_eq!(
        text_value(
            &TextValue::Format(
                "value {0} then {1}".into(),
                vec![Value::Literal("{1}".into()), Value::Literal("done".into())]
            ),
            &context,
            None
        ),
        "value {1} then done",
        "renderer formatting must not rescan inserted placeholder text",
    );
    let row = Widget {
        id: "row".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(80.0),
            h: turts_mod_api::Dimension::Px(20.0),
        },
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Box {
            fill: None,
            border: Some("#abcdef".into()),
            children: vec![Widget {
                id: "row.button".into(),
                at: None,
                size: Size {
                    w: turts_mod_api::Dimension::Px(80.0),
                    h: turts_mod_api::Dimension::Px(20.0),
                },
                layer: 0,
                visible: Vec::new(),
                tooltip: None,
                widget: Primitive::Button {
                    icon: None,
                    label: Some(TextValue::Item(ItemField::Name)),
                    action: ButtonAction::Item(turts_mod_api::ActionField::Action),
                    hotkey_label: false,
                    font: Default::default(),
                    size: Default::default(),
                },
            }],
        },
    };
    let list = Widget {
        id: "units".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(100.0),
            h: turts_mod_api::Dimension::Px(100.0),
        },
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::List {
            source: turts_mod_api::Source::SelectionUnits,
            direction: turts_mod_api::Direction::Vertical,
            gap: 2,
            max_items: 4,
            item: Box::new(row),
        },
    };
    let root = spawn_widget(
        &mut world,
        None,
        &list,
        Vec2::new(100.0, 100.0),
        &context,
        "test",
        &test_tooltip_style(),
        None,
    );
    refresh_bound_widgets(&mut world, &context);
    let row_entity = world
        .iter_entities()
        .find(|entity| {
            entity
                .get::<primitives::AuthoredWidgetId>()
                .is_some_and(|id| id.0.starts_with("units/row["))
        })
        .unwrap()
        .id();
    let row_id = world.get::<Children>(row_entity).unwrap()[0];
    assert_eq!(
        world.get::<Node>(row_entity).unwrap().border,
        UiRect::all(placement::pixel(1.0)),
        "the authored Box row keeps its border after initial placement"
    );
    world.get_mut::<Node>(row_entity).unwrap().overflow = Overflow::scroll();
    assert_eq!(world.get::<Text>(row_id).unwrap().0, "Scout");
    assert_eq!(
        world
            .get::<primitives::WidgetButtonAction>(row_id)
            .unwrap()
            .0,
        primitives::WidgetAction::Select { unit: 1 }
    );

    content
        .catalog
        .units
        .get_mut(&turts_mod_api::ContentId::parse("test:scout").unwrap())
        .unwrap()
        .name = "Scout II".into();
    let updated_context = sources::SourceContext {
        content: &content,
        ..context
    };
    refresh_bound_widgets(&mut world, &updated_context);
    let updated_row = world
        .iter_entities()
        .find(|entity| {
            entity
                .get::<primitives::AuthoredWidgetId>()
                .is_some_and(|id| id.0.starts_with("units/row["))
        })
        .unwrap()
        .id();
    assert_eq!(
        updated_row, row_entity,
        "the real row entity survives changing source values"
    );
    let updated_button = world.get::<Children>(updated_row).unwrap()[0];
    assert_eq!(updated_button, row_id);
    assert_eq!(world.get::<Text>(updated_button).unwrap().0, "Scout II");
    assert_eq!(
        world.get::<Node>(updated_row).unwrap().border,
        UiRect::all(placement::pixel(1.0)),
        "reconciliation updates geometry without replacing the Box border"
    );
    assert_eq!(
        world.get::<Node>(updated_row).unwrap().overflow,
        Overflow::scroll(),
        "reconciliation preserves the row's existing overflow style"
    );
    assert!(
        world
            .get::<Children>(root)
            .is_some_and(|children| children.contains(&updated_row))
    );
}

#[test]
fn text_that_appears_after_an_empty_binding_keeps_authored_typography() {
    let mut world = World::new();
    world.insert_resource(assets::WidgetAssets::default());
    let replica = crate::replica::Replica::new(20);
    let content = crate::content::ClientContent::default();
    let selection = Vec::new();
    let groups = BTreeMap::new();
    let alerts = Vec::new();
    let bindings = BTreeMap::new();
    let input_map = turts_mod_api::InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot: 0,
        interface: None,
        interface_package: Some("test"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &input_map,
        guide: &[],
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    let text = Widget {
        id: "notice".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Fit,
            h: turts_mod_api::Dimension::Fit,
        },
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Text {
            text: TextValue::Bind(Value::LastNotice),
            font: None,
            size: 21,
            colour: "#abcdef".into(),
            align: turts_mod_api::TextAlign::Centre,
            wrap: true,
        },
    };
    let entity = spawn_widget(
        &mut world,
        None,
        &text,
        Vec2::new(300.0, 120.0),
        &context,
        "test",
        &test_tooltip_style(),
        None,
    );
    refresh_bound_widgets(&mut world, &context);
    assert!(world.get::<Text>(entity).is_none());

    let updated_context = sources::SourceContext {
        last_notice: Some("New alert"),
        ..context
    };
    refresh_bound_widgets(&mut world, &updated_context);
    assert_eq!(world.get::<Text>(entity).unwrap().0, "New alert");
    assert_eq!(
        world.get::<TextFont>(entity).unwrap().font_size,
        FontSize::Px(21.0)
    );
    assert_eq!(
        world.get::<TextColor>(entity).unwrap().0,
        primitives::parse_colour("#abcdef").unwrap()
    );
    assert!(world.get::<TextLayout>(entity).is_some());
}

#[test]
fn grid_item_tooltips_inherit_context_and_template_tooltips_override_it() {
    let mut replica = crate::replica::Replica::new(20);
    replica.units.insert(
        1,
        crate::replica::UnitReplica {
            content_id: "test:worker".into(),
            owner_slot: 0,
            facing: None,
            samples: Default::default(),
        },
    );
    let worker_id = turts_mod_api::ContentId::parse("test:worker").unwrap();
    let mut content = crate::content::ClientContent::default();
    content.catalog.units.insert(
        worker_id.clone(),
        turts_mod_api::ResolvedUnit {
            id: worker_id.clone(),
            name: "Worker".into(),
            footprint: turts_mod_api::Footprint::Circle { radius: 1.0 },
            movement: Some(turts_mod_api::MovementDef { speed: 2.0 }),
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
        },
    );
    let move_id = turts_mod_api::ContentId::parse("test:move").unwrap();
    let attack_id = turts_mod_api::ContentId::parse("test:attack").unwrap();
    let order = |id: &str,
                 name: &str,
                 behaviour: turts_mod_api::Behaviour,
                 target: turts_mod_api::TargetKind,
                 hotkey: &str,
                 description: &str,
                 column: u8| turts_mod_api::OrderDef {
        id: id.into(),
        name: name.into(),
        behaviour,
        target,
        fire: None,
        engage: None,
        icon: String::new(),
        hotkey: hotkey.into(),
        cursor: None,
        card: Some(turts_mod_api::CardCell { row: 0, column }),
        units: vec![worker_id.to_string()],
        description: description.into(),
    };
    content.catalog.orders.insert(
        move_id.clone(),
        order(
            "test:move",
            "Move",
            turts_mod_api::Behaviour::Move,
            turts_mod_api::TargetKind::Point,
            "key_move",
            "Move description",
            0,
        ),
    );
    content.catalog.orders.insert(
        attack_id.clone(),
        order(
            "test:attack",
            "Attack",
            turts_mod_api::Behaviour::Attack,
            turts_mod_api::TargetKind::Unit,
            "key_attack",
            "Attack description",
            1,
        ),
    );
    content.input_map.actions = vec![
        turts_mod_api::ActionDef {
            name: "key_move".into(),
            default: vec![turts_mod_api::Binding::Key {
                code: "KeyM".into(),
                mods: Vec::new(),
            }],
        },
        turts_mod_api::ActionDef {
            name: "key_attack".into(),
            default: vec![turts_mod_api::Binding::Key {
                code: "KeyA".into(),
                mods: Vec::new(),
            }],
        },
    ];
    content.order_rules.smart = vec![
        turts_mod_api::SmartOrder {
            target: turts_mod_api::SmartTarget::Ground,
            needs: turts_mod_api::Capability::Movement,
            order: "test:move".into(),
        },
        turts_mod_api::SmartOrder {
            target: turts_mod_api::SmartTarget::ForeignUnit,
            needs: turts_mod_api::Capability::Weapon,
            order: "test:attack".into(),
        },
    ];
    let selection = [1];
    let groups = Default::default();
    let alerts = Vec::new();
    let bindings = Default::default();
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot: 0,
        interface: None,
        interface_package: Some("test"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &content.input_map,
        guide: &[],
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    let inherited = Value::Format(
        "{0} | {1} | {2} | {3}".into(),
        vec![
            Value::Item(ItemField::Name),
            Value::Item(ItemField::Title),
            Value::Item(ItemField::Body),
            Value::Item(ItemField::Reason),
        ],
    );
    let make_grid = |id: &str, template_tooltip: Option<Value>| Widget {
        id: id.into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(130.0),
            h: turts_mod_api::Dimension::Px(36.0),
        },
        layer: 0,
        visible: Vec::new(),
        tooltip: Some(inherited.clone()),
        widget: Primitive::Grid {
            source: turts_mod_api::Source::CommandCard,
            columns: 2,
            rows: 1,
            cell: (64, 32),
            gap: 2,
            item: Box::new(Widget {
                id: "order".into(),
                at: None,
                size: Size {
                    w: turts_mod_api::Dimension::Fill,
                    h: turts_mod_api::Dimension::Fill,
                },
                layer: 0,
                visible: Vec::new(),
                tooltip: template_tooltip,
                widget: Primitive::Button {
                    icon: None,
                    label: Some(TextValue::Item(ItemField::Name)),
                    action: ButtonAction::Item(turts_mod_api::ActionField::Action),
                    hotkey_label: false,
                    font: Default::default(),
                    size: Default::default(),
                },
            }),
        },
    };

    let mut world = World::new();
    let grid = spawn_widget(
        &mut world,
        None,
        &make_grid("card", None),
        Vec2::new(200.0, 100.0),
        &context,
        "test",
        &test_tooltip_style(),
        None,
    );
    refresh_bound_widgets(&mut world, &context);
    assert!(world.get::<primitives::WidgetTooltip>(grid).is_none());
    let mut rows = world
        .iter_entities()
        .filter_map(|entity| {
            let action = entity.get::<primitives::WidgetButtonAction>()?;
            Some((action.0.clone(), entity.id()))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|(action, _)| format!("{action:?}"));
    assert_eq!(rows.len(), 2);
    let move_button = rows
        .iter()
        .find(|(action, _)| {
            action
                == &primitives::WidgetAction::Order {
                    id: "test:move".into(),
                }
        })
        .unwrap()
        .1;
    let attack_button = rows
        .iter()
        .find(|(action, _)| {
            action
                == &primitives::WidgetAction::Order {
                    id: "test:attack".into(),
                }
        })
        .unwrap()
        .1;
    assert_eq!(
        world
            .get::<primitives::WidgetTooltip>(move_button)
            .unwrap()
            .0,
        "Move | M | Move description | "
    );
    assert_eq!(
        world
            .get::<primitives::WidgetTooltip>(attack_button)
            .unwrap()
            .0,
        "Attack | A | Attack description | Requires a selected unit with this capability"
    );
    assert_eq!(
        world.get::<primitives::WidgetEnabled>(attack_button),
        Some(&primitives::WidgetEnabled(false))
    );

    let override_grid = spawn_widget(
        &mut world,
        None,
        &make_grid("override_card", Some(Value::Item(ItemField::Name))),
        Vec2::new(200.0, 100.0),
        &context,
        "test",
        &test_tooltip_style(),
        None,
    );
    refresh_bound_widgets(&mut world, &context);
    let override_button = world
        .iter_entities()
        .find(|entity| {
            entity.contains::<Button>()
                && entity
                    .get::<primitives::AuthoredWidgetId>()
                    .is_some_and(|id| {
                        id.0.starts_with("override_card/") && id.0.contains("test:move")
                    })
        })
        .unwrap()
        .id();
    assert_eq!(
        world
            .get::<primitives::WidgetTooltip>(override_button)
            .unwrap()
            .0,
        "Move"
    );
    assert!(
        world
            .get::<primitives::WidgetTooltip>(override_grid)
            .is_none()
    );
}

#[test]
fn disabled_authored_button_cannot_dispatch_an_action() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, WidgetsPlugin))
        .init_resource::<CoreInputQueue>()
        .init_resource::<crate::ingest::ReplicaState>()
        .init_resource::<crate::commands::PendingCommands>();
    let replica = crate::replica::Replica::new(20);
    let content = crate::content::ClientContent::default();
    let selection = Vec::new();
    let groups = Default::default();
    let alerts = Vec::new();
    let bindings = Default::default();
    let input_map = turts_mod_api::InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot: 0,
        interface: None,
        interface_package: Some("test"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &input_map,
        guide: &[],
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    let item = sources::ItemData {
        fields: vec![(ItemField::Enabled, serde_json::json!(false))],
    };
    let button = Widget {
        id: "test.disabled".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(80.0),
            h: turts_mod_api::Dimension::Px(24.0),
        },
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Button {
            icon: None,
            label: Some(TextValue::Literal("Disabled".into())),
            action: ButtonAction::Input("toggle_guide".into()),
            hotkey_label: false,
            font: Default::default(),
            size: Default::default(),
        },
    };
    let entity = spawn_widget(
        app.world_mut(),
        None,
        &button,
        Vec2::new(100.0, 100.0),
        &context,
        "test",
        &test_tooltip_style(),
        Some(&item),
    );
    refresh_bound_widgets(app.world_mut(), &context);
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(entity)
            .is_some()
    );
    app.world_mut().trigger(Activate { entity });
    assert!(app.world().resource::<CoreInputQueue>().0.is_empty());
    assert!(!app.world().resource::<guide::GuideOpen>().is_open);
}

#[test]
fn fit_text_and_image_use_real_asset_components_with_bevy_auto_sizing() {
    let mut world = World::new();
    let mut image_store = Assets::<Image>::default();
    let image_handle = image_store.add(Image::default());
    let mut font_store = Assets::<Font>::default();
    let font_handle = font_store.add(Font::from_bytes(
        include_bytes!("../../../../mods/muster/assets/fonts/Roboto-Regular.ttf").to_vec(),
    ));
    let mut widget_assets = assets::WidgetAssets::default();
    widget_assets
        .images
        .insert("test:icon".into(), image_handle.clone());
    widget_assets
        .fonts
        .insert("test:font".into(), font_handle.clone());
    widget_assets
        .nine_slices
        .insert("test:icon".into(), (2, 3, 4, 5));
    world.insert_resource(widget_assets);
    world.insert_resource(image_store);

    let replica = crate::replica::Replica::new(20);
    let content = crate::content::ClientContent::default();
    let selection = Vec::new();
    let groups = Default::default();
    let alerts = Vec::new();
    let bindings = Default::default();
    let input_map = turts_mod_api::InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot: 0,
        interface: None,
        interface_package: Some("test"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &input_map,
        guide: &[],
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    let fit = Size {
        w: turts_mod_api::Dimension::Fit,
        h: turts_mod_api::Dimension::Fit,
    };
    let text = Widget {
        id: "fit.text".into(),
        at: None,
        size: fit,
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Text {
            text: TextValue::Literal("Measured by the installed font".into()),
            font: Some("font".into()),
            size: 19,
            colour: "#ffffff".into(),
            align: turts_mod_api::TextAlign::Left,
            wrap: false,
        },
    };
    let image = Widget {
        id: "fit.image".into(),
        at: None,
        size: fit,
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Image {
            image: ButtonIcon::Asset("icon".into()),
            nine_slice: None,
            tint: None,
        },
    };
    let style = test_tooltip_style();
    let text_entity = spawn_widget(
        &mut world,
        None,
        &text,
        Vec2::new(500.0, 300.0),
        &context,
        "test",
        &style,
        None,
    );
    let image_entity = spawn_widget(
        &mut world,
        None,
        &image,
        Vec2::new(500.0, 300.0),
        &context,
        "test",
        &style,
        None,
    );
    for entity in [text_entity, image_entity] {
        let node = world.get::<Node>(entity).unwrap();
        assert_eq!(node.width, Val::Auto);
        assert_eq!(node.height, Val::Auto);
    }
    assert_eq!(
        world.get::<TextFont>(text_entity).unwrap().font,
        FontSource::Handle(font_handle)
    );
    assert_eq!(
        world.get::<ImageNode>(image_entity).unwrap().image,
        image_handle
    );

    let sliced = Widget {
        id: "fit.sliced-image".into(),
        at: None,
        size: fit,
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Image {
            image: ButtonIcon::Asset("icon".into()),
            nine_slice: Some((1, 1, 1, 1)),
            tint: None,
        },
    };
    let sliced_entity = spawn_widget(
        &mut world,
        None,
        &sliced,
        Vec2::new(500.0, 300.0),
        &context,
        "test",
        &style,
        None,
    );
    assert!(matches!(
        world.get::<ImageNode>(sliced_entity).unwrap().image_mode,
        NodeImageMode::Sliced(_)
    ));
    update_fit_layout(&mut world);
    assert_eq!(
        world.get::<Node>(sliced_entity).unwrap().width,
        placement::pixel(1.0),
        "Fit for Sliced images uses actual retained image dimensions"
    );

    let bar = Widget {
        id: "fit.bar".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(120.0),
            h: turts_mod_api::Dimension::Px(12.0),
        },
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Bar {
            value: Value::Literal("3".into()),
            max: Value::Literal("4".into()),
            fill: Fill::Image {
                id: "icon".into(),
                nine_slice: None,
            },
            background: Fill::Image {
                id: "icon".into(),
                nine_slice: None,
            },
            direction: turts_mod_api::Direction::Horizontal,
        },
    };
    let bar_entity = spawn_widget(
        &mut world,
        None,
        &bar,
        Vec2::new(500.0, 300.0),
        &context,
        "test",
        &style,
        None,
    );
    let background = world.get::<ImageNode>(bar_entity).unwrap();
    assert_eq!(background.image, image_handle);
    assert!(matches!(background.image_mode, NodeImageMode::Sliced(_)));
    let fill = world
        .get::<Children>(bar_entity)
        .unwrap()
        .iter()
        .find_map(|child| world.get::<ImageNode>(child))
        .expect("Bar image fill keeps its own rendered image");
    assert_eq!(fill.image, image_handle);
    assert!(matches!(fill.image_mode, NodeImageMode::Sliced(_)));

    let measured_child = Widget {
        id: "fit.measured-child".into(),
        at: Some(turts_mod_api::At {
            anchor: Anchor::BottomRight,
            offset: (-4.0, -3.0),
            pivot: Some(Anchor::BottomRight),
        }),
        size: fit,
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Text {
            text: TextValue::Literal("measured child".into()),
            font: Some("font".into()),
            size: 19,
            colour: "#ffffff".into(),
            align: turts_mod_api::TextAlign::Left,
            wrap: false,
        },
    };
    let containers = [
        Primitive::Box {
            fill: None,
            border: None,
            children: vec![measured_child.clone()],
        },
        Primitive::List {
            source: turts_mod_api::Source::Balances,
            direction: turts_mod_api::Direction::Vertical,
            gap: 0,
            max_items: 1,
            item: Box::new(measured_child.clone()),
        },
        Primitive::Grid {
            source: turts_mod_api::Source::Balances,
            columns: 1,
            rows: 1,
            cell: (1, 1),
            gap: 0,
            item: Box::new(measured_child.clone()),
        },
    ];
    for (index, primitive) in containers.into_iter().enumerate() {
        let container = Widget {
            id: format!("fit.container.{index}"),
            at: None,
            size: fit,
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: primitive.clone(),
        };
        let parent = spawn_widget(
            &mut world,
            None,
            &container,
            Vec2::new(500.0, 300.0),
            &context,
            "test",
            &style,
            None,
        );
        if !matches!(primitive, Primitive::Box { .. }) {
            let child = spawn_widget(
                &mut world,
                Some(parent),
                &measured_child,
                Vec2::new(500.0, 300.0),
                &context,
                "test",
                &style,
                None,
            );
            world.entity_mut(child).insert(ComputedNode {
                size: Vec2::new(80.0, 40.0),
                inverse_scale_factor: 0.5,
                ..default()
            });
        } else {
            let child = world.get::<Children>(parent).unwrap()[0];
            world.entity_mut(child).insert(ComputedNode {
                size: Vec2::new(80.0, 40.0),
                inverse_scale_factor: 0.5,
                ..default()
            });
        }
        let child = world.get::<Children>(parent).unwrap()[0];
        if !matches!(primitive, Primitive::Box { .. })
            && let Some(mut node) = world.get_mut::<Node>(child)
        {
            node.border = UiRect::all(placement::pixel(3.0));
            node.overflow = Overflow::scroll();
        }
        update_fit_layout(&mut world);
        let node = world.get::<Node>(parent).unwrap();
        assert_eq!(
            node.width,
            placement::pixel(44.0),
            "Fit {index} width uses the measured child and bottom-right inset"
        );
        assert_eq!(node.height, placement::pixel(23.0));
        let child = world.get::<Children>(parent).unwrap()[0];
        assert_eq!(
            world.get::<Node>(child).unwrap().left,
            placement::pixel(40.0),
            "the anchored child is repositioned relative to the resolved Fit width"
        );
        assert_eq!(
            world.get::<Node>(child).unwrap().top,
            placement::pixel(20.0)
        );
        if !matches!(primitive, Primitive::Box { .. }) {
            let node = world.get::<Node>(child).unwrap();
            assert_eq!(node.border, UiRect::all(placement::pixel(3.0)));
            assert_eq!(node.overflow, Overflow::scroll());
        }
        world.entity_mut(child).insert(ComputedNode {
            size: Vec2::new(120.0, 60.0),
            inverse_scale_factor: 0.5,
            ..default()
        });
        update_fit_layout(&mut world);
        assert_eq!(
            world.get::<Node>(parent).unwrap().width,
            placement::pixel(64.0),
            "Fit containers follow later measured content changes"
        );
        assert_eq!(
            world.get::<Node>(child).unwrap().left,
            placement::pixel(60.0),
            "the anchored child follows the updated natural container size"
        );
    }
}

#[test]
fn normal_hovered_pressed_and_disabled_buttons_receive_distinct_states() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, update_widget_button_states);
    let normal = app
        .world_mut()
        .spawn((
            Button,
            primitives::WidgetEnabled(true),
            BackgroundColor(Color::NONE),
        ))
        .id();
    let hovered = app
        .world_mut()
        .spawn((
            Button,
            primitives::WidgetEnabled(true),
            bevy::picking::hover::Hovered(true),
            BackgroundColor(Color::NONE),
        ))
        .id();
    let pressed = app
        .world_mut()
        .spawn((
            Button,
            primitives::WidgetEnabled(true),
            bevy::ui::Pressed,
            BackgroundColor(Color::NONE),
        ))
        .id();
    let disabled = app
        .world_mut()
        .spawn((
            Button,
            primitives::WidgetEnabled(false),
            BackgroundColor(Color::NONE),
        ))
        .id();
    app.update();
    let colour = |entity| app.world().get::<BackgroundColor>(entity).unwrap().0;
    assert_eq!(colour(normal), Color::srgb(0.13, 0.26, 0.30));
    assert_eq!(colour(hovered), Color::srgb(0.18, 0.40, 0.44));
    assert_eq!(colour(pressed), Color::srgb(0.10, 0.30, 0.36));
    assert_eq!(colour(disabled), Color::srgb(0.20, 0.20, 0.20));
}

#[test]
fn bar_refresh_tracks_bound_progress_without_replacing_fill_entity() {
    let mut world = World::new();
    let replica = crate::replica::Replica::new(20);
    let content = crate::content::ClientContent::default();
    let selection = Vec::new();
    let groups = Default::default();
    let alerts = Vec::new();
    let bindings = Default::default();
    let input_map = turts_mod_api::InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot: 0,
        interface: None,
        interface_package: Some("test"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &input_map,
        guide: &[],
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    let item = sources::ItemData {
        fields: vec![
            (ItemField::Progress, serde_json::json!(20.0)),
            (ItemField::Count, serde_json::json!(100.0)),
        ],
    };
    let bar = Widget {
        id: "progress".into(),
        at: None,
        size: Size {
            w: turts_mod_api::Dimension::Px(100.0),
            h: turts_mod_api::Dimension::Px(8.0),
        },
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::Bar {
            value: Value::Item(ItemField::Progress),
            max: Value::Item(ItemField::Count),
            fill: Fill::Colour("#00ff00".into()),
            background: Fill::Colour("#000000".into()),
            direction: turts_mod_api::Direction::Horizontal,
        },
    };
    let root = spawn_widget(
        &mut world,
        None,
        &bar,
        Vec2::new(100.0, 8.0),
        &context,
        "test",
        &test_tooltip_style(),
        Some(&item),
    );
    let fill = world
        .get::<Children>(root)
        .unwrap()
        .first()
        .copied()
        .unwrap();
    let action_identity = fill;
    refresh_bound_widgets(&mut world, &context);
    assert_eq!(
        world.get::<Node>(fill).unwrap().width,
        placement::percent(20.0)
    );
    let updated = sources::ItemData {
        fields: vec![
            (ItemField::Progress, serde_json::json!(80.0)),
            (ItemField::Count, serde_json::json!(100.0)),
        ],
    };
    world.get_mut::<WidgetBinding>(root).unwrap().item = Some(updated);
    refresh_bound_widgets(&mut world, &context);
    assert_eq!(
        world.get::<Children>(root).unwrap().first().copied(),
        Some(action_identity)
    );
    assert_eq!(
        world.get::<Node>(fill).unwrap().width,
        placement::percent(80.0)
    );
}

#[test]
fn command_card_grid_requires_authored_cells_but_generic_grid_can_flow() {
    let command_card = turts_mod_api::Source::CommandCard;
    let located = sources::ItemData {
        fields: vec![(
            ItemField::CardCell,
            serde_json::json!({ "row": 1, "column": 2 }),
        )],
    };
    let unplaced = sources::ItemData::default();
    assert!(grid_item_included(&command_card, true, &located));
    assert!(
        !grid_item_included(&command_card, true, &unplaced),
        "an unplaced action does not claim a reserved card cell"
    );
    assert!(
        grid_item_included(&command_card, false, &unplaced),
        "the same source can still drive an authored list"
    );
    assert!(
        grid_item_included(&turts_mod_api::Source::Balances, true, &unplaced),
        "generic grids retain sequential flow"
    );
}

fn forge_interface() -> turts_mod_api::InterfaceDef {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge");
    let manifest: turts_mod_api::Manifest =
        ron::from_str(&std::fs::read_to_string(dir.join("manifest.ron")).unwrap()).unwrap();
    ron::from_str(&std::fs::read_to_string(dir.join(&manifest.exports.interface[0])).unwrap())
        .unwrap()
}

fn find_item_template<'a>(widgets: &'a [Widget], id: &str) -> Option<&'a Widget> {
    widgets.iter().find_map(|widget| match &widget.widget {
        Primitive::List { item, .. } | Primitive::Grid { item, .. } if widget.id == id => {
            Some(item.as_ref())
        }
        Primitive::Box { children, .. } => find_item_template(children, id),
        _ => None,
    })
}

/// PT1 step 5: button labels with "×" and "—" use the interface font, not the Bevy default
/// font. The default font has only U+0020 to U+007E, so these glyphs show as boxes.
#[test]
fn button_labels_use_the_interface_tooltip_font() {
    let interface = forge_interface();
    assert_eq!(interface.tooltip_style.font.as_deref(), Some("forge_ui"));
    let mut world = World::new();
    let mut font_store = Assets::<Font>::default();
    let forge_font = font_store.add(Font::from_bytes(
        include_bytes!("../../../../mods/forge/assets/fonts/Roboto-Regular.ttf").to_vec(),
    ));
    let mut widget_assets = assets::WidgetAssets::default();
    widget_assets
        .fonts
        .insert("forge:forge_ui".into(), forge_font.clone());
    world.insert_resource(widget_assets);

    let replica = crate::replica::Replica::new(20);
    let content = crate::content::ClientContent::default();
    let selection = Vec::new();
    let groups = Default::default();
    let alerts = Vec::new();
    let bindings = Default::default();
    let input_map = turts_mod_api::InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    let context = sources::SourceContext {
        binding_labels: None,
        replica: &replica,
        content: &content,
        selection: &selection,
        owner_slot: 0,
        interface: Some(&interface),
        interface_package: Some("forge"),
        control_groups: &groups,
        alerts: &alerts,
        bindings: &bindings,
        input_map: &input_map,
        guide: &[],
        guide_open: false,
        last_notice: None,
        core: sources::CoreFeed::empty(),
    };
    let cases = [
        (
            "selection_units",
            sources::ItemData {
                fields: vec![
                    (ItemField::Name, serde_json::json!("Proving Lab")),
                    (ItemField::Count, serde_json::json!(1)),
                    (ItemField::Health, serde_json::json!(300)),
                ],
            },
            "Proving Lab ×1 — 300 HP",
        ),
        (
            "research_entries",
            sources::ItemData {
                fields: vec![
                    (ItemField::Name, serde_json::json!("Hardened Shells")),
                    (
                        ItemField::Reason,
                        serde_json::json!("Needs 10 Shell (0/10)"),
                    ),
                ],
            },
            "Hardened Shells — Needs 10 Shell (0/10)",
        ),
    ];
    for (list, item, expected) in cases {
        let template = find_item_template(&interface.root, list).unwrap();
        let entity = spawn_widget(
            &mut world,
            None,
            template,
            Vec2::new(256.0, 40.0),
            &context,
            "forge",
            &interface.tooltip_style,
            Some(&item),
        );
        assert_eq!(world.get::<Text>(entity).unwrap().0, expected);
        let font = world.get::<TextFont>(entity).unwrap();
        assert_eq!(
            font.font,
            FontSource::Handle(forge_font.clone()),
            "{list}: the label must use the interface font"
        );
        assert_eq!(font.font_size, FontSize::Px(14.0));

        // A label that the refresh adds later takes the same font.
        world.entity_mut(entity).remove::<(Text, TextFont)>();
        refresh_bound_widgets(&mut world, &context);
        assert_eq!(world.get::<Text>(entity).unwrap().0, expected);
        assert_eq!(
            world.get::<TextFont>(entity).unwrap().font,
            FontSource::Handle(forge_font.clone()),
            "{list}: the refreshed label must use the interface font"
        );
    }

    // With no interface font, the label keeps the default font.
    let mut plain = interface.tooltip_style.clone();
    plain.font = None;
    let template = find_item_template(&interface.root, "research_entries").unwrap();
    let entity = spawn_widget(
        &mut world,
        None,
        template,
        Vec2::new(256.0, 40.0),
        &context,
        "forge",
        &plain,
        Some(&sources::ItemData {
            fields: vec![(ItemField::Name, serde_json::json!("Plain"))],
        }),
    );
    assert_eq!(
        world.get::<TextFont>(entity).unwrap().font,
        FontSource::Handle(Handle::default())
    );
}

/// The contract of the 0.2.4 source stubs of L0-CORE (`widgets/sources/*.rs`, plan 5.3): until the
/// owner task builds a variant, a list source has no rows, a value is null, a predicate does not
/// hold. The context has one selected unit, so the primary field arms reach their stub. Each owner
/// task replaces its own test here with its own.
mod stub_contract {
    use super::*;
    use crate::replica::UnitReplica;
    use std::collections::VecDeque;
    use turts_mod_api::{OutcomeKind, OutcomeReasonKind, PrimaryField, Source, StatField};

    fn with_context(check: impl FnOnce(&sources::SourceContext<'_>)) {
        let mut replica = crate::replica::Replica::new(20);
        replica.units.insert(
            1,
            UnitReplica {
                content_id: "test:unit".into(),
                owner_slot: 0,
                facing: None,
                samples: VecDeque::from([(1, turts_net_protocol::Pos { x: 0.0, y: 0.0 })]),
            },
        );
        let content = crate::content::ClientContent::default();
        let selection = vec![1];
        let groups = BTreeMap::new();
        let alerts = Vec::new();
        let bindings = BTreeMap::new();
        let input_map = turts_mod_api::InputMapDef {
            actions: Vec::new(),
            engine_overrides: Vec::new(),
        };
        let guide = Vec::new();
        check(&sources::SourceContext {
            binding_labels: None,
            replica: &replica,
            content: &content,
            selection: &selection,
            owner_slot: 0,
            interface: None,
            interface_package: Some("test"),
            control_groups: &groups,
            alerts: &alerts,
            bindings: &bindings,
            input_map: &input_map,
            guide: &guide,
            guide_open: false,
            last_notice: None,
            core: sources::CoreFeed::empty(),
        });
    }

    fn assert_null(value: Value, context: &sources::SourceContext<'_>) {
        assert_eq!(
            sources::read_value(&value, *context),
            serde_json::Value::Null,
            "{value:?} must read null until its owner builds it"
        );
    }

    fn assert_no_rows(source: Source, context: &sources::SourceContext<'_>) {
        let data = sources::read_source(&source, *context);
        assert!(
            data.scalar.is_none() && data.items.is_empty(),
            "{source:?} must have no rows until its owner builds it"
        );
    }

    fn assert_does_not_hold(predicate: Predicate, context: &sources::SourceContext<'_>) {
        assert!(
            !predicate_matches(&predicate, context),
            "{predicate:?} must not hold until its owner builds it"
        );
        assert!(
            predicate_matches(&Predicate::Not(Box::new(predicate)), context),
            "Not of a predicate that does not hold must hold"
        );
    }

    #[test]
    fn ui_hud_stubs_have_no_rows_no_value_and_no_predicate() {
        with_context(|context| {
            assert_no_rows(Source::ConstructionQueue, context);
            assert_no_rows(Source::Sites, context);
            assert_no_rows(Source::Notices, context);
            assert_no_rows(Source::SelectionTypes, context);
            assert_null(
                Value::Supply("test:supply".into(), turts_mod_api::SupplySide::Provided),
                context,
            );
            assert_null(
                Value::Supply("test:supply".into(), turts_mod_api::SupplySide::Consumed),
                context,
            );
            assert_null(Value::ConstructionQueue, context);
            assert_null(Value::Sites, context);
            assert_null(Value::Notices, context);
            assert_null(Value::SelectionTypes, context);
            assert_does_not_hold(Predicate::SupplyShort("test:supply".into()), context);
            assert_does_not_hold(Predicate::ItemHas(ItemField::Role), context);
        });
    }

    #[test]
    fn ui_result_stubs_have_no_value_and_no_predicate() {
        with_context(|context| {
            assert_null(Value::Outcome, context);
            assert_null(Value::OutcomeReason, context);
            assert_null(Value::MatchClock, context);
            assert_does_not_hold(Predicate::MatchOver, context);
            assert_does_not_hold(Predicate::OutcomeIs(OutcomeKind::Victory), context);
            assert_does_not_hold(
                Predicate::OutcomeReasonIs(OutcomeReasonKind::KeyUnitsLost),
                context,
            );
        });
    }

    #[test]
    fn ui2_result_stub_has_no_value() {
        with_context(|context| {
            assert_null(Value::MatchStats(StatField::UnitsTrained), context);
        });
    }

    #[test]
    fn ui2_conn_stub_has_no_value() {
        with_context(|context| assert_null(Value::Connection, context));
    }

    #[test]
    fn ui_hint_stubs_have_no_value_and_no_predicate() {
        with_context(|context| {
            assert_null(Value::Hint, context);
            assert_does_not_hold(Predicate::HintShown, context);
        });
    }

    #[test]
    fn ui_place_stub_does_not_hold() {
        with_context(|context| assert_does_not_hold(Predicate::Placing, context));
    }

    #[test]
    fn ui2_hud_stubs_have_no_primary_value_and_no_predicate() {
        with_context(|context| {
            for field in [
                PrimaryField::Activity,
                PrimaryField::Cargo,
                PrimaryField::Armour,
            ] {
                assert_null(Value::SelectionPrimary(field), context);
                assert_does_not_hold(Predicate::PrimaryHasValue(field), context);
            }
        });
    }

    #[test]
    fn new_widget_actions_activate_nothing_until_their_owner_builds_them() {
        use turts_interaction::InputEvent;
        for action in [
            primitives::WidgetAction::Place {
                constructor: 1,
                content: "test:building".into(),
            },
            primitives::WidgetAction::FocusAlert {
                x: 3.0,
                y: 4.0,
                unit: Some(1),
            },
            primitives::WidgetAction::FocusAlert {
                x: 3.0,
                y: 4.0,
                unit: None,
            },
            primitives::WidgetAction::Input {
                name: "leave_match".into(),
            },
        ] {
            let mut app = App::new();
            app.init_resource::<CoreInputQueue>();
            app.init_resource::<ReplicaState>();
            app.add_observer(activate_widget);
            let button = app
                .world_mut()
                .spawn((
                    Button,
                    primitives::WidgetButtonAction(action.clone()),
                    primitives::WidgetEnabled(true),
                ))
                .id();
            app.world_mut().trigger(Activate { entity: button });
            let queued: &Vec<InputEvent> = &app.world().resource::<CoreInputQueue>().0;
            assert!(
                queued.is_empty(),
                "{action:?} must queue no input until its owner builds it: {queued:?}"
            );
        }
    }

    /// Control: the guard of `leave_match` leaves every other engine action to the core.
    #[test]
    fn other_input_actions_still_reach_the_core() {
        let mut app = App::new();
        app.init_resource::<CoreInputQueue>();
        app.init_resource::<ReplicaState>();
        app.add_observer(activate_widget);
        let button = app
            .world_mut()
            .spawn((
                Button,
                primitives::WidgetButtonAction(primitives::WidgetAction::Input {
                    name: "zoom_in".into(),
                }),
                primitives::WidgetEnabled(true),
            ))
            .id();
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(
            app.world().resource::<CoreInputQueue>().0,
            vec![turts_interaction::InputEvent::UiAction {
                name: "zoom_in".into()
            }]
        );
    }
}
