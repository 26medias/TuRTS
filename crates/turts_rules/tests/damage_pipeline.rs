use turts_rules::{
    Ratio,
    damage::{Curve, Hit, applied},
};

#[test]
fn pipeline_matches_independent_fraction_cases() {
    let mut count = 0;
    for (line, row) in include_str!("fixtures/damage_pipeline.tsv")
        .lines()
        .enumerate()
    {
        if row.starts_with('#') || row.is_empty() {
            continue;
        }
        let columns = row.split_whitespace().collect::<Vec<_>>();
        let n = |i: usize| columns[i].parse::<u64>().unwrap();
        let ratio = |a, b| Ratio::new(n(a), n(b)).unwrap();
        let curve = match n(4) {
            0 => Curve::None,
            1 => Curve::Ratio { k: ratio(5, 6) },
            2 => Curve::Percent {
                per_point: ratio(5, 6),
                cap: ratio(7, 8),
            },
            _ => panic!("bad fixture curve"),
        };
        let result = Hit {
            base: n(0),
            bonus: n(1),
            cell: ratio(2, 3),
            curve,
            points: n(9),
            flat: n(10),
            minimum: n(11),
        }
        .resolve();
        let expected = columns[13].parse::<u64>().ok();
        assert_eq!(result, expected, "row {}: {}", line + 1, row);
        assert_eq!(
            result.map(|hit| applied(hit, n(12))),
            columns[14].parse::<u64>().ok(),
            "applied row {}",
            line + 1
        );
        count += 1;
    }
    assert!(count >= 40);
}

#[test]
fn ratio_hits_to_kill_include_integer_rounding() {
    for (base, points, expected) in [
        (100, 0, 10),
        (100, 100, 20),
        (100, 200, 31),
        (100, 300, 40),
        (10, 200, 334),
    ] {
        let hit = Hit {
            base,
            bonus: 0,
            cell: Ratio::ONE,
            curve: Curve::Ratio {
                k: Ratio::new(100, 1).unwrap(),
            },
            points,
            flat: 0,
            minimum: 1,
        }
        .resolve()
        .unwrap();
        assert_eq!(
            1000_u64.div_ceil(hit),
            expected,
            "base {base} points {points}"
        );
    }
}
