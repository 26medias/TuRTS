//! Atomic accounting for public query and proposal work.
use serde::{Deserialize, Serialize};
use turts_mod_api::BotQuota;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetUsage {
    pub queries: u32,
    pub candidates: u32,
    pub rows: u32,
    pub proposals: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetDimension {
    Queries,
    Candidates,
    Rows,
    Proposals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetExceeded(pub BudgetDimension);

/// One controller's explicitly allocated quota. A rejected charge changes no counter.
#[derive(Debug, Clone)]
pub struct DecisionBudget {
    limit: BotQuota,
    used: BudgetUsage,
}

impl DecisionBudget {
    pub fn new(limit: BotQuota) -> Self {
        Self {
            limit,
            used: BudgetUsage::default(),
        }
    }
    pub fn used(&self) -> BudgetUsage {
        self.used
    }
    pub fn remaining(&self) -> BotQuota {
        BotQuota {
            queries: self.limit.queries - self.used.queries,
            candidates: self.limit.candidates - self.used.candidates,
            rows: self.limit.rows - self.used.rows,
            proposals: self.limit.proposals - self.used.proposals,
        }
    }
    pub fn charge(&mut self, added: BudgetUsage) -> Result<(), BudgetExceeded> {
        fn next(
            old: u32,
            added: u32,
            limit: u32,
            dimension: BudgetDimension,
        ) -> Result<u32, BudgetExceeded> {
            old.checked_add(added)
                .filter(|value| *value <= limit)
                .ok_or(BudgetExceeded(dimension))
        }
        let used = BudgetUsage {
            queries: next(
                self.used.queries,
                added.queries,
                self.limit.queries,
                BudgetDimension::Queries,
            )?,
            candidates: next(
                self.used.candidates,
                added.candidates,
                self.limit.candidates,
                BudgetDimension::Candidates,
            )?,
            rows: next(
                self.used.rows,
                added.rows,
                self.limit.rows,
                BudgetDimension::Rows,
            )?,
            proposals: next(
                self.used.proposals,
                added.proposals,
                self.limit.proposals,
                BudgetDimension::Proposals,
            )?,
        };
        self.used = used;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_mixed_charge_neither_wraps_nor_consumes_other_dimensions() {
        let mut budget = DecisionBudget::new(BotQuota {
            queries: 10,
            candidates: u32::MAX,
            rows: 2,
            proposals: 1,
        });
        budget
            .charge(BudgetUsage {
                candidates: u32::MAX,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            budget.charge(BudgetUsage {
                queries: 1,
                candidates: 1,
                ..Default::default()
            }),
            Err(BudgetExceeded(BudgetDimension::Candidates))
        );
        assert_eq!(budget.used().queries, 0);
        assert_eq!(budget.remaining().candidates, 0);
        assert_eq!(
            budget.charge(BudgetUsage {
                rows: 3,
                ..Default::default()
            }),
            Err(BudgetExceeded(BudgetDimension::Rows))
        );
        assert_eq!(budget.used().rows, 0);
    }
}
