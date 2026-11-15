//! Private deterministic geology generation. This module does not serialize seeds or columns.
//!
//! Version 1 uses the SplitMix64 finalizer and Weyl stream specified below. Coordinate packing,
//! band ordering, wrapping arithmetic, rejection rule, and choice order are replay contracts.
use super::material::{self, Column, MaterialKey, NaturalInterval};

pub const GENERATOR_VERSION: u32 = 1;
pub const MAX_SELECTION_ATTEMPTS: u32 = 64;
const WEYL_INCREMENT: u64 = 0x9e37_79b9_7f4a_7c15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeightedMaterial {
    pub material: MaterialKey,
    pub weight: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatumBand {
    pub bottom: i32,
    pub top: i32,
    /// Declared order is significant. Every choice must have positive weight.
    pub choices: Vec<WeightedMaterial>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedVersion,
    InvalidDimensions,
    InvalidInitialSurface,
    InvalidBands,
    InvalidWeights,
    WeightOverflow,
    ChoiceLimit,
    OutsideGrid,
    SelectionExhausted,
    Allocation,
    Material(material::Error),
}
impl From<material::Error> for Error {
    fn from(value: material::Error) -> Self {
        Self::Material(value)
    }
}
struct ValidatedBand {
    definition: DatumBand,
    total_weight: u64,
}

/// Server-private generator state. Deliberately has no Debug, seed getter, or wire representation.
pub struct Geology {
    private_seed: u64,
    columns: u32,
    rows: u32,
    floor: i32,
    ceiling: i32,
    maximum_initial: i32,
    bands: Vec<ValidatedBand>,
}
impl Geology {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: u32,
        private_seed: u64,
        columns: u32,
        rows: u32,
        floor: i32,
        ceiling: i32,
        maximum_initial: i32,
        bands: Vec<DatumBand>,
    ) -> Result<Self, Error> {
        if version != GENERATOR_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        if columns == 0 || rows == 0 {
            return Err(Error::InvalidDimensions);
        }
        let cells = u64::from(columns)
            .checked_mul(u64::from(rows))
            .ok_or(Error::InvalidDimensions)?;
        material::storage_bound(cells, floor, ceiling)?;
        if maximum_initial < floor || maximum_initial > ceiling {
            return Err(Error::InvalidInitialSurface);
        }
        // Validate every public band, including bands above any particular generated column.
        let mut expected = floor;
        let mut choice_count = 0u64;
        for band in &bands {
            if band.bottom != expected || band.top <= band.bottom || band.top > maximum_initial {
                return Err(Error::InvalidBands);
            }
            let count = u64::try_from(band.choices.len()).map_err(|_| Error::ChoiceLimit)?;
            choice_count = choice_count.checked_add(count).ok_or(Error::ChoiceLimit)?;
            if choice_count > material::MAX_MATERIAL_STEPS {
                return Err(Error::ChoiceLimit);
            }
            weight_sum(&band.choices)?;
            expected = band.top;
        }
        if expected != maximum_initial {
            return Err(Error::InvalidBands);
        }
        let mut validated = Vec::new();
        validated
            .try_reserve_exact(bands.len())
            .map_err(|_| Error::Allocation)?;
        for definition in bands {
            let total_weight = weight_sum(&definition.choices)?;
            validated.push(ValidatedBand {
                definition,
                total_weight,
            });
        }
        Ok(Self {
            private_seed,
            columns,
            rows,
            floor,
            ceiling,
            maximum_initial,
            bands: validated,
        })
    }

    /// Generate only one clipped column. No full-grid allocation or public seed field is needed.
    pub fn column(&self, x: u32, y: u32, initial_surface: i32) -> Result<Column, Error> {
        if x >= self.columns || y >= self.rows {
            return Err(Error::OutsideGrid);
        }
        if initial_surface < self.floor || initial_surface > self.maximum_initial {
            return Err(Error::InvalidInitialSurface);
        }
        let count = self
            .bands
            .partition_point(|band| band.definition.bottom < initial_surface);
        let mut intervals: Vec<NaturalInterval> = Vec::new();
        intervals
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        // Packing has no host-endian or usize dependence. A column's public initial surface
        // cannot change the material chosen for a retained datum band.
        let coordinate = (u64::from(x) << 32) | u64::from(y);
        let cell_seed = mix64(self.private_seed ^ coordinate);
        for (index, band) in self.bands.iter().take(count).enumerate() {
            let index = u64::try_from(index).map_err(|_| Error::ChoiceLimit)?;
            let mut state = mix64(cell_seed ^ index);
            let ticket = unbiased_ticket(band.total_weight, || next64(&mut state))?;
            let material = material_for_ticket(&band.definition.choices, ticket)?;
            let top = band.definition.top.min(initial_surface);
            if let Some(previous) = intervals.last_mut().filter(|row| row.material == material) {
                previous.top = top;
            } else {
                intervals.push(NaturalInterval {
                    bottom: band.definition.bottom,
                    top,
                    material,
                });
            }
        }
        Column::new(self.floor, self.ceiling, initial_surface, intervals).map_err(Error::from)
    }
}
fn weight_sum(choices: &[WeightedMaterial]) -> Result<u64, Error> {
    if choices.is_empty() {
        return Err(Error::InvalidWeights);
    }
    choices.iter().try_fold(0u64, |total, choice| {
        if choice.weight == 0 {
            return Err(Error::InvalidWeights);
        }
        total
            .checked_add(choice.weight)
            .ok_or(Error::WeightOverflow)
    })
}
fn material_for_ticket(
    choices: &[WeightedMaterial],
    mut ticket: u64,
) -> Result<MaterialKey, Error> {
    for choice in choices {
        if ticket < choice.weight {
            return Ok(choice.material);
        }
        ticket -= choice.weight;
    }
    Err(Error::InvalidWeights)
}
fn unbiased_ticket(total: u64, mut draw: impl FnMut() -> u64) -> Result<u64, Error> {
    if total == 0 {
        return Err(Error::InvalidWeights);
    }
    // [threshold, 2^64) has a size exactly divisible by total. Do not use a fallback
    // remainder when rejection exhausts its finite budget.
    let threshold = total.wrapping_neg() % total;
    for _ in 0..MAX_SELECTION_ATTEMPTS {
        let value = draw();
        if value >= threshold {
            return Ok(value % total);
        }
    }
    Err(Error::SelectionExhausted)
}
fn mix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
fn next64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(WEYL_INCREMENT);
    mix64(*state)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn weighted(material: u32, weight: u64) -> WeightedMaterial {
        WeightedMaterial { material, weight }
    }
    fn band(bottom: i32, top: i32) -> DatumBand {
        DatumBand {
            bottom,
            top,
            choices: (0..16).map(|material| weighted(material, 1)).collect(),
        }
    }
    fn generator(seed: u64, bands: Vec<DatumBand>) -> Geology {
        Geology::new(GENERATOR_VERSION, seed, 4, 3, -4, 4, 2, bands).unwrap()
    }
    #[test]
    fn version1_stream_and_actual_material_golden_vectors_are_frozen() {
        let mut state = 0;
        assert_eq!(next64(&mut state), 0xe220_a839_7b1d_cdaf);
        assert_eq!(next64(&mut state), 0x6e78_9e6a_a1b9_65f4);
        assert_eq!(next64(&mut state), 0x06c4_5d18_8009_454f);
        // Zero seed, (0,0), datum band 0 starts the exact stream above. Its first low nibble
        // is 15; uniform16 choices must select numeric material15, without a modulo fallback.
        let geology = generator(0, vec![band(-4, 2)]);
        let column = geology.column(0, 0, 2).unwrap();
        assert_eq!(
            column.natural_intervals(),
            &[NaturalInterval {
                bottom: -4,
                top: 2,
                material: 15
            }]
        );
        assert_eq!(
            geology.column(0, 0, -1).unwrap().natural_intervals(),
            &[NaturalInterval {
                bottom: -4,
                top: -1,
                material: 15
            }]
        );
        // These packed-coordinate seeds also produce a zero cell key. Keep both axes and
        // their exact bit positions frozen, rather than testing the origin alone.
        for (seed, x, y) in [(0x0000_0002_0000_0001, 2, 1), (0x0000_0003_0000_0002, 3, 2)] {
            let geology = generator(seed, vec![band(-4, 2)]);
            assert_eq!(
                geology.column(x, y, 2).unwrap().natural_intervals(),
                &[NaturalInterval {
                    bottom: -4,
                    top: 2,
                    material: 15
                }]
            );
        }
        // Changing only the material IDs leaves ticket selection unchanged.
        let choices = (100..116).map(|material| weighted(material, 1)).collect();
        let geology = generator(
            0,
            vec![DatumBand {
                bottom: -4,
                top: 2,
                choices,
            }],
        );
        assert_eq!(
            geology.column(0, 0, 2).unwrap().exposed().unwrap().material,
            115
        );
    }
    // Independent wider-integer oracle: explicit low64 masking replaces wrapping u64
    // multiplication, and weight tickets are expanded into slots instead of prefix scanning.
    fn reference_mix(mut value: u128) -> u64 {
        let mask = u128::from(u64::MAX);
        value = ((value ^ (value >> 30)) * 0xbf58_476d_1ce4_e5b9u128) & mask;
        value = ((value ^ (value >> 27)) * 0x94d0_49bb_1331_11ebu128) & mask;
        ((value ^ (value >> 31)) & mask) as u64
    }
    fn reference_material(
        seed: u64,
        x: u32,
        y: u32,
        index: usize,
        choices: &[WeightedMaterial],
    ) -> u32 {
        let coordinate = u64::from(x) * (1u64 << 32) + u64::from(y);
        let cell = reference_mix(u128::from(seed ^ coordinate));
        let mut state = reference_mix(u128::from(cell ^ index as u64));
        let slots = choices
            .iter()
            .flat_map(|choice| (0..choice.weight).map(move |_| choice.material))
            .collect::<Vec<_>>();
        let range = 1u128 << 64;
        let threshold = range % slots.len() as u128;
        for _ in 0..64 {
            state = ((u128::from(state) + u128::from(WEYL_INCREMENT)) % range) as u64;
            let value = reference_mix(u128::from(state));
            if u128::from(value) >= threshold {
                return slots
                    [usize::try_from(value % u64::try_from(slots.len()).unwrap()).unwrap()];
            }
        }
        panic!("reference bounded selection exhausted")
    }
    #[test]
    fn small_grids_match_independent_weight_slot_oracle_at_every_clipped_elevation() {
        let bands = vec![
            DatumBand {
                bottom: -4,
                top: -1,
                choices: vec![weighted(7, 1), weighted(9, 2)],
            },
            DatumBand {
                bottom: -1,
                top: 2,
                choices: vec![weighted(11, 3), weighted(13, 2)],
            },
        ];
        for seed in [0, 1, 9, u64::MAX] {
            let geology = generator(seed, bands.clone());
            for x in 0..4 {
                for y in 0..3 {
                    for surface in -4..=2 {
                        let column = geology.column(x, y, surface).unwrap();
                        let actual = column
                            .natural_intervals()
                            .iter()
                            .flat_map(|row| (row.bottom..row.top).map(move |_| row.material))
                            .collect::<Vec<_>>();
                        let expected = (-4..surface)
                            .map(|elevation| {
                                let index = usize::from(elevation >= -1);
                                reference_material(seed, x, y, index, &bands[index].choices)
                            })
                            .collect::<Vec<_>>();
                        assert_eq!(actual, expected);
                        assert_eq!(column, geology.column(x, y, surface).unwrap());
                        assert!(
                            column.natural_intervals().len()
                                <= usize::try_from(surface + 4).unwrap()
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn rejection_is_unbiased_bounded_and_never_silently_falls_back() {
        let mut calls = 0;
        // total10 rejects 0..5, accepts6. A fallback after the first rejection would select0.
        let ticket = unbiased_ticket(10, || {
            calls += 1;
            if calls == 1 { 0 } else { 6 }
        })
        .unwrap();
        assert_eq!(ticket, 6);
        assert_eq!(calls, 2);
        let mut calls = 0;
        assert_eq!(
            unbiased_ticket(u64::MAX, || {
                calls += 1;
                0
            }),
            Err(Error::SelectionExhausted)
        );
        assert_eq!(calls, MAX_SELECTION_ATTEMPTS);
        assert_eq!(
            unbiased_ticket(0, || panic!("must not draw")),
            Err(Error::InvalidWeights)
        );
        assert_eq!(unbiased_ticket(1, || u64::MAX), Ok(0));
        // Small exact source domain: rejection removes just the incomplete prefix for every
        // bound. Each remaining ticket must have the same number of source values.
        for total in 1u16..=255 {
            let threshold = 256 % total;
            let mut counts = vec![0u16; usize::from(total)];
            for value in threshold..256 {
                counts[usize::from(value % total)] += 1;
            }
            assert!(counts.iter().all(|count| *count == counts[0]));
        }
    }
    #[test]
    fn invalid_authoring_is_rejected_even_above_a_low_column() {
        assert!(matches!(
            Geology::new(0, 0, 1, 1, -4, 4, 2, vec![band(-4, 2)]),
            Err(Error::UnsupportedVersion)
        ));
        assert!(matches!(
            Geology::new(GENERATOR_VERSION + 1, 0, 1, 1, -4, 4, 2, vec![band(-4, 2)]),
            Err(Error::UnsupportedVersion)
        ));
        for bands in [
            vec![],
            vec![band(-3, 2)],
            vec![band(-4, 1)],
            vec![band(-4, 3)],
            vec![band(-4, -4)],
            vec![band(-4, 0), band(1, 2)],
            vec![band(-4, 0), band(-1, 2)],
        ] {
            assert!(matches!(
                Geology::new(GENERATOR_VERSION, 0, 1, 1, -4, 4, 2, bands),
                Err(Error::InvalidBands)
            ));
        }
        for choices in [
            vec![],
            vec![weighted(1, 0)],
            vec![weighted(1, 2), weighted(2, 0)],
        ] {
            let bands = vec![
                band(-4, 0),
                DatumBand {
                    bottom: 0,
                    top: 2,
                    choices,
                },
            ];
            assert!(matches!(
                Geology::new(GENERATOR_VERSION, 0, 1, 1, -4, 4, 2, bands),
                Err(Error::InvalidWeights)
            ));
        }
        let bands = vec![DatumBand {
            bottom: -4,
            top: 2,
            choices: vec![weighted(1, u64::MAX), weighted(2, 1)],
        }];
        assert!(matches!(
            Geology::new(GENERATOR_VERSION, 0, 1, 1, -4, 4, 2, bands),
            Err(Error::WeightOverflow)
        ));
    }
    #[test]
    fn storage_coordinate_surface_and_bedrock_bounds_are_public_and_exact() {
        assert!(matches!(
            Geology::new(GENERATOR_VERSION, 0, 0, 1, -4, 4, 2, vec![]),
            Err(Error::InvalidDimensions)
        ));
        assert!(matches!(
            Geology::new(GENERATOR_VERSION, 0, 1025, 1024, -4, 4, 2, vec![]),
            Err(Error::Material(material::Error::InvalidBounds))
        ));
        assert!(matches!(
            Geology::new(GENERATOR_VERSION, 0, 1024, 1024, -8, 9, 2, vec![]),
            Err(Error::Material(material::Error::StorageLimit))
        ));
        let geology = generator(0, vec![band(-4, 2)]);
        assert_eq!(geology.column(4, 0, 2), Err(Error::OutsideGrid));
        assert_eq!(geology.column(0, 3, 2), Err(Error::OutsideGrid));
        assert_eq!(geology.column(0, 0, -5), Err(Error::InvalidInitialSurface));
        assert_eq!(geology.column(0, 0, 3), Err(Error::InvalidInitialSurface));
        let geology = Geology::new(GENERATOR_VERSION, u64::MAX, 2, 2, -4, 4, -4, vec![]).unwrap();
        let column = geology.column(1, 1, -4).unwrap();
        assert!(column.natural_intervals().is_empty());
        assert_eq!(column.exposed(), None);
        assert_eq!(column.surface(), -4);
    }
}
