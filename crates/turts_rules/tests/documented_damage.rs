use turts_rules::{
    Ratio,
    damage::{Curve, Hit, applied},
};
const DOC: &str = include_str!("../../../docs/modding/damage.md");
fn ratio(text: &str) -> Ratio {
    let (n, d) = text.split_once('/').unwrap();
    Ratio::new(n.parse().unwrap(), d.parse().unwrap()).unwrap()
}
fn rows(section: &str) -> Vec<Vec<&str>> {
    let start = format!("<!-- damage-{section}:start -->");
    let end = format!("<!-- damage-{section}:end -->");
    DOC.split_once(&start)
        .unwrap()
        .1
        .split_once(&end)
        .unwrap()
        .0
        .lines()
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
#[test]
fn documented_worked_hits_use_their_published_inputs_and_expected_numbers() {
    let cases = rows("examples");
    assert_eq!(cases.len(), 14);
    for row in cases {
        let num = |i: usize| row[i].parse::<u64>().unwrap();
        let sum = |text: &str| {
            text.split('+').try_fold(0_u64, |total, part| {
                total.checked_add(part.parse::<u64>().unwrap())
            })
        };
        let curve = if row[4] == "None" {
            Curve::None
        } else if let Some(text) = row[4].strip_prefix("Ratio(") {
            Curve::Ratio {
                k: ratio(text.trim_end_matches(')')),
            }
        } else {
            let (point, cap) = row[4]
                .strip_prefix("Percent(")
                .unwrap()
                .trim_end_matches(')')
                .split_once(',')
                .unwrap();
            Curve::Percent {
                per_point: ratio(point),
                cap: ratio(cap),
            }
        };
        let hit = sum(row[1]).zip(sum(row[2])).and_then(|(base, bonus)| {
            Hit {
                base,
                bonus,
                cell: ratio(row[3]),
                curve,
                points: num(5),
                flat: num(6),
                minimum: num(7),
            }
            .resolve()
        });
        assert_eq!(hit, row[9].parse::<u64>().ok(), "{}", row[0]);
        assert_eq!(
            hit.map(|hit| applied(hit, num(8))),
            row[10].parse::<u64>().ok(),
            "{} applied",
            row[0]
        );
    }
}
#[test]
fn documented_ratio_rounding_counts_use_the_published_numbers() {
    let cases = rows("kills");
    assert_eq!(cases.len(), 5);
    for row in cases {
        let hit = Hit {
            base: row[0].parse().unwrap(),
            bonus: 0,
            cell: Ratio::ONE,
            curve: Curve::Ratio {
                k: Ratio::new(100, 1).unwrap(),
            },
            points: row[1].parse().unwrap(),
            flat: 0,
            minimum: 1,
        }
        .resolve()
        .unwrap();
        assert_eq!(
            1000_u64.div_ceil(hit),
            row[2].parse::<u64>().unwrap(),
            "{row:?}"
        );
    }
}
