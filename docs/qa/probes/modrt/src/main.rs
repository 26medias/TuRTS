// Probe for gate 1 of the QA0 design, K1 (O2 field table versus O1 serde(default)).
// It uses the REAL component types of turts_mod_api (path dependency) and ron 0.12.2.
// UnitDefO2 has the same typed fields as turts_mod_api::UnitDef, so the consumers do not change.
// Its Deserialize is hand-written and dispatches every non-fixed key through TABLE.
use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use std::collections::BTreeSet;
use std::fmt;
use turts_mod_api::*;

struct Row {
    name: &'static str,
    since: (u64, u64, u64),
    required: bool,
}

// One row per optional key. `since` values are illustrative.
const TABLE: &[Row] = &[
    Row { name: "movement", since: (0, 1, 0), required: true },
    Row { name: "inventory", since: (0, 2, 0), required: true },
    Row { name: "gatherer", since: (0, 2, 0), required: true },
    Row { name: "drop_off", since: (0, 2, 0), required: true },
    Row { name: "producer", since: (0, 2, 0), required: true },
    Row { name: "output_inventory", since: (0, 2, 1), required: false },
    Row { name: "extractor", since: (0, 2, 1), required: false },
    Row { name: "conveyor", since: (0, 2, 1), required: false },
    Row { name: "inserter", since: (0, 2, 1), required: false },
    Row { name: "machine", since: (0, 2, 1), required: false },
    Row { name: "research", since: (0, 2, 1), required: false },
    Row { name: "health", since: (0, 2, 1), required: false },
    Row { name: "weapon", since: (0, 2, 1), required: false },
    Row { name: "removal", since: (0, 2, 1), required: false },
];

// serde needs &'static [&'static str] for unknown_field: a second list, kept in sync by hand.
const ALL: &[&str] = &[
    "id", "name", "footprint", "movement", "inventory", "gatherer", "drop_off", "producer",
    "output_inventory", "extractor", "conveyor", "inserter", "machine", "research", "health",
    "weapon", "removal",
];

#[derive(Debug, Default)]
struct UnitDefO2 {
    id: Option<String>,
    name: Option<String>,
    footprint: Option<Footprint>,
    movement: Option<MovementDef>,
    inventory: Option<InventoryDef>,
    gatherer: Option<GathererDef>,
    drop_off: Option<DropOffDef>,
    producer: Option<ProducerDef>,
    output_inventory: Option<InventoryDef>,
    extractor: Option<ExtractorDef>,
    conveyor: Option<ConveyorDef>,
    inserter: Option<InserterDef>,
    machine: Option<MachineDef>,
    research: Option<ResearchStationDef>,
    health: Option<HealthDef>,
    weapon: Option<WeaponDef>,
    removal: Option<RemovalDef>,
    present: BTreeSet<&'static str>,
}

static DUP_CHECK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

struct V;
impl<'de> Visitor<'de> for V {
    type Value = UnitDefO2;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a unit")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<UnitDefO2, A::Error> {
        let mut u = UnitDefO2::default();
        while let Some(k) = m.next_key::<String>()? {
            let Some(name) = ALL.iter().copied().find(|n| *n == k) else {
                return Err(de::Error::unknown_field(&k, ALL));
            };
            if !u.present.insert(name) && DUP_CHECK.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(de::Error::duplicate_field(name));
            }
            // A typed arm per key is still needed: the table cannot hold the value type.
            match name {
                "id" => u.id = Some(m.next_value()?),
                "name" => u.name = Some(m.next_value()?),
                "footprint" => u.footprint = Some(m.next_value()?),
                "movement" => u.movement = m.next_value()?,
                "inventory" => u.inventory = m.next_value()?,
                "gatherer" => u.gatherer = m.next_value()?,
                "drop_off" => u.drop_off = m.next_value()?,
                "producer" => u.producer = m.next_value()?,
                "output_inventory" => u.output_inventory = m.next_value()?,
                "extractor" => u.extractor = m.next_value()?,
                "conveyor" => u.conveyor = m.next_value()?,
                "inserter" => u.inserter = m.next_value()?,
                "machine" => u.machine = m.next_value()?,
                "research" => u.research = m.next_value()?,
                "health" => u.health = m.next_value()?,
                "weapon" => u.weapon = m.next_value()?,
                "removal" => u.removal = m.next_value()?,
                _ => unreachable!(),
            }
        }
        for fixed in ["id", "name", "footprint"] {
            if !u.present.contains(fixed) {
                return Err(de::Error::missing_field(fixed));
            }
        }
        for row in TABLE.iter().filter(|r| r.required) {
            if !u.present.contains(row.name) {
                return Err(de::Error::missing_field(row.name));
            }
        }
        Ok(u)
    }
}
impl<'de> Deserialize<'de> for UnitDefO2 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_struct("UnitDef", ALL, V)
    }
}
impl UnitDefO2 {
    fn into_real(self) -> UnitDef {
        UnitDef {
            id: self.id.unwrap(),
            name: self.name.unwrap(),
            footprint: self.footprint.unwrap(),
            movement: self.movement,
            inventory: self.inventory,
            gatherer: self.gatherer,
            drop_off: self.drop_off,
            producer: self.producer,
            output_inventory: self.output_inventory,
            extractor: self.extractor,
            conveyor: self.conveyor,
            inserter: self.inserter,
            machine: self.machine,
            research: self.research,
            health: self.health,
            weapon: self.weapon,
            removal: self.removal,
        }
    }
}

