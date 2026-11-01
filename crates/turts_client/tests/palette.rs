use turts_client::palette::SLOT_RGB;

// Dichromat matrices from Machado, Oliveira and Fernandes (2009), DOI 10.1109/TVCG.2009.113.
// CIEDE2000 equations and the reference pair follow Sharma, Wu and Dalal (2005).
const MACHADO_2009: [[[f64; 3]; 3]; 3] = [
    [
        [0.152_286, 1.052_583, -0.204_868],
        [0.114_503, 0.786_281, 0.099_216],
        [-0.003_882, -0.048_116, 1.051_998],
    ],
    [
        [0.367_322, 0.860_646, -0.227_968],
        [0.280_085, 0.672_501, 0.047_413],
        [-0.011_820, 0.042_940, 0.968_881],
    ],
    [
        [1.255_528, -0.076_749, -0.178_779],
        [-0.078_411, 0.930_809, 0.147_602],
        [0.004_733, 0.691_367, 0.303_900],
    ],
];

#[test]
fn ciede2000_reference_pair_matches_published_test_vector() {
    let left = Lab(50.0, 2.6772, -79.7751);
    let right = Lab(50.0, 0.0, -82.7485);
    assert!((ciede2000(left, right) - 2.0425).abs() < 0.0001);
}

#[test]
fn every_slot_pair_is_separated_under_all_machado_2009_deficiencies() {
    for matrix in MACHADO_2009 {
        let transformed: Vec<_> = SLOT_RGB
            .iter()
            .map(|rgb| srgb_to_lab(simulate(*rgb, matrix)))
            .collect();
        for left in 0..transformed.len() {
            for right in left + 1..transformed.len() {
                assert!(
                    ciede2000(transformed[left], transformed[right]) >= 20.0,
                    "palette slots {left} and {right} have CIEDE2000 distance {:.3}",
                    ciede2000(transformed[left], transformed[right]),
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Lab(f64, f64, f64);

fn simulate(rgb: [u8; 3], matrix: [[f64; 3]; 3]) -> [f64; 3] {
    let linear = rgb.map(|channel| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    std::array::from_fn(|row| {
        (0..3)
            .map(|column| matrix[row][column] * linear[column])
            .sum::<f64>()
            .clamp(0.0, 1.0)
    })
}

fn srgb_to_lab(rgb: [f64; 3]) -> Lab {
    let [r, g, b] = rgb;
    let x = (0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / 0.950_47;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175_0 * b;
    let z = (0.019_333_9 * r + 0.119_192_0 * g + 0.950_304_1 * b) / 1.088_83;
    let f = |v: f64| {
        if v > 216.0 / 24_389.0 {
            v.cbrt()
        } else {
            (24_389.0 / 27_000.0) * v + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    Lab(116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

// CIEDE2000 with the neutral-axis rotation term and parametric factors set to one.
fn ciede2000(left: Lab, right: Lab) -> f64 {
    let Lab(l1, a1, b1) = left;
    let Lab(l2, a2, b2) = right;
    let c1 = a1.hypot(b1);
    let c2 = a2.hypot(b2);
    let c_bar = (c1 + c2) / 2.0;
    let g = 0.5 * (1.0 - (c_bar.powi(7) / (c_bar.powi(7) + 25.0_f64.powi(7))).sqrt());
    let ap1 = (1.0 + g) * a1;
    let ap2 = (1.0 + g) * a2;
    let cp1 = ap1.hypot(b1);
    let cp2 = ap2.hypot(b2);
    let hp1 = hue(ap1, b1);
    let hp2 = hue(ap2, b2);
    let dl = l2 - l1;
    let dc = cp2 - cp1;
    let dh_angle = hue_delta(hp1, hp2, cp1, cp2);
    let dh = 2.0 * (cp1 * cp2).sqrt() * (dh_angle.to_radians() / 2.0).sin();
    let l_bar = (l1 + l2) / 2.0;
    let cp_bar = (cp1 + cp2) / 2.0;
    let hp_bar = hue_mean(hp1, hp2, cp1, cp2);
    let t = 1.0 - 0.17 * (hp_bar - 30.0).to_radians().cos()
        + 0.24 * (2.0 * hp_bar).to_radians().cos()
        + 0.32 * (3.0 * hp_bar + 6.0).to_radians().cos()
        - 0.20 * (4.0 * hp_bar - 63.0).to_radians().cos();
    let delta_theta = 30.0 * (-((hp_bar - 275.0) / 25.0).powi(2)).exp();
    let rc = 2.0 * (cp_bar.powi(7) / (cp_bar.powi(7) + 25.0_f64.powi(7))).sqrt();
    let sl = 1.0 + 0.015 * (l_bar - 50.0).powi(2) / (20.0 + (l_bar - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * cp_bar;
    let sh = 1.0 + 0.015 * cp_bar * t;
    let rt = -rc * (2.0 * delta_theta).to_radians().sin();
    let x = dl / sl;
    let y = dc / sc;
    let z = dh / sh;
    (x * x + y * y + z * z + rt * y * z).sqrt()
}

fn hue(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        0.0
    } else {
        b.atan2(a).to_degrees().rem_euclid(360.0)
    }
}

fn hue_delta(h1: f64, h2: f64, c1: f64, c2: f64) -> f64 {
    if c1 * c2 == 0.0 {
        0.0
    } else if (h2 - h1).abs() <= 180.0 {
        h2 - h1
    } else if h2 <= h1 {
        h2 - h1 + 360.0
    } else {
        h2 - h1 - 360.0
    }
}

fn hue_mean(h1: f64, h2: f64, c1: f64, c2: f64) -> f64 {
    if c1 * c2 == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        (h1 + h2 - 360.0) / 2.0
    }
}
