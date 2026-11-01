use crate::{ContentRef, orders::Capability};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceDef {
    pub id: String,
    pub root: Vec<Widget>,
    pub tooltip_style: TooltipStyle,
    pub cursors: BTreeMap<String, String>,
    pub camera: CameraDef,
    pub light: Option<LightDef>,
    pub slot_colours: Vec<String>,
    pub world: WorldStyle,
    pub alerts: Vec<AlertDef>,
    pub reasons: BTreeMap<String, String>,
    pub units: BTreeMap<ContentRef, UnitPresentation>,
    pub selection_priority: Vec<ContentRef>,
    pub guide: Vec<GuidePage>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Widget {
    pub id: String,
    #[serde(default)]
    pub at: Option<At>,
    #[serde(default = "fill_size")]
    pub size: Size,
    #[serde(default)]
    pub layer: i16,
    #[serde(default)]
    pub visible: Vec<Predicate>,
    pub widget: Primitive,
    #[serde(default)]
    pub tooltip: Option<Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Primitive {
    Box {
        fill: Option<Fill>,
        border: Option<String>,
        children: Vec<Widget>,
    },
    Image {
        image: ButtonIcon,
        nine_slice: Option<(u16, u16, u16, u16)>,
        tint: Option<String>,
    },
    Text {
        text: TextValue,
        font: Option<String>,
        size: u16,
        colour: String,
        align: TextAlign,
        wrap: bool,
    },
    Bar {
        value: Value,
        max: Value,
        fill: Fill,
        background: Fill,
        direction: Direction,
    },
    Button {
        icon: Option<ButtonIcon>,
        label: Option<TextValue>,
        action: ButtonAction,
        hotkey_label: bool,
    },
    List {
        source: Source,
        direction: Direction,
        gap: u16,
        max_items: u16,
        item: Box<Widget>,
    },
    Grid {
        source: Source,
        columns: u16,
        rows: u16,
        cell: (u16, u16),
        gap: u16,
        item: Box<Widget>,
    },
    Tooltip {
        value: Value,
    },
    Cursor {
        cursors: BTreeMap<String, (String, (u16, u16))>,
    },
    MinimapView {
        #[serde(default)]
        background: Option<Fill>,
        frame: String,
        #[serde(default = "default_minimap_unit_size")]
        unit_size: f32,
        #[serde(default)]
        ping: Option<String>,
    },
}
fn fill_size() -> Size {
    Size {
        w: Dimension::Fill,
        h: Dimension::Fill,
    }
}

fn default_minimap_unit_size() -> f32 {
    2.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ButtonIcon {
    Asset(String),
    Item(IconField),
    Bind(Value),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ButtonAction {
    Order(String),
    Item(ActionField),
    Input(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum IconField {
    Icon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ActionField {
    Action,
    CancelAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Fill {
    Colour(String),
    Image {
        id: String,
        nine_slice: Option<(u16, u16, u16, u16)>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TextValue {
    Literal(String),
    Bind(Value),
    Item(ItemField),
    Format(String, Vec<Value>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Value {
    Balance(String),
    Balances,
    SelectionPrimary(PrimaryField),
    SelectionUnits,
    CommandCard,
    BuildMenu,
    ProductionQueue,
    ControlGroups,
    Alerts,
    LastNotice,
    Guide,
    GuideOpen,
    Bindings,
    Item(ItemField),
    Literal(String),
    Format(String, Vec<Value>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum PrimaryField {
    Name,
    Icon,
    Portrait,
    Owner,
    Health,
    MaxHealth,
    Inventory,
    Damage,
    Range,
    Ammo,
    Cooldown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ItemField {
    Icon,
    Name,
    Action,
    Enabled,
    Reason,
    CardCell,
    Health,
    Count,
    Cost,
    Time,
    Progress,
    RemainingSeconds,
    CancelAction,
    Title,
    Body,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Source {
    Balance(String),
    Balances,
    SelectionUnits,
    CommandCard,
    BuildMenu,
    ProductionQueue,
    ControlGroups,
    Alerts,
    Bindings,
    Guide,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Predicate {
    SelectionEmpty,
    SelectionNotEmpty,
    SelectionSingle,
    SelectionMany,
    PrimaryOwned,
    PrimaryForeign,
    PrimaryHas(Capability),
    GuideOpen,
    Not(Box<Predicate>),
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Centre,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
    Frac(f32, f32),
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Dimension {
    Px(f32),
    Frac(f32),
    Fill,
    Fit,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub at: Option<At>,
    pub size: Size,
    pub layer: i16,
    pub visible: Vec<Predicate>,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct At {
    pub anchor: Anchor,
    pub offset: (f32, f32),
    #[serde(default)]
    pub pivot: Option<Anchor>,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Size {
    pub w: Dimension,
    pub h: Dimension,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Direction {
    Horizontal,
    Vertical,
    Up,
    Down,
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TextAlign {
    Left,
    Centre,
    Right,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TooltipStyle {
    pub background: String,
    pub colour: String,
    pub font: Option<String>,
    pub size: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraDef {
    pub pitch: f32,
    pub yaw: f32,
    pub field_of_view: f32,
    pub minimum: f32,
    pub maximum: f32,
    pub start: CameraStart,
    pub height: f32,
    pub margin: f32,
    pub pan_speed: f32,
    pub edge_band: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum CameraStart {
    OwnedCentre,
    MapCentre,
    Point(f32, f32),
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPoint {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LightDef {
    pub direction: (f32, f32, f32),
    pub colour: String,
    pub illuminance: f32,
    pub ambient: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldStyle {
    pub ground: String,
    pub grid: String,
    pub selection: String,
    pub health: String,
    pub range: String,
    pub show_range: bool,
    pub tooltip: WorldTooltipStyle,
    pub health_bar: WorldHealthBarStyle,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldTooltipStyle {
    pub background: String,
    pub colour: String,
    pub font: Option<String>,
    pub size: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldHealthBarStyle {
    pub width: u16,
    pub height: u16,
    pub background: String,
    pub health: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertDef {
    pub id: String,
    pub trigger: AlertTrigger,
    pub sound: Option<String>,
    pub text: String,
    pub minimap_ping: bool,
    pub cooldown: f32,
    pub priority: i16,
    pub coalesce_radius: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AlertTrigger {
    OwnUnitDamaged,
    OwnUnitDestroyed,
    ProductionComplete,
    ResearchComplete,
    ResearchReady,
    ExtractorFull,
    ExtractorDepleted,
    OutputBlocked,
    OutOfAmmo,
    InserterWaiting(f32),
    GathererWaiting(f32),
}
impl AlertTrigger {
    pub fn waiting_seconds(&self) -> Option<f32> {
        match self {
            Self::InserterWaiting(seconds) | Self::GathererWaiting(seconds) => Some(*seconds),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitPresentation {
    pub icon: Option<String>,
    pub portrait: Option<String>,
    pub responses: BTreeMap<String, Vec<String>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidePage {
    pub id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub widgets: Vec<Widget>,
}
