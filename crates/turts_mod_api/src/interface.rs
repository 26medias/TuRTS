use crate::{ContentRef, Written, orders::Capability};
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
    /// API 0.2.4, row H14: the hints of the player guide, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<HintDef>,
    /// API 0.2.4, row H14: the line that shows when no hint holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint_fallback: Option<String>,
    /// API 0.2.4, row H18: the unit that is selected at the start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_selection: Option<ContentRef>,
    /// API 0.2.4, row H3: texts of the state keys (`STATE_KEYS`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub states: BTreeMap<String, String>,
    /// API 0.2.4, row H16: the cap of effect sounds that play at one time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects_voices: Option<u16>,
    /// API 0.2.4, row H16: the distance at which an effect sound is silent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audible_range: Option<f32>,
    /// API 0.2.4, row H20: the least time between two alert sounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert_sound_gap_secs: Option<f32>,
}
impl InterfaceDef {
    /// True when the interface places a widget (at any depth) that is a button with the action
    /// `Input(name)`. The engine asks it with `toggle_sound`: it draws its own sound switch only
    /// when the answer is false.
    pub fn places_input_button(&self, name: &str) -> bool {
        fn walk(widget: &Widget, name: &str) -> bool {
            match &widget.widget {
                Primitive::Button {
                    action: ButtonAction::Input(action),
                    ..
                } => action == name,
                Primitive::Box { children, .. } => children.iter().any(|child| walk(child, name)),
                Primitive::List { item, .. } | Primitive::Grid { item, .. } => walk(item, name),
                _ => false,
            }
        }
        self.root.iter().any(|widget| walk(widget, name))
    }
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
        /// API 0.2.4, row H13. `Written` keeps the difference between an absent key and
        /// `font: None`: an older engine refuses the key (design 6.10, "Version-table
        /// detection"). The key is written back only when the file wrote it.
        #[serde(default, skip_serializing_if = "Written::is_unwritten")]
        font: Written<Option<String>>,
        /// API 0.2.4, row H13.
        #[serde(default, skip_serializing_if = "Written::is_unwritten")]
        size: Written<Option<u16>>,
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
    /// API 0.2.4, row H7: the supply of a slot, provided or consumed.
    Supply(ContentRef, SupplySide),
    Connection,
    Outcome,
    OutcomeReason,
    MatchStats(StatField),
    MatchClock,
    Hint,
    /// API 0.2.4, row H25: the words of the sound switch, `Sound: on` or `Sound: off`. It reads
    /// `Sound: on` only when the player has switched the sound on and the browser lets audio run.
    /// Put it in a `Button` label with the action `Input("toggle_sound")`. Engine fallback: a game
    /// whose interface places no widget with the action `toggle_sound` gets the engine's own switch
    /// (design 6.10, row H25), so the silent default always has a way back to sound.
    SoundState,
}

/// API 0.2.4, row H7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SupplySide {
    Provided,
    Consumed,
}

/// API 0.2.4, row H7: one count of the match statistics (design 6.7.9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum StatField {
    UnitsTrained,
    UnitsLost,
    UnitsDestroyed,
    StructuresCompleted,
    StructuresLost,
    /// A `Balance` resource.
    Gathered(ContentRef),
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
    /// API 0.2.4, row H8.
    Activity,
    Cargo,
    Armour,
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
    /// API 0.2.4, row H9.
    Role,
    BuildTimeNow,
    SupplyChange,
    CausesShortage,
    State,
    Paid,
    Refund,
    Badge,
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
    /// API 0.2.4, row H7: the list sources. They are in `Source` only: a list name is not a
    /// `Value` (orchestrator ruling M3 on the review of `L0-API1`, option (a)).
    ConstructionQueue,
    Sites,
    Notices,
    SelectionTypes,
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
    /// API 0.2.4, row H10.
    ItemHas(ItemField),
    PrimaryHasValue(PrimaryField),
    SupplyShort(ContentRef),
    MatchOver,
    OutcomeIs(OutcomeKind),
    /// The reason of the own slot (design 6.10 row H10).
    OutcomeReasonIs(OutcomeReasonKind),
    Placing,
    HintShown,
    /// API 0.2.4, row H25: true when `Value::SoundState` reads `Sound: on`. Use it to style the
    /// switch (for example one widget with `SoundOn` and one with `Not(SoundOn)`).
    SoundOn,
}

/// API 0.2.4, row H10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum OutcomeKind {
    Victory,
    Defeat,
    Draw,
}

/// API 0.2.4, row H10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum OutcomeReasonKind {
    KeyUnitsLost,
    Surrendered,
    Disconnected,
    OpponentsDefeated,
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
    /// API 0.2.4, row H11: the style of the placement ghost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<PlacementStyle>,
    /// API 0.2.4, row H11: the ring of the build area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_area_ring: Option<LineStyle>,
    /// API 0.2.4, row H11: the ring of the gap (it also draws the clearance ring of a node).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_ring: Option<LineStyle>,
    /// API 0.2.4, row H11: the progress bar of a site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress_bar: Option<BarStyle>,
    /// API 0.2.4, row H11: the load bar of a gatherer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_bar: Option<BarStyle>,
    /// API 0.2.4, row H11: the line to a rally point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rally: Option<LineStyle>,
    /// API 0.2.4, row H11: the floating text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub float_text: Option<FloatTextStyle>,
    /// API 0.2.4, row H24: name labels over units. `None`: no labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_labels: Option<LabelStyle>,
}

