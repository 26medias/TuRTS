//! Declarative presentation layers. These types contain no renderer or simulation state.
use crate::{ContentRef, schema::DamagedEffect};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectCost {
    #[default]
    Small,
    Medium,
    Large,
}
impl EffectCost {
    pub fn is_small(&self) -> bool {
        *self == Self::Small
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectBlend {
    #[default]
    Alpha,
    Add,
}
/// Legacy damage conditions remain valid. A list allows smoke and fire at separate thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DamagedEffects {
    Single(DamagedEffect),
    Multiple(Vec<DamagedEffect>),
}
impl DamagedEffects {
    pub fn as_slice(&self) -> &[DamagedEffect] {
        match self {
            Self::Single(effect) => std::slice::from_ref(effect),
            Self::Multiple(effects) => effects,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectLayer {
    pub kind: EffectLayerKind,
    #[serde(default = "white")]
    pub colour: String,
    #[serde(default)]
    pub end_colour: Option<String>,
    #[serde(default = "small")]
    pub size: f32,
    #[serde(default)]
    pub end_size: Option<f32>,
    #[serde(default = "short_ms")]
    pub ms: u32,
    #[serde(default)]
    pub delay_ms: u32,
    #[serde(default)]
    pub image: Option<ContentRef>,
    #[serde(default)]
    pub blend: EffectBlend,
    #[serde(default)]
    pub offset: [f32; 3],
    #[serde(default)]
    pub looping: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum EffectLayerKind {
    Flash {
        #[serde(default = "one")]
        radius: f32,
        #[serde(default)]
        light: bool,
    },
    Sparks {
        #[serde(default = "eight")]
        count: u16,
        #[serde(default = "three")]
        speed: f32,
        #[serde(default = "spread")]
        spread_deg: f32,
        #[serde(default = "gravity")]
        gravity: f32,
    },
    Burst {
        #[serde(default = "small")]
        radius_start: f32,
        #[serde(default = "one")]
        radius_end: f32,
    },
    Smoke {
        #[serde(default = "rate")]
        rate: f32,
        #[serde(default = "half")]
        rise: f32,
        #[serde(default = "half")]
        spread: f32,
    },
    Fire {
        #[serde(default = "rate")]
        rate: f32,
        #[serde(default = "one")]
        height: f32,
    },
    Debris {
        #[serde(default = "eight")]
        count: u16,
        #[serde(default = "three")]
        speed: f32,
        #[serde(default)]
        model: Option<ContentRef>,
        #[serde(default)]
        image: Option<ContentRef>,
    },
    Tracer {
        #[serde(default = "thin")]
        width: f32,
    },
    Projectile {
        #[serde(default)]
        model: Option<ContentRef>,
        #[serde(default)]
        image: Option<ContentRef>,
        #[serde(default = "projectile_speed")]
        speed: f32,
        #[serde(default)]
        arc_height: f32,
        #[serde(default)]
        trail: Option<ContentRef>,
    },
    Trail {
        #[serde(default = "thin")]
        width: f32,
        #[serde(default = "short_ms")]
        ms: u32,
    },
    Decal {
        #[serde(default)]
        image: Option<ContentRef>,
        #[serde(default = "one")]
        size: f32,
        #[serde(default = "decal_ms")]
        ms: u32,
    },
    Shake {
        #[serde(default = "small")]
        strength: f32,
        #[serde(default = "short_ms")]
        ms: u32,
    },
}
fn white() -> String {
    "#ffffff".into()
}
fn one() -> f32 {
    1.
}
fn half() -> f32 {
    0.5
}
fn small() -> f32 {
    0.1
}
fn thin() -> f32 {
    0.04
}
fn three() -> f32 {
    3.
}
fn eight() -> u16 {
    8
}
fn rate() -> f32 {
    8.
}
fn gravity() -> f32 {
    9.
}
fn spread() -> f32 {
    60.
}
fn projectile_speed() -> f32 {
    30.
}
fn short_ms() -> u32 {
    250
}
fn decal_ms() -> u32 {
    60_000
}
