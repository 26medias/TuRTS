use std::collections::HashMap;
use turts_mod_api::{ContentCatalog, ModError, Shape};
use turts_mod_runtime::{archive::PackageArchive, visuals::visuals_by_content};

#[derive(Clone, Debug, PartialEq)]
pub struct ClientContent {
    pub catalog: ContentCatalog,
    pub shapes: HashMap<String, Shape>,
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
                source_files: Default::default(),
            },
            shapes: HashMap::new(),
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
            source_files: Default::default(),
        };
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
            for (id, file) in part.source_files {
                if catalog.source_files.insert(id.clone(), file).is_some() {
                    return Err(ModError::new(
                        &id.package.0,
                        "id",
                        format!("duplicate content id `{id}`"),
                    ));
                }
            }
        }
        Ok(Self { catalog, shapes })
    }
}