/// API 0.2.4, row H11: the colours of the placement ghost. All three fields are required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementStyle {
    pub valid: String,
    pub invalid: String,
    pub stripes: bool,
}

/// API 0.2.4, row H11: a ring or a line (a colour `#RRGGBB` and a width in pixels).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineStyle {
    pub colour: String,
    pub width: f32,
}

/// API 0.2.4, row H11: a bar in the world; the fields follow `WorldHealthBarStyle`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarStyle {
    pub width: u16,
    pub height: u16,
    pub background: String,
    pub fill: String,
}

/// API 0.2.4, row H11: floating text. `sound` is the id of a sound asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatTextStyle {
    pub colour: String,
    #[serde(default)]
    pub font: Option<String>,
    pub size: u16,
    #[serde(default)]
    pub sound: Option<String>,
}

/// API 0.2.4, row H24 (erratum E22): name labels over units. The label text is `UnitDef.name`.
/// `show`, `colour` and `size` are required; `own_only` is false and `font` is `None` when
/// absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelStyle {
    pub show: LabelShow,
    #[serde(default)]
    pub own_only: bool,
    pub colour: String,
    #[serde(default)]
    pub font: Option<String>,
    pub size: u16,
}

/// API 0.2.4, row H24. There is no `Never`: `None` in `unit_labels` means no labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum LabelShow {
    Always,
    Selected,
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
    /// API 0.2.4, row H12: the bar is as wide as the footprint of the unit.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub scale_to_footprint: bool,
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
    /// API 0.2.4, row H5: the unit types that raise the alert. `[]`: all units.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub units: Vec<ContentRef>,
    /// API 0.2.4, row H6.
    #[serde(default, skip_serializing_if = "AlertKind::is_event")]
    pub kind: AlertKind,
    /// API 0.2.4, row H6: seconds that a `State` alert stays after its trigger ends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_after: Option<f32>,
    /// API 0.2.4, row H17: a click on the alert selects its unit.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub select_on_click: bool,
    /// API 0.2.4, row H22: the unit types that the camera goes to, in order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<Vec<ContentRef>>,
    /// API 0.2.4, row H21.
    #[serde(default, skip_serializing_if = "SoundWhen::is_always")]
    pub sound_when: SoundWhen,
}

/// API 0.2.4, row H6: does the alert come once (`Event`), or hold while its trigger holds
/// (`State`)?
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AlertKind {
    #[default]
    Event,
    State,
}

impl AlertKind {
    pub fn is_event(&self) -> bool {
        *self == Self::Event
    }
}

/// API 0.2.4, row H21.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SoundWhen {
    #[default]
    Always,
    /// Only with the trigger `ProductionComplete`: the sound plays when the producer queue is
    /// empty after this completion.
    QueueEmpty,
}

impl SoundWhen {
    pub fn is_always(&self) -> bool {
        *self == Self::Always
    }
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
    /// API 0.2.4, row H4.
    ConstructionComplete,
    SupplyShort(ContentRef),
    SupplyRestored(ContentRef),
    InsufficientFunds(ContentRef),
    SlotDisconnected,
    SlotReconnected,
    /// A gatherer that has no order, for at least the given seconds.
    GathererIdle(f32),
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
    /// API 0.2.4, row H19: the role line of the info panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
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

/// API 0.2.4, row H14: one hint of the player guide. Every field is required.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HintDef {
    pub id: String,
    pub kind: HintKind,
    pub text: String,
    pub starts: HintCondition,
    pub done: HintCondition,
}

/// A `Step` is part of the path of a first match; a `Situation` shows when its condition holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum HintKind {
    Step,
    Situation,
}

/// API 0.2.4, row H14: the condition list of design 7.8.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum HintCondition {
    MatchStarted,
    OwnUnits {
        units: Vec<ContentRef>,
        state: OwnUnitState,
        at_least: u16,
    },
    OwnJob {
        production: ContentRef,
    },
    OwnRally {
        unit: ContentRef,
    },
    OrderAccepted {
        order: ContentRef,
        target: HintTarget,
    },
    SupplyShort(ContentRef),
    /// An own job has `held` true and `secs` have passed since its `held_since_tick`
    /// (design 6.4, erratum E21).
    JobOnHold {
        secs: f32,
    },
    HintDone(String),
    HintShownFor {
        id: String,
        secs: f32,
    },
    /// The selection holds at least one own unit of these types.
    Selected {
        units: Vec<ContentRef>,
    },
    /// The first step that is not done and whose start holds.
    StepCurrent(String),
    All(Vec<HintCondition>),
    Any(Vec<HintCondition>),
    Not(Box<HintCondition>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum OwnUnitState {
    Pending,
    Site,
    Complete,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum HintTarget {
    Point,
    Unit,
    Any,
}
