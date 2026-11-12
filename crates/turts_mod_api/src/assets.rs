use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetsDef {
    pub images: Vec<ImageAsset>,
    pub fonts: Vec<FileAsset>,
    pub sounds: Vec<FileAsset>,
    pub cursors: Vec<CursorAsset>,
    /// API 0.2.4, row V1: glTF binary models (`id`, `path`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<FileAsset>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageAsset {
    pub id: String,
    pub path: String,
    pub nine_slice: Option<(u16, u16, u16, u16)>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileAsset {
    pub id: String,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorAsset {
    pub id: String,
    pub image: String,
    pub hotspot: (u16, u16),
}
