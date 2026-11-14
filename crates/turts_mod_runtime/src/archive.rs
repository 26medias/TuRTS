//! Package archive read.

use std::collections::BTreeMap;
use std::io::Read;

use serde::de::{DeserializeOwned, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use turts_mod_api::{
    AssetsDef, CombatDef, ContentCatalog, InputMapDef, InterfaceDef, LockEntry, Manifest, MapDef,
    MatchDef, ModError, NodeDef, OrderDef, OrderRulesDef, PackageId, ProductionDef, RecipeDef,
    ResourceDef, SmartOrder, TechnologyDef, UnitDef, VisualDef, VisualEffectsDef,
    content::SourceDef,
};

use crate::content::validate_content_with_m3;
use crate::hash::sha256_hex;
use crate::manifest::validate_manifest;
use crate::parse::parse_ron;

/// Archive path of the manifest.
pub const MANIFEST_PATH: &str = "manifest.ron";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOrderRulesDef {
    queue_depth: u16,
    smart: Vec<SmartOrder>,
    #[serde(default)]
    rally: Option<Vec<SmartOrder>>,
}

fn parse_order_rules(package: &str, path: &str, text: &str) -> Result<OrderRulesDef, ModError> {
    let raw: RawOrderRulesDef = parse_ron(package, path, text)?;
    if raw.queue_depth > u8::MAX.into() {
        return Err(ModError::new(
            package,
            "queue_depth",
            format!("must be between 1 and 255, got {}", raw.queue_depth),
        )
        .with_file(path));
    }
    Ok(OrderRulesDef {
        queue_depth: raw.queue_depth as u8,
        smart: raw.smart,
        rally: raw.rally,
    })
}

/// A package archive, read in memory. Nothing is written to disk.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageArchive {
    /// Hash of the exact archive bytes.
    pub hash: String,
    /// Length of the archive in bytes.
    pub size: u64,
    pub manifest: Manifest,
    files: BTreeMap<String, Vec<u8>>,
}

impl PackageArchive {
    /// Read an archive and check its manifest. An unsafe path (absolute, `..`, empty or `.`
    /// part), a repeated path, an entry that is not a regular file, or a missing
    /// `manifest.ron` is an error with the field path `archive`.
    pub fn parse(bytes: &[u8]) -> Result<Self, ModError> {
        let files = read_entries(bytes)?;
        let manifest_bytes = files
            .get(MANIFEST_PATH)
            .ok_or_else(|| archive_error(format!("the archive has no {MANIFEST_PATH}")))?;
        let text = utf8(manifest_bytes, "", MANIFEST_PATH)?;
        let manifest: Manifest = parse_ron("", MANIFEST_PATH, text)?;
        validate_manifest(&manifest)?;
        Ok(Self {
            hash: sha256_hex(bytes),
            size: bytes.len() as u64,
            manifest,
            files,
        })
    }

    pub fn id(&self) -> PackageId {
        PackageId(self.manifest.id.clone())
    }

    /// The bytes of one file of the archive.
    pub fn file(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }

    /// The exported unit definitions as `(archive path, definition)`.
    pub fn units(&self) -> Result<Vec<(String, UnitDef)>, ModError> {
        self.parse_exports("exports.units", &self.manifest.exports.units)
    }

    /// Exported resource definitions as `(archive path, definition)`.
    pub fn resources(&self) -> Result<Vec<(String, ResourceDef)>, ModError> {
        self.parse_exports("exports.resources", &self.manifest.exports.resources)
    }

    /// Exported node definitions as `(archive path, definition)`.
    pub fn nodes(&self) -> Result<Vec<(String, NodeDef)>, ModError> {
        self.parse_exports("exports.nodes", &self.manifest.exports.nodes)
    }

    /// Exported production definitions as `(archive path, definition)`.
    pub fn production(&self) -> Result<Vec<(String, ProductionDef)>, ModError> {
        self.parse_exports("exports.production", &self.manifest.exports.production)
    }

    /// Exported recipe definitions as `(archive path, definition)`.
    pub fn recipes(&self) -> Result<Vec<(String, RecipeDef)>, ModError> {
        self.parse_exports("exports.recipes", &self.manifest.exports.recipes)
    }

    /// Exported technology definitions as `(archive path, definition)`.
    pub fn technologies(&self) -> Result<Vec<(String, TechnologyDef)>, ModError> {
        self.parse_exports("exports.technologies", &self.manifest.exports.technologies)
    }

