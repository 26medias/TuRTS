//! Integer material laws and staged, datum-anchored column edits.
//!
//! Account values passed to staging are quantities available to this one step. The simulation
//! must commit the returned account effect and column edit in its own atomic transaction.
use super::Ratio;

pub const MAX_CELLS: u64 = 1_048_576;
pub const MAX_MATERIAL_STEPS: u64 = 16_777_216;
pub type MaterialKey = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    ZeroDimension,
    ZeroDenominator,
    ArithmeticOverflow,
    FractionalAmount,
    AmountLimit,
    ZeroWork,
    InvalidBounds,
    InvalidIntervals,
    StorageLimit,
    Bedrock,
    Ceiling,
    FullOutput,
    InsufficientInput,
    StaleStep,
    InvalidCount,
    RunBoundary,
    Allocation,
}

/// Public worst-case capacity, independent of the selected hidden geology.
pub fn storage_bound(cells: u64, floor: i32, ceiling: i32) -> Result<u64, Error> {
    if cells == 0 || cells > MAX_CELLS || ceiling <= floor {
        return Err(Error::InvalidBounds);
    }
    let span =
        u64::try_from(i64::from(ceiling) - i64::from(floor)).map_err(|_| Error::InvalidBounds)?;
    let steps = cells.checked_mul(span).ok_or(Error::ArithmeticOverflow)?;
    if steps > MAX_MATERIAL_STEPS {
        return Err(Error::StorageLimit);
    }
    Ok(steps)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prism {
    volume: u128,
    world_cube: u128,
}
impl Prism {
    pub fn new(length_scale: u64, cell_width: u64, elevation_step: u64) -> Result<Self, Error> {
        if length_scale == 0 || cell_width == 0 || elevation_step == 0 {
            return Err(Error::ZeroDimension);
        }
        let width = u128::from(cell_width);
        let scale = u128::from(length_scale);
        let volume = width
            .checked_mul(width)
            .and_then(|area| area.checked_mul(u128::from(elevation_step)))
            .ok_or(Error::ArithmeticOverflow)?;
        let world_cube = scale
            .checked_mul(scale)
            .and_then(|square| square.checked_mul(scale))
            .ok_or(Error::ArithmeticOverflow)?;
        Ok(Self { volume, world_cube })
    }
    pub fn volume_quanta(self) -> u128 {
        self.volume
    }
    pub fn world_cube_quanta(self) -> u128 {
        self.world_cube
    }

    /// Check the authored products before division. Do not round or reduce an overflowing ratio.
    pub fn amount(self, coefficient: Ratio, maximum: u64) -> Result<u64, Error> {
        if coefficient.denominator == 0 {
            return Err(Error::ZeroDenominator);
        }
        let numerator = self
            .volume
            .checked_mul(u128::from(coefficient.numerator))
            .ok_or(Error::ArithmeticOverflow)?;
        let denominator = self
            .world_cube
            .checked_mul(u128::from(coefficient.denominator))
            .ok_or(Error::ArithmeticOverflow)?;
        if !numerator.is_multiple_of(denominator) {
            return Err(Error::FractionalAmount);
        }
        let amount = u64::try_from(numerator / denominator).map_err(|_| Error::AmountLimit)?;
        if amount > maximum {
            return Err(Error::AmountLimit);
        }
        Ok(amount)
    }
    pub fn work(self, coefficient: Ratio, maximum: u64) -> Result<u64, Error> {
        let amount = self.amount(coefficient, maximum)?;
        if amount == 0 {
            return Err(Error::ZeroWork);
        }
        Ok(amount)
    }
}

/// Half-open datum elevations [bottom, top). One integer elevation is one complete prism.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaturalInterval {
    pub bottom: i32,
    pub top: i32,
    pub material: MaterialKey,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    Natural,
    NoYield,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exposed {
    pub material: MaterialKey,
    pub provenance: Provenance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedRun {
    pub bottom: i32,
    pub top: i32,
    pub material: MaterialKey,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    floor: i32,
    ceiling: i32,
    initial_surface: i32,
    natural_top: i32,
    surface: i32,
    natural: Vec<NaturalInterval>,
    placed: Vec<PlacedRun>,
    revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepTerms {
    /// Natural yield on removal; authored input on fill. Placed removal always yields zero.
    pub material: Ratio,
    pub work: Ratio,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Remove,
    Fill,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepEffect {
    pub operation: Operation,
    pub steps: u32,
    pub exposed: Exposed,
    /// Credit for Remove, debit for Fill. A physical/account destination is chosen by the adapter.
    pub amount: u64,
    pub required_work: u64,
    pub before: i32,
    pub after: i32,
    pub revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Expected {
    floor: i32,
    ceiling: i32,
    natural_top: i32,
    surface: i32,
    revision: u64,
    exposed: Option<Exposed>,
}
/// A checked edit. Drop to cancel. Its fields cannot be changed by the adapter.
#[derive(Debug)]
pub struct PreparedStep {
    expected: Expected,
    effect: StepEffect,
}
impl PreparedStep {
    pub fn effect(&self) -> StepEffect {
        self.effect
    }
}
impl Column {
    pub fn new(
        floor: i32,
        ceiling: i32,
        initial_surface: i32,
        natural: Vec<NaturalInterval>,
    ) -> Result<Self, Error> {
        storage_bound(1, floor, ceiling)?;
        if initial_surface < floor || initial_surface > ceiling {
            return Err(Error::InvalidBounds);
        }
        let mut expected = floor;
        for interval in &natural {
            if interval.bottom != expected
                || interval.top <= interval.bottom
                || interval.top > initial_surface
            {
                return Err(Error::InvalidIntervals);
            }
            expected = interval.top;
        }
        if expected != initial_surface {
            return Err(Error::InvalidIntervals);
        }
        Ok(Self {
            floor,
            ceiling,
            initial_surface,
            natural_top: initial_surface,
            surface: initial_surface,
            natural,
            placed: Vec::new(),
            revision: 0,
        })
    }
    pub fn floor(&self) -> i32 {
        self.floor
    }
    pub fn ceiling(&self) -> i32 {
        self.ceiling
    }
    pub fn initial_surface(&self) -> i32 {
        self.initial_surface
    }
    pub fn surface(&self) -> i32 {
        self.surface
    }
    pub fn natural_top(&self) -> i32 {
        self.natural_top
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn natural_intervals(&self) -> &[NaturalInterval] {
        &self.natural
    }
    pub fn placed_runs(&self) -> &[PlacedRun] {
        &self.placed
    }
    pub fn exposed(&self) -> Option<Exposed> {
        if let Some(run) = self.placed.last() {
            return Some(Exposed {
                material: run.material,
                provenance: Provenance::NoYield,
            });
        }
        if self.natural_top == self.floor {
            return None;
        }
        let index = self
            .natural
            .partition_point(|row| row.bottom < self.natural_top);
        self.natural.get(index.checked_sub(1)?).map(|row| Exposed {
            material: row.material,
            provenance: Provenance::Natural,
        })
    }
    fn expected(&self) -> Expected {
        Expected {
            floor: self.floor,
            ceiling: self.ceiling,
            natural_top: self.natural_top,
            surface: self.surface,
            revision: self.revision,
            exposed: self.exposed(),
        }
    }
    /// Execution-only run length. It must never enter a preview, query or admission result.
    /// Only the currently exposed interval is inspected, not the material below it.
    pub fn exposed_run_steps(&self) -> u32 {
        self.placed
            .last()
            .map(|row| (i64::from(row.top) - i64::from(row.bottom)) as u32)
            .or_else(|| {
                self.natural
                    .last()
                    .map(|row| (i64::from(row.top) - i64::from(row.bottom)) as u32)
            })
            .unwrap_or(0)
    }
    pub fn stage_remove(
        &self,
        prism: Prism,
        terms: StepTerms,
        output_headroom: u64,
    ) -> Result<PreparedStep, Error> {
        self.stage_remove_many(prism, terms, output_headroom, 1)
    }
    pub fn stage_remove_many(
        &self,
        prism: Prism,
        terms: StepTerms,
        output_headroom: u64,
        steps: u32,
    ) -> Result<PreparedStep, Error> {
        if steps == 0 {
            return Err(Error::InvalidCount);
        }
        let exposed = self.exposed().ok_or(Error::Bedrock)?;
        if steps > self.exposed_run_steps() {
            return Err(Error::RunBoundary);
        }
        let required_work = prism
            .work(terms.work, u64::MAX)?
            .checked_mul(u64::from(steps))
            .ok_or(Error::ArithmeticOverflow)?;
        let per_step = match exposed.provenance {
            Provenance::Natural => prism.amount(terms.material, u64::MAX)?,
            Provenance::NoYield => 0,
        };
        let amount = per_step
            .checked_mul(u64::from(steps))
            .ok_or(Error::ArithmeticOverflow)?;
        if amount > output_headroom {
            return Err(Error::FullOutput);
        }
        let after = i64::from(self.surface)
            .checked_sub(i64::from(steps))
            .and_then(|value| i32::try_from(value).ok());
        self.prepared(
            Operation::Remove,
            exposed,
            amount,
            required_work,
            after,
            steps,
        )
    }
    pub fn stage_fill(
        &self,
        material: MaterialKey,
        prism: Prism,
        terms: StepTerms,
        input_available: u64,
    ) -> Result<PreparedStep, Error> {
        self.stage_fill_many(material, prism, terms, input_available, 1)
    }
    pub fn stage_fill_many(
        &self,
        material: MaterialKey,
        prism: Prism,
        terms: StepTerms,
        input_available: u64,
        steps: u32,
    ) -> Result<PreparedStep, Error> {
        if steps == 0 {
            return Err(Error::InvalidCount);
        }
        if self.surface == self.ceiling {
            return Err(Error::Ceiling);
        }
        let after = i64::from(self.surface)
            .checked_add(i64::from(steps))
            .and_then(|value| i32::try_from(value).ok());
        if after.is_none_or(|after| after > self.ceiling) {
            return Err(Error::Ceiling);
        }
        let required_work = prism
            .work(terms.work, u64::MAX)?
            .checked_mul(u64::from(steps))
            .ok_or(Error::ArithmeticOverflow)?;
        let amount = prism
            .amount(terms.material, u64::MAX)?
            .checked_mul(u64::from(steps))
            .ok_or(Error::ArithmeticOverflow)?;
        if amount > input_available {
            return Err(Error::InsufficientInput);
        }
        self.prepared(
            Operation::Fill,
            Exposed {
                material,
                provenance: Provenance::NoYield,
            },
            amount,
            required_work,
            after,
            steps,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn prepared(
        &self,
        operation: Operation,
        exposed: Exposed,
        amount: u64,
        required_work: u64,
        after: Option<i32>,
        steps: u32,
    ) -> Result<PreparedStep, Error> {
        let after = after.ok_or(Error::ArithmeticOverflow)?;
        if after < self.floor || after > self.ceiling {
            return Err(Error::InvalidBounds);
        }
        let revision = self
            .revision
            .checked_add(u64::from(steps))
            .ok_or(Error::ArithmeticOverflow)?;
        Ok(PreparedStep {
            expected: self.expected(),
            effect: StepEffect {
                operation,
                steps,
                exposed,
                amount,
                required_work,
                before: self.surface,
                after,
                revision,
            },
        })
    }
    /// No logical state changes on error. Resource endpoint validation belongs to the same
    /// enclosing simulation transaction; this method does not mutate an external account.
    pub fn commit(&mut self, prepared: PreparedStep) -> Result<StepEffect, Error> {
        if self.expected() != prepared.expected {
            return Err(Error::StaleStep);
        }
        let effect = prepared.effect;
        match effect.operation {
            Operation::Remove => {
                if let Some(run) = self.placed.last_mut() {
                    run.top = effect.after;
                    if run.top == run.bottom {
                        self.placed.pop();
                    }
                } else {
                    self.natural_top = effect.after;
                    if let Some(interval) = self.natural.last_mut() {
                        interval.top = effect.after;
                        if interval.top == interval.bottom {
                            self.natural.pop();
                        }
                    }
                }
            }
            Operation::Fill => {
                if let Some(run) = self
                    .placed
                    .last_mut()
                    .filter(|run| run.material == effect.exposed.material)
                {
                    run.top = effect.after;
                } else {
                    self.placed.try_reserve(1).map_err(|_| Error::Allocation)?;
                    self.placed.push(PlacedRun {
                        bottom: self.surface,
                        top: effect.after,
                        material: effect.exposed.material,
                        provenance: Provenance::NoYield,
                    });
                }
            }
        }
        self.surface = effect.after;
        self.revision = effect.revision;
        Ok(effect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ONE: Ratio = Ratio {
        numerator: 1,
        denominator: 1,
    };
    const TERMS: StepTerms = StepTerms {
        material: ONE,
        work: ONE,
    };
    fn prism() -> Prism {
        Prism::new(1, 1, 1).unwrap()
    }
    fn column() -> Column {
        Column::new(
            -3,
            5,
            2,
            vec![
                NaturalInterval {
                    bottom: -3,
                    top: -1,
                    material: 7,
                },
                NaturalInterval {
                    bottom: -1,
                    top: 2,
                    material: 9,
                },
            ],
        )
        .unwrap()
    }
    fn remove(column: &mut Column) -> StepEffect {
        let step = column.stage_remove(prism(), TERMS, u64::MAX).unwrap();
        column.commit(step).unwrap()
    }
    fn fill(column: &mut Column, material: u32) -> StepEffect {
        let step = column
            .stage_fill(material, prism(), TERMS, u64::MAX)
            .unwrap();
        column.commit(step).unwrap()
    }
    #[test]
    fn per_world_cube_density_is_exact_across_small_resolutions() {
        // Independent oracle uses integer world-sized cubes: scale³ unit-quanta cubes make
        // one world cube. No production helper computes the expected quotient/remainder.
        for scale in 1u64..=8 {
            for width in 1u64..=8 {
                for height in 1u64..=8 {
                    let prism = Prism::new(scale, width, height).unwrap();
                    for numerator in 0u64..=5 {
                        for denominator in 1u64..=5 {
                            let cubes = width * width * height * numerator;
                            let divisor = scale * scale * scale * denominator;
                            let ratio = Ratio {
                                numerator,
                                denominator,
                            };
                            if cubes.is_multiple_of(divisor) {
                                assert_eq!(prism.amount(ratio, u64::MAX), Ok(cubes / divisor));
                            } else {
                                assert_eq!(
                                    prism.amount(ratio, u64::MAX),
                                    Err(Error::FractionalAmount)
                                );
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(
            Prism::new(1000, 1000, 250).unwrap().amount(
                Ratio {
                    numerator: 4,
                    denominator: 1
                },
                1
            ),
            Ok(1)
        );
        assert_eq!(
            Prism::new(2000, 2000, 500).unwrap().amount(
                Ratio {
                    numerator: 4,
                    denominator: 1
                },
                1
            ),
            Ok(1)
        );
        assert_eq!(
            Prism::new(1000, 2000, 250).unwrap().amount(
                Ratio {
                    numerator: 4,
                    denominator: 1
                },
                4
            ),
            Ok(4)
        );
    }
    #[test]
    fn numeric_limits_fail_before_rounding_or_mutation() {
        assert_eq!(Prism::new(0, 1, 1), Err(Error::ZeroDimension));
        assert_eq!(Prism::new(1, 0, 1), Err(Error::ZeroDimension));
        assert_eq!(Prism::new(1, 1, 0), Err(Error::ZeroDimension));
        assert_eq!(Prism::new(u64::MAX, 1, 1), Err(Error::ArithmeticOverflow));
        assert_eq!(
            Prism::new(1, u64::MAX, u64::MAX),
            Err(Error::ArithmeticOverflow)
        );
        let p = Prism::new(1, u64::MAX, 1).unwrap();
        assert_eq!(
            p.amount(
                Ratio {
                    numerator: 2,
                    denominator: 1
                },
                u64::MAX
            ),
            Err(Error::ArithmeticOverflow)
        );
        assert_eq!(p.amount(ONE, u64::MAX), Err(Error::AmountLimit));
        let p = Prism::new(u64::MAX, 1, 1);
        assert!(p.is_err());
        let p = Prism::new(1u64 << 40, 1, 1).unwrap();
        assert_eq!(
            p.amount(
                Ratio {
                    numerator: 1,
                    denominator: 1u64 << 20
                },
                u64::MAX
            ),
            Err(Error::ArithmeticOverflow)
        );
        assert_eq!(
            prism().amount(
                Ratio {
                    numerator: 1,
                    denominator: 0
                },
                4
            ),
            Err(Error::ZeroDenominator)
        );
        assert_eq!(prism().amount(ONE, 0), Err(Error::AmountLimit));
        assert_eq!(
            prism().work(
                Ratio {
                    numerator: 0,
                    denominator: 1
                },
                9
            ),
            Err(Error::ZeroWork)
        );
        assert_eq!(storage_bound(MAX_CELLS, -8, 8), Ok(MAX_MATERIAL_STEPS));
        assert_eq!(storage_bound(MAX_CELLS, -8, 9), Err(Error::StorageLimit));
        assert_eq!(
            storage_bound(MAX_CELLS + 1, 0, 1),
            Err(Error::InvalidBounds)
        );
        assert_eq!(storage_bound(1, 2, 2), Err(Error::InvalidBounds));
    }
    #[test]
    fn installation_rejects_gaps_overlaps_and_initial_surface_mismatch() {
        for intervals in [
            vec![],
            vec![NaturalInterval {
                bottom: -2,
                top: 2,
                material: 1,
            }],
            vec![NaturalInterval {
                bottom: -3,
                top: 1,
                material: 1,
            }],
            vec![NaturalInterval {
                bottom: -3,
                top: 3,
                material: 1,
            }],
            vec![NaturalInterval {
                bottom: -3,
                top: -3,
                material: 1,
            }],
            vec![
                NaturalInterval {
                    bottom: -3,
                    top: 0,
                    material: 1,
                },
                NaturalInterval {
                    bottom: -1,
                    top: 2,
                    material: 2,
                },
            ],
            vec![
                NaturalInterval {
                    bottom: -3,
                    top: 0,
                    material: 1,
                },
                NaturalInterval {
                    bottom: 1,
                    top: 2,
                    material: 2,
                },
            ],
        ] {
            assert_eq!(
                Column::new(-3, 5, 2, intervals),
                Err(Error::InvalidIntervals)
            );
        }
        assert_eq!(Column::new(-3, 5, 6, vec![]), Err(Error::InvalidBounds));
        let bedrock = Column::new(-3, 5, -3, vec![]).unwrap();
        assert_eq!(bedrock.exposed(), None);
        assert!(matches!(
            bedrock.stage_remove(prism(), TERMS, 10),
            Err(Error::Bedrock)
        ));
    }
    #[test]
    fn natural_datum_and_depletion_survive_fill_remove_cycles() {
        let mut column = column();
        for _ in 0..3 {
            assert_eq!(remove(&mut column).exposed.material, 9);
        }
        assert_eq!(column.natural_top(), -1);
        assert_eq!(column.exposed().unwrap().material, 7);
        for _ in 0..20 {
            fill(&mut column, 9);
            assert_eq!(column.natural_top(), -1);
            let step = remove(&mut column);
            assert_eq!(step.amount, 0);
            assert_eq!(step.exposed.provenance, Provenance::NoYield);
        }
        assert_eq!(
            column.natural_intervals(),
            &[NaturalInterval {
                bottom: -3,
                top: -1,
                material: 7
            }]
        );
        assert_eq!(remove(&mut column).exposed.material, 7);
        assert_eq!(remove(&mut column).exposed.material, 7);
        let before = column.clone();
        assert!(matches!(
            column.stage_remove(prism(), TERMS, 99),
            Err(Error::Bedrock)
        ));
        assert_eq!(column, before);
        fill(&mut column, 7);
        assert_eq!(remove(&mut column).amount, 0);
        assert!(column.natural_intervals().is_empty());
        assert_eq!(column.natural_top(), column.floor());
    }
    #[test]
    fn failed_staging_cancellation_and_stale_commit_keep_all_state() {
        let mut column = column();
        let before = column.clone();
        assert!(matches!(
            column.stage_remove(prism(), TERMS, 0),
            Err(Error::FullOutput)
        ));
        assert!(matches!(
            column.stage_fill(5, prism(), TERMS, 0),
            Err(Error::InsufficientInput)
        ));
        let zero_work = StepTerms {
            work: Ratio {
                numerator: 0,
                denominator: 1,
            },
            ..TERMS
        };
        assert!(matches!(
            column.stage_remove(prism(), zero_work, 9),
            Err(Error::ZeroWork)
        ));
        let fractional = StepTerms {
            material: Ratio {
                numerator: 1,
                denominator: 2,
            },
            ..TERMS
        };
        assert!(matches!(
            column.stage_fill(5, prism(), fractional, 9),
            Err(Error::FractionalAmount)
        ));
        let cancelled = column.stage_remove(prism(), TERMS, 9).unwrap();
        assert_eq!(cancelled.effect().after, 1);
        let _ = cancelled;
        assert_eq!(column, before);
        let stale = column.stage_remove(prism(), TERMS, 9).unwrap();
        fill(&mut column, 5);
        let before = column.clone();
        assert_eq!(column.commit(stale), Err(Error::StaleStep));
        assert_eq!(column, before);
        column.revision = u64::MAX;
        let before = column.clone();
        assert!(matches!(
            column.stage_fill(5, prism(), TERMS, 9),
            Err(Error::ArithmeticOverflow)
        ));
        assert_eq!(column, before);
    }
    #[test]
    fn compact_placed_runs_stay_within_public_span_capacity() {
        let mut column = Column::new(0, 6, 0, vec![]).unwrap();
        fill(&mut column, 1);
        fill(&mut column, 1);
        fill(&mut column, 2);
        fill(&mut column, 1);
        fill(&mut column, 1);
        fill(&mut column, 1);
        assert_eq!(
            column.placed_runs(),
            &[
                PlacedRun {
                    bottom: 0,
                    top: 2,
                    material: 1,
                    provenance: Provenance::NoYield
                },
                PlacedRun {
                    bottom: 2,
                    top: 3,
                    material: 2,
                    provenance: Provenance::NoYield
                },
                PlacedRun {
                    bottom: 3,
                    top: 6,
                    material: 1,
                    provenance: Provenance::NoYield
                },
            ]
        );
        let before = column.clone();
        assert!(matches!(
            column.stage_fill(1, prism(), TERMS, 99),
            Err(Error::Ceiling)
        ));
        assert_eq!(column, before);
        for _ in 0..3 {
            remove(&mut column);
        }
        assert_eq!(column.placed_runs().len(), 2);
        assert_eq!(column.exposed().unwrap().material, 2);
    }
    #[test]
    fn homogeneous_batches_match_exact_sequential_columns_quantities_and_revisions() {
        let terms = StepTerms {
            material: Ratio {
                numerator: 2,
                denominator: 1,
            },
            work: Ratio {
                numerator: 3,
                denominator: 1,
            },
        };
        for count in 1u32..=8 {
            let original = Column::new(
                -8,
                8,
                0,
                vec![NaturalInterval {
                    bottom: -8,
                    top: 0,
                    material: 7,
                }],
            )
            .unwrap();
            let mut batch = original.clone();
            let mut sequential = original;
            let staged = batch.stage_remove_many(prism(), terms, 16, count).unwrap();
            let effect = batch.commit(staged).unwrap();
            let mut total_amount = 0;
            let mut total_work = 0;
            for _ in 0..count {
                let staged = sequential.stage_remove(prism(), terms, 16).unwrap();
                let effect = sequential.commit(staged).unwrap();
                total_amount += effect.amount;
                total_work += effect.required_work;
            }
            assert_eq!(batch, sequential);
            assert_eq!(effect.steps, count);
            assert_eq!(effect.amount, total_amount);
            assert_eq!(effect.required_work, total_work);
            let staged = batch.stage_fill_many(9, prism(), terms, 16, count).unwrap();
            let filled = batch.commit(staged).unwrap();
            for _ in 0..count {
                let staged = sequential.stage_fill(9, prism(), terms, 16).unwrap();
                sequential.commit(staged).unwrap();
            }
            assert_eq!(batch, sequential);
            assert_eq!(filled.amount, u64::from(count) * 2);
            let staged = batch.stage_remove_many(prism(), terms, 0, count).unwrap();
            assert_eq!(batch.commit(staged).unwrap().amount, 0);
            for _ in 0..count {
                let staged = sequential.stage_remove(prism(), terms, 0).unwrap();
                sequential.commit(staged).unwrap();
            }
            assert_eq!(batch, sequential);
            assert_eq!(batch.revision(), u64::from(count) * 3);
        }
    }
    #[test]
    fn batch_boundary_count_capacity_and_overflow_refusals_are_atomic() {
        let mut column = column();
        let before = column.clone();
        assert!(matches!(
            column.stage_remove_many(prism(), TERMS, 99, 0),
            Err(Error::InvalidCount)
        ));
        assert!(matches!(
            column.stage_remove_many(prism(), TERMS, 99, 4),
            Err(Error::RunBoundary)
        ));
        assert!(matches!(
            column.stage_remove_many(prism(), TERMS, 1, 2),
            Err(Error::FullOutput)
        ));
        assert!(matches!(
            column.stage_fill_many(1, prism(), TERMS, 1, 2),
            Err(Error::InsufficientInput)
        ));
        assert!(matches!(
            column.stage_fill_many(1, prism(), TERMS, 99, 4),
            Err(Error::Ceiling)
        ));
        let too_much_work = StepTerms {
            work: Ratio {
                numerator: u64::MAX,
                denominator: 1,
            },
            ..TERMS
        };
        assert!(matches!(
            column.stage_remove_many(prism(), too_much_work, 99, 2),
            Err(Error::ArithmeticOverflow)
        ));
        assert_eq!(column, before);
        let stale = column.stage_remove_many(prism(), TERMS, 99, 2).unwrap();
        remove(&mut column);
        let before = column.clone();
        assert_eq!(column.commit(stale), Err(Error::StaleStep));
        assert_eq!(column, before);
    }

    #[test]
    fn seeded_tapes_match_independent_unit_prism_stack_and_conserve_every_commit() {
        // Oracle stores each individual prism and provenance. It has no intervals, runs,
        // cursor, stage token, or calls to production accounting for its expected quantities.
        for seed in 1u64..=24 {
            let mut rng = seed;
            let mut column = column();
            let mut stack = vec![(7, true), (7, true), (9, true), (9, true), (9, true)];
            let mut input = 1000u64;
            let mut natural_output = 0u64;
            let mut placed_removed = 0u64;
            let mut filled = 0u64;
            for _ in 0..1000 {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let before = column.clone();
                let cancel = (rng >> 12).is_multiple_of(5);
                let fill_next = rng & 1 == 0;
                let available = if rng.is_multiple_of(7) { 0 } else { u64::MAX };
                let staged = if fill_next {
                    column.stage_fill(
                        ((rng >> 16) % 3) as u32,
                        prism(),
                        TERMS,
                        available.min(input),
                    )
                } else {
                    column.stage_remove(prism(), TERMS, available)
                };
                let should_succeed = if fill_next {
                    stack.len() < 8 && available > 0 && input > 0
                } else {
                    stack
                        .last()
                        .is_some_and(|(_, natural)| !natural || available > 0)
                };
                assert_eq!(staged.is_ok(), should_succeed);
                if cancel || !should_succeed {
                    let _ = staged;
                    assert_eq!(column, before);
                    continue;
                }
                let effect = column.commit(staged.unwrap()).unwrap();
                if fill_next {
                    stack.push((((rng >> 16) % 3) as u32, false));
                    input -= 1;
                    filled += 1;
                    assert_eq!(effect.amount, 1);
                } else {
                    let (material, natural) = stack.pop().unwrap();
                    assert_eq!(effect.exposed.material, material);
                    assert_eq!(effect.amount, u64::from(natural));
                    natural_output += u64::from(natural);
                    placed_removed += u64::from(!natural);
                }
                assert_eq!(column.surface(), -3 + i32::try_from(stack.len()).unwrap());
                assert_eq!(
                    column.natural_top(),
                    -3 + i32::try_from(stack.iter().filter(|(_, n)| *n).count()).unwrap()
                );
                assert_eq!(
                    natural_output + stack.iter().filter(|(_, n)| *n).count() as u64,
                    5
                );
                assert_eq!(
                    filled,
                    placed_removed + stack.iter().filter(|(_, n)| !*n).count() as u64
                );
                assert_eq!(1000 - input, filled);
                assert!(column.natural_intervals().len() + column.placed_runs().len() <= 8);
                assert_eq!(
                    column
                        .exposed()
                        .map(|top| (top.material, top.provenance == Provenance::Natural)),
                    stack.last().copied()
                );
            }
        }
    }
}
