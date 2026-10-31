use std::collections::HashMap;
use turts_mod_api::{CameraDef, ContentCatalog, InputMapDef, ModError, OrderRulesDef, Shape};
use turts_mod_runtime::{archive::PackageArchive, visuals::visuals_by_content};

#[derive(Clone, Debug, PartialEq)]
pub struct ClientContent {
    pub catalog: ContentCatalog,
    pub shapes: HashMap<String, Shape>,
    pub input_map: InputMapDef,
    pub order_rules: OrderRulesDef,
    pub selection_priority: Vec<String>,
    pub camera: Option<CameraDef>,
}
impl Default for ClientContent {
    fn default() -> Self {
        Self {
            catalog: ContentCatalog {
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
            input_map: InputMapDef {
                actions: Vec::new(),
                engine_overrides: Vec::new(),
            },
            order_rules: OrderRulesDef {
                queue_depth: turts_mod_api::DEFAULT_QUEUE_DEPTH,
                smart: Vec::new(),
            },
            selection_priority: Vec::new(),
            camera: None,
        }
    }
}
impl ClientContent {
    pub fn from_archives(archives: &[PackageArchive]) -> Result<Self, ModError> {
        let shapes = visuals_by_content(archives)?;
        let mut catalog = ContentCatalog {
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
        };
        let mut selection_priority = Vec::new();
        let mut camera = None;
        for archive in archives {
            let part = archive.content()?;
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
        Ok(Self {
            catalog,
            shapes,
            input_map,
            order_rules,
            selection_priority,
            camera,
        })
    }
}
