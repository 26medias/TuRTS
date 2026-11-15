//! Portable bot controllers, bounded observations, and coordinated ordinary orders.
//!
//! The host owns authority and supplies permitted immutable facts. This crate never receives
//! a mutable world or an unrestricted host callback.
pub mod budget;
pub mod seed;

pub use budget::{BudgetDimension, BudgetExceeded, BudgetUsage, DecisionBudget};
pub use seed::derive_seed;
