use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetsDef {
    pub images: Vec<ImageAsset>,
    pub fonts: Vec<FileAsset>,
    pub sounds: Vec<FileAsset>,
    pub cursors: Vec<CursorAsset>,
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
