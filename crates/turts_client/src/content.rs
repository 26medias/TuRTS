use std::collections::HashMap;
use turts_mod_api::{
    CameraDef, ContentCatalog, ContentId, InputMapDef, ModError, OrderRulesDef, Shape,
};
use turts_mod_runtime::{archive::PackageArchive, visuals::visuals_by_content};

#[derive(Clone, Debug, PartialEq)]
pub struct ClientContent {
    pub catalog: ContentCatalog,
    pub shapes: HashMap<String, Shape>,
    pub model_pick_shapes: HashMap<String, Shape>,
    pub input_map: InputMapDef,
    pub order_rules: OrderRulesDef,
    pub selection_priority: Vec<String>,
    pub camera: Option<CameraDef>,
    pub terrain: Option<std::sync::Arc<crate::terrain::store::Layout>>,
}
impl Default for ClientContent {
    fn default() -> Self {
        Self {
            catalog: ContentCatalog {
                utilities: Default::default(),
                terrain_materials: Default::default(),
                terrain_movement_classes: Default::default(),
                terrain_tools: Default::default(),
                bots: Default::default(),
                combat: None,
                resources: Default::default(),
                units: Default::default(),
                nodes: Default::default(),
                production: Default::default(),
                recipes: Default::default(),
                technologies: Default::default(),
                orders: Default::default(),
                order_queue_depths: std::collections::BTreeMap::new(),
                source_files: Default::default(),
            },
            shapes: HashMap::new(),
            model_pick_shapes: HashMap::new(),
            input_map: InputMapDef {
                actions: Vec::new(),
                engine_overrides: Vec::new(),
            },
            order_rules: OrderRulesDef {
                queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
                smart: Vec::new(),
                rally: None,
            },
            selection_priority: Vec::new(),
            camera: None,
            terrain: None,
        }
    }
}
impl ClientContent {
    pub fn from_archives(archives: &[PackageArchive]) -> Result<Self, ModError> {
        Self::from_archives_for_root(archives, None)
    }

    pub fn from_archives_for_root(
        archives: &[PackageArchive],
        root: Option<&str>,
    ) -> Result<Self, ModError> {
        let shapes = visuals_by_content(archives)?;
        let mut catalog = ContentCatalog {
            utilities: Default::default(),
            terrain_materials: Default::default(),
            terrain_movement_classes: Default::default(),
            terrain_tools: Default::default(),
            bots: Default::default(),
            combat: None,
            resources: Default::default(),
            units: Default::default(),
            nodes: Default::default(),
            production: Default::default(),
            recipes: Default::default(),
            technologies: Default::default(),
            orders: Default::default(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: Default::default(),
        };
        let mut input_map = InputMapDef {
            actions: Vec::new(),
            engine_overrides: Vec::new(),
        };
        let mut order_rules = OrderRulesDef {
            queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
            smart: Vec::new(),
            rally: None,
        };
        let mut selection_priority = Vec::new();
        let mut camera = None;
        for archive in archives {
            let part = archive.content_for_catalog()?;
            if let Some(table) = part.combat
                && catalog.combat.replace(table).is_some()
            {
                return Err(ModError::new(
                    &archive.id().0,
                    "exports.combat",
                    "a match can have only one damage table",
                ));
            }
            for (id, value) in part.terrain_materials {
                if catalog
                    .terrain_materials
                    .insert(id.clone(), value)
                    .is_some()
                {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, value) in part.terrain_movement_classes {
                if catalog
                    .terrain_movement_classes
                    .insert(id.clone(), value)
                    .is_some()
                {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, value) in part.terrain_tools {
                if catalog.terrain_tools.insert(id.clone(), value).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, value) in part.resources {
                if catalog.resources.insert(id.clone(), value).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, value) in part.units {
                if catalog.units.insert(id.clone(), value).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, value) in part.nodes {
                if catalog.nodes.insert(id.clone(), value).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, value) in part.production {
                if catalog.production.insert(id.clone(), value).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, value) in part.recipes {
                if catalog.recipes.insert(id.clone(), value).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, value) in part.technologies {
                if catalog.technologies.insert(id.clone(), value).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, value) in part.orders {
                if catalog.orders.insert(id.clone(), value).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (id, file) in part.source_files {
                if catalog.source_files.insert(id.clone(), file).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
            for (_, map) in archive.input_maps()? {
                input_map.actions.extend(map.actions);
                input_map.engine_overrides.extend(map.engine_overrides);
            }
            for (_, rules) in archive.order_rules()? {
                order_rules.queue_depth = rules.queue_depth;
                order_rules.smart.extend(rules.smart);
            }
            for (_, interface) in archive.interfaces()? {
                if camera.is_none() {
                    camera = Some(interface.camera.clone());
                }
                if selection_priority.is_empty() && !interface.selection_priority.is_empty() {
                    selection_priority = interface.selection_priority;
                }
            }
        }
        // All libraries are available before profile references are checked. Importing
        // them loads definitions only; the server separately selects and installs bots.
        for archive in archives {
            let profiles = archive.bots()?;
            let bots =
                turts_mod_runtime::bots::validate_bots(&archive.id(), profiles.clone(), &catalog)?;
            for (file, bot) in profiles {
                catalog
                    .source_files
                    .insert(ContentId::new(&archive.id(), &bot.id), file);
            }
            for (id, bot) in bots {
                if catalog.bots.insert(id.clone(), bot).is_some() {
                    return Err(ModError::new(&id.package.0, "id", "duplicate bot profile"));
                }
            }
        }
        // Resolve library references only after every public package has contributed.
        let mut selected = None;
        for archive in archives {
            if root.is_some_and(|id| archive.manifest.id != id) {
                continue;
            }
            let maps = archive.maps()?;
            if !maps.is_empty() {
                selected = Some((archive, maps));
                break;
            }
        }
        let origin = selected
            .as_ref()
            .map(|(archive, _)| archive.id())
            .unwrap_or_else(|| turts_mod_api::PackageId("client".into()));
        turts_mod_runtime::terrain::validate_terrain_catalog(&origin, &catalog)?;
        for archive in archives {
            for (file, interface) in archive.interfaces()? {
                turts_mod_runtime::terrain::validate_terrain_tool_sources(
                    &archive.id(),
                    &file,
                    &interface,
                    &catalog,
                )?;
            }
        }
        let terrain = if let Some((archive, maps)) = selected {
            let (file, map) = &maps[0];
            turts_mod_runtime::terrain::validate_terrain_map(&archive.id(), file, map, &catalog)?;
            map.terrain
                .as_ref()
                .map(|definition| {
                    crate::terrain::store::Layout::new(
                        ContentId::new(&archive.id(), &map.id),
                        definition,
                    )
                    .map(std::sync::Arc::new)
                    .map_err(|error| {
                        ModError::new(
                            &archive.id().0,
                            file,
                            format!("terrain client layout: {error:?}"),
                        )
                    })
                })
                .transpose()?
        } else {
            None
        };
        Ok(Self {
            catalog,
            shapes,
            model_pick_shapes: HashMap::new(),
            input_map,
            order_rules,
            selection_priority,
            camera,
            terrain,
        })
    }
}