    pub fn orders(&self) -> Result<Vec<(String, OrderDef)>, ModError> {
        let mut out = Vec::new();
        for path in &self.manifest.exports.orders {
            let text = self.export_text("exports.orders", path)?;
            match parse_ron::<OrderDef>(&self.manifest.id, path, text) {
                Ok(value) => out.push((path.clone(), value)),
                Err(order_error) => match parse_order_rules(&self.manifest.id, path, text) {
                    Ok(_) => {}
                    Err(rules_error) => {
                        return Err(classified_order_error(
                            &self.manifest.id,
                            path,
                            text,
                            order_error,
                            rules_error,
                        ));
                    }
                },
            }
        }
        Ok(out)
    }
    pub fn order_rules(&self) -> Result<Vec<(String, OrderRulesDef)>, ModError> {
        let mut out = Vec::new();
        for path in &self.manifest.exports.orders {
            let text = self.export_text("exports.orders", path)?;
            match parse_ron::<OrderDef>(&self.manifest.id, path, text) {
                Ok(_) => {}
                Err(order_error) => match parse_order_rules(&self.manifest.id, path, text) {
                    Ok(value) => out.push((path.clone(), value)),
                    Err(rules_error) => {
                        return Err(classified_order_error(
                            &self.manifest.id,
                            path,
                            text,
                            order_error,
                            rules_error,
                        ));
                    }
                },
            }
        }
        Ok(out)
    }
    pub fn input_maps(&self) -> Result<Vec<(String, InputMapDef)>, ModError> {
        self.parse_exports("exports.input", &self.manifest.exports.input)
    }
    pub fn interfaces(&self) -> Result<Vec<(String, InterfaceDef)>, ModError> {
        self.parse_exports("exports.interface", &self.manifest.exports.interface)
    }
    pub fn assets(&self) -> Result<Vec<(String, AssetsDef)>, ModError> {
        self.parse_exports("exports.assets", &self.manifest.exports.assets)
    }
    /// The exported damage tables (API 0.2.4, `exports.combat`) as `(archive path, definition)`.
    pub fn combat_defs(&self) -> Result<Vec<(String, CombatDef)>, ModError> {
        self.parse_exports("exports.combat", &self.manifest.exports.combat)
    }
    /// The exported visual effect files (API 0.2.4, `exports.visual_effects`). Only the client
    /// reads them.
    pub fn visual_effect_defs(&self) -> Result<Vec<(String, VisualEffectsDef)>, ModError> {
        self.parse_exports(
            "exports.visual_effects",
            &self.manifest.exports.visual_effects,
        )
    }
    /// The exported match files (API 0.2.4, `exports.match`) as `(archive path, definition)`.
    pub fn match_defs(&self) -> Result<Vec<(String, MatchDef)>, ModError> {
        self.parse_exports("exports.match", &self.manifest.exports.match_files)
    }

    /// Parse and resolve all gameplay content. Presentation files are never read here.
    pub fn content(&self) -> Result<ContentCatalog, ModError> {
        // The version table speaks first: the RAW rows need only the raw documents. A package
        // that writes a field of a newer API is told so before a content cross-check fails
        // for the same reason (an old API has no `input` group, so "unknown input action").
        // The rows and lines of the files that only the client reads (visuals, visual effects)
        // run in `visuals`: this path never reads a presentation file.
        let rows = crate::version_table::rows_for(false);
        let lines = crate::unbuilt::lines_for(false, crate::version_table::VERSION_TABLE);
        let raw = crate::raw_presence::RawDocuments::read(
            &self.manifest,
            crate::version_table::raw_groups(&rows),
            |path| self.file(path),
        )?;
        crate::version_table::check_version_table(&self.manifest, None, &raw, &rows)?;
        // A feature of the API that its lane has not built yet is refused here, after the
        // version: a `^0.2.3` package hears about the version first.
        crate::unbuilt::check_unbuilt(
            &self.manifest,
            None,
            &raw,
            &lines,
            crate::version_table::VERSION_TABLE,
        )?;
        let origin = self.id();
        let resource_files = self.resources()?;
        let unit_files = self.units()?;
        let production_files = self.production()?;
        let technology_files = self.technologies()?;
        let match_files = self.match_defs()?;
        let resources = resource_files
            .iter()
            .cloned()
            .map(|(file, value)| SourceDef { file, value })
            .collect();
        let units = unit_files
            .iter()
            .cloned()
            .map(|(file, value)| SourceDef { file, value })
            .collect();
        let nodes = self
            .nodes()?
            .into_iter()
            .map(|(file, value)| SourceDef { file, value })
            .collect();
        let production = production_files
            .iter()
            .cloned()
            .map(|(file, value)| SourceDef { file, value })
            .collect();
        let recipes = self
            .recipes()?
            .into_iter()
            .map(|(file, value)| SourceDef { file, value })
            .collect();
        let technologies = technology_files
            .iter()
            .cloned()
            .map(|(file, value)| SourceDef { file, value })
            .collect();
        let mut catalog = validate_content_with_m3(
            &origin,
            resources,
            units,
            nodes,
            production,
            recipes,
            technologies,
        )?;
        let mut mc_content = crate::content_mc_validation::validate_mc(self)?;
        // The parsed gameplay files, for the rows of the version table that read a variant.
        mc_content.units = unit_files;
        mc_content.resources = resource_files;
        mc_content.production = production_files;
        mc_content.technologies = technology_files;
        mc_content.matches = match_files;
        mc_content.mobile_weapon_present = catalog
            .units
            .values()
            .any(|unit| unit.weapon.is_some() && unit.movement.is_some());
        let order_queue_depth = mc_content
            .order_rules
            .first()
            .map_or(turts_mod_api::DEFAULT_QUEUE_DEPTH, |(_, rules)| {
                rules.queue_depth
            });
        catalog
            .order_queue_depths
            .insert(origin.clone(), order_queue_depth);
        for (file, order) in &mc_content.orders {
            let id = turts_mod_api::ContentId::new(&origin, &order.id);
            catalog.orders.insert(id.clone(), order.clone());
            catalog.source_files.insert(id, file.clone());
        }
        crate::version_table::check_version_table(&self.manifest, Some(&mc_content), &raw, &rows)?;
        crate::unbuilt::check_unbuilt(
            &self.manifest,
            Some(&mc_content),
            &raw,
            &lines,
            crate::version_table::VERSION_TABLE,
        )?;
        // The validation column of the 0.2.4 table (`L0-API2`), after the `UNBUILT` refusal: a
        // package that uses a feature that is not built hears that first, because nothing else
        // about it can be used yet. The lane that builds a feature deletes its line, and the
        // checks of the feature are already in force. A test reaches them while the line stands
        // through `check_api_0_2_4`.
        self.check_api_0_2_4()?;
        crate::content_m3::validate_m3(&self.manifest, &catalog)?;
        Ok(catalog)
    }

