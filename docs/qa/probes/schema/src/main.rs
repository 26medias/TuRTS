// Probe: can a flat RON struct dispatch fields through a registry table,
// so a new capability is one table row, while a typo still fails?
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HealthDef {
    max: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SightDef {
    radius: f32,
}

#[derive(Debug)]
enum Cap {
    Health(HealthDef),
    Sight(SightDef),
}

// One row per field: name, engine API minor that added it.
const REGISTRY: &[(&str, u32)] = &[("health", 2), ("sight", 3)];

#[derive(Debug)]
struct UnitDef {
    id: String,
    caps: BTreeMap<String, Cap>,
}

struct V;
impl<'de> Visitor<'de> for V {
    type Value = UnitDef;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a unit")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<UnitDef, A::Error> {
        let mut id = None;
        let mut caps = BTreeMap::new();
        while let Some(k) = m.next_key::<String>()? {
            match k.as_str() {
                "id" => id = Some(m.next_value::<String>()?),
                "health" => {
                    caps.insert(k, Cap::Health(m.next_value()?));
                }
                "sight" => {
                    caps.insert(k, Cap::Sight(m.next_value()?));
                }
                other => {
                    let mut names: Vec<&'static str> = vec!["id"];
                    names.extend(REGISTRY.iter().map(|(n, _)| *n));
                    let leaked: &'static [&'static str] = Box::leak(names.into_boxed_slice());
                    return Err(de::Error::unknown_field(other, leaked));
                }
            }
        }
        Ok(UnitDef { id: id.ok_or_else(|| de::Error::missing_field("id"))?, caps })
    }
}
impl<'de> Deserialize<'de> for UnitDef {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_struct("UnitDef", &["id", "health", "sight"], V)
    }
}

fn main() {
    let cases = [
        ("C0 known", r#"(id: "a", health: (max: 5))"#),
        ("C1 new field sight", r#"(id: "a", health: (max: 5), sight: (radius: 7.0))"#),
        ("C2 typo helth", r#"(id: "a", helth: (max: 5))"#),
        ("C3 typo inside block", r#"(id: "a", health: (maxx: 5))"#),
        ("C4 duplicate key", r#"(id: "a", health: (max: 5), health: (max: 6))"#),
    ];
    for (name, text) in cases {
        match ron::from_str::<UnitDef>(text) {
            Ok(u) => println!("{name}: ACCEPTED {:?}", u),
            Err(e) => println!("{name}: REJECTED {:?} at {}:{}", e.code, e.span.start.line, e.span.start.col),
        }
    }
    let _ = REGISTRY.iter().map(|(_, v)| v).count();
}