// The design's version gate: refuse a present field when engine_api "matches a version below
// since_api". semver has no "admits any version below X" query, so this tests one version:
// the version just before `since` (patch - 1), as content_m3_validation.rs does with 0.2.0.
fn gate(req: &str, u: &UnitDefO2) -> Result<(), String> {
    let r = semver::VersionReq::parse(req).unwrap();
    for row in TABLE {
        if !u.present.contains(row.name) {
            continue;
        }
        let (a, b, c) = row.since;
        if c == 0 {
            continue;
        }
        let below = semver::Version::new(a, b, c - 1);
        if r.matches(&below) {
            return Err(format!("`{}` needs engine api {a}.{b}.{c}; `{req}` admits {below}", row.name));
        }
    }
    Ok(())
}

fn show<T: fmt::Debug>(label: &str, r: Result<T, ron::error::SpannedError>) -> String {
    match r {
        Ok(_) => format!("{label}: ACCEPTED"),
        Err(e) => format!("{label}: REJECTED {:?}", e.code),
    }
}

fn main() {
    // 1. Golden agreement on every unit file in the repo.
    let mut files = Vec::new();
    for root in ["/home/julien/Projects/TuRTS/mods", "/home/julien/Projects/TuRTS/crates"] {
        walk(std::path::Path::new(root), &mut files);
    }
    files.sort();
    let (mut same, mut diff, mut rt) = (0, 0, 0);
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let real: Result<UnitDef, _> = ron::from_str(&text);
        let o2: Result<UnitDefO2, _> = ron::from_str(&text);
        match (real, o2) {
            (Ok(a), Ok(b)) => {
                let b = b.into_real();
                if a == b { same += 1 } else { diff += 1; println!("DIFF {}", f.display()) }
                // Round trip as m2_variants.rs does: ron::ser::to_string_pretty then parse.
                let ser = ron::ser::to_string_pretty(&a, ron::ser::PrettyConfig::default()).unwrap();
                let back: UnitDefO2 = ron::from_str(&ser).expect("round trip parses");
                if back.into_real() == a { rt += 1 } else { println!("RT DIFF {}", f.display()) }
            }
            (a, b) => { diff += 1; println!("PARSE MISMATCH {} real={} o2={}", f.display(), a.is_ok(), b.is_ok()) }
        }
    }
    println!("G golden: {} files, {same} equal, {diff} differ, {rt} round-trip equal", files.len());

    // 2. Error cases, real derive (= O1) versus O2.
    let base = r#"(id: "s", name: "S", footprint: Circle(radius: 0.6), movement: Some((speed: 6.0)), inventory: None, gatherer: None, drop_off: None, producer: None"#;
    let cases = [
        ("E0 valid", format!("{base})")),
        ("E1 typo helth", format!("{base}, helth: Some((max: 5)))")),
        ("E2 duplicate health", format!("{base}, health: Some((max: 5)), health: Some((max: 6)))")),
        ("E3 duplicate required movement", r#"(id: "s", name: "S", footprint: Circle(radius: 0.6), movement: None, movement: Some((speed: 6.0)), inventory: None, gatherer: None, drop_off: None, producer: None)"#.to_string()),
        ("E4 missing required movement", r#"(id: "s", name: "S", footprint: Circle(radius: 0.6), inventory: None, gatherer: None, drop_off: None, producer: None)"#.to_string()),
        ("E5 missing fixed name", r#"(id: "s", footprint: Circle(radius: 0.6), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None)"#.to_string()),
        ("E6 typo inside block", format!("{base}, health: Some((maxx: 5)))")),
    ];
    for (label, text) in &cases {
        println!("{}", show(&format!("{label} [O1 real UnitDef]"), ron::from_str::<UnitDef>(text)));
        println!("{}", show(&format!("{label} [O2 table]"), ron::from_str::<UnitDefO2>(text)));
    }
    DUP_CHECK.store(false, std::sync::atomic::Ordering::Relaxed);
    println!("{}", show("E2 duplicate health [O2 naive, no seen-set]", ron::from_str::<UnitDefO2>(&cases[2].1)));
    DUP_CHECK.store(true, std::sync::atomic::Ordering::Relaxed);

    // 3. Version gate (S4) against the real ENGINE_API_VERSION.
    let engine = semver::Version::parse(ENGINE_API_VERSION).unwrap();
    println!("V engine = {engine}");
    for req in ["^0.2", "^0.2.1", "^0.3", "^0.3.0"] {
        let r = semver::VersionReq::parse(req).unwrap();
        println!("V manifest.rs:34 engine check for `{req}` against {engine}: {}", if r.matches(&engine) { "pass" } else { "REFUSED" });
        let e030 = semver::Version::new(0, 3, 0);
        println!("V   same check if engine were 0.3.0: {}", if r.matches(&e030) { "pass" } else { "REFUSED" });
    }
    let u: UnitDefO2 = ron::from_str(&format!("{base}, health: Some((max: 5)))")).unwrap();
    for req in ["^0.2", "^0.2.1"] {
        println!("V gate health(since 0.2.1) with `{req}`: {:?}", gate(req, &u));
    }
}

fn walk(p: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(p) else { return };
    for e in rd.flatten() {
        let path = e.path();
        let s = path.to_string_lossy();
        if s.contains("/target") || s.contains("node_modules") {
            continue;
        }
        if path.is_dir() {
            walk(&path, out);
        } else if s.contains("/units/") && s.ends_with(".ron") {
            out.push(path);
        }
    }
}