    /// The validation column of the 0.2.4 table (design 6.10) on the gameplay, order, match,
    /// interface and input files. `Ok` for a package of an edition older than 0.2.4. `content`
    /// calls it after the `UNBUILT_FEATURES` refusal, so a feature that is not built is refused
    /// as not built; this method reaches the checks of such a feature.
    pub fn check_api_0_2_4(&self) -> Result<(), ModError> {
        crate::mp1_validation::validate(self)
    }

    /// The exported map definitions as `(archive path, definition)`.
    pub fn maps(&self) -> Result<Vec<(String, MapDef)>, ModError> {
        self.parse_exports("exports.maps", &self.manifest.exports.maps)
    }

    /// The exported visual definitions as `(archive path, definition)`. Only the client
    /// calls this.
    pub fn visuals(&self) -> Result<Vec<(String, VisualDef)>, ModError> {
        self.check_presentation_versions()?;
        self.visuals_unchecked()
    }

    /// The visuals with no version check: the presentation checks of API 0.2.4 read them.
    pub(crate) fn visuals_unchecked(&self) -> Result<Vec<(String, VisualDef)>, ModError> {
        self.parse_exports("exports.visuals", &self.manifest.exports.visuals)
    }

    /// The version rows and the `UNBUILT` lines of the files that only the client reads (visuals
    /// and visual effects, API 0.2.4): the same two steps as `content`, on the raw documents of
    /// those groups. `content` never reads them.
    fn check_presentation_versions(&self) -> Result<(), ModError> {
        let rows = crate::version_table::rows_for(true);
        let lines = crate::unbuilt::lines_for(true, crate::version_table::VERSION_TABLE);
        let raw = crate::raw_presence::RawDocuments::read(
            &self.manifest,
            crate::version_table::raw_groups(&rows),
            |path| self.file(path),
        )?;
        crate::version_table::check_version_table(&self.manifest, None, &raw, &rows)?;
        crate::unbuilt::check_unbuilt(
            &self.manifest,
            None,
            &raw,
            &lines,
            crate::version_table::VERSION_TABLE,
        )?;
        crate::mp1_validation::validate_presentation(self)
    }

    /// The checks of API 0.2.4 on the visual and effect files (rows V3 and V4). `visuals` calls
    /// them after the `UNBUILT` refusal; this method reaches them while a line stands.
    pub fn check_api_0_2_4_presentation(&self) -> Result<(), ModError> {
        crate::mp1_validation::validate_presentation(self)
    }

    /// The lock entry that describes this archive. The caller gives the download URLs.
    pub fn lock_entry(&self, urls: Vec<String>) -> LockEntry {
        LockEntry {
            id: self.manifest.id.clone(),
            version: self.manifest.version.clone(),
            hash: self.hash.clone(),
            size: self.size,
            urls,
            context: self.manifest.context,
            engine_api: self.manifest.engine_api.clone(),
        }
    }

    /// Check a lock entry against this archive. The first mismatch is the error, with the
    /// field path `lock.<field>`. The archive cannot say where it was downloaded from, so
    /// `urls` is the one lock field that this check does not compare.
    pub fn check_against_lock(&self, entry: &LockEntry) -> Result<(), ModError> {
        let own = self.lock_entry(Vec::new());
        let fields = [
            ("id", own.id, entry.id.clone()),
            ("version", own.version, entry.version.clone()),
            ("hash", own.hash, entry.hash.clone()),
            ("size", own.size.to_string(), entry.size.to_string()),
            (
                "context",
                format!("{:?}", own.context),
                format!("{:?}", entry.context),
            ),
            ("engine_api", own.engine_api, entry.engine_api.clone()),
        ];
        for (field, archive, lock) in fields {
            if archive != lock {
                return Err(ModError::new(
                    &self.manifest.id,
                    &format!("lock.{field}"),
                    format!("the archive has {field} `{archive}` but the lock entry has `{lock}`"),
                ));
            }
        }
        Ok(())
    }

    fn parse_exports<T: DeserializeOwned>(
        &self,
        field: &str,
        paths: &[String],
    ) -> Result<Vec<(String, T)>, ModError> {
        let package = self.manifest.id.as_str();
        let mut out = Vec::with_capacity(paths.len());
        for path in paths {
            let text = self.export_text(field, path)?;
            out.push((path.clone(), parse_ron(package, path, text)?));
        }
        Ok(out)
    }

    fn export_text(&self, field: &str, path: &str) -> Result<&str, ModError> {
        let bytes = self.file(path).ok_or_else(|| {
            ModError::new(
                &self.manifest.id,
                field,
                format!("exported file `{path}` is not in the archive"),
            )
            .with_file(MANIFEST_PATH)
        })?;
        utf8(bytes, &self.manifest.id, path)
    }
}

fn classified_order_error(
    package: &str,
    path: &str,
    text: &str,
    order_error: ModError,
    rules_error: ModError,
) -> ModError {
    match ron::from_str::<OrderExportFields>(text) {
        Ok(fields) if fields.0.contains("queue_depth") || fields.0.contains("smart") => rules_error,
        Ok(_) => order_error,
        Err(_) => ModError::new(
            package,
            path,
            format!(
                "cannot parse this export as an order or order rules (order: {}; rules: {})",
                order_error.message, rules_error.message
            ),
        )
        .with_file(path),
    }
}

struct OrderExportFields(std::collections::BTreeSet<String>);

impl<'de> Deserialize<'de> for OrderExportFields {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldSetVisitor;

        impl<'de> Visitor<'de> for FieldSetVisitor {
            type Value = OrderExportFields;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a named RON struct")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut fields = std::collections::BTreeSet::new();
                while let Some(field) = map.next_key::<String>()? {
                    let _: IgnoredAny = map.next_value()?;
                    fields.insert(field);
                }
                Ok(OrderExportFields(fields))
            }
        }

        deserializer.deserialize_struct("", &[], FieldSetVisitor)
    }
}

/// Whether an archive path is relative, has no empty, `.` or `..` part, and has no backslash.
pub(crate) fn is_safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn archive_error(message: String) -> ModError {
    ModError::new("", "archive", message)
}

fn utf8<'a>(bytes: &'a [u8], package: &str, file: &str) -> Result<&'a str, ModError> {
    std::str::from_utf8(bytes)
        .map_err(|_| ModError::new(package, file, "the file is not valid UTF-8").with_file(file))
}

/// Read every regular file of the tar into memory. Directory entries are skipped.
fn read_entries(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, ModError> {
    // The tar error text can echo bytes of the archive, so the message does not carry it.
    let unreadable =
        |_: std::io::Error| archive_error("the bytes are not a valid tar archive".to_string());
    let mut files = BTreeMap::new();
    let mut archive = tar::Archive::new(bytes);
    for entry in archive.entries().map_err(unreadable)? {
        let mut entry = entry.map_err(unreadable)?;
        let raw = entry.path_bytes().into_owned();
        let name = String::from_utf8_lossy(&raw).into_owned();
        let kind = entry.header().entry_type();
        let path = match std::str::from_utf8(&raw) {
            Ok(text) if kind.is_dir() => text.strip_suffix('/').unwrap_or(text),
            Ok(text) => text,
            Err(_) => "",
        };
        if !is_safe_path(path) {
            return Err(archive_error(format!("unsafe path `{name}`")));
        }
        if kind.is_dir() {
            continue;
        }
        if !kind.is_file() {
            return Err(archive_error(format!(
                "entry `{name}` is not a regular file"
            )));
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(unreadable)?;
        if files.insert(path.to_string(), data).is_some() {
            return Err(archive_error(format!("duplicate path `{name}`")));
        }
    }
    Ok(files)
}
