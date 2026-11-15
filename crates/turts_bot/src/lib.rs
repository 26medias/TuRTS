//! Portable bot controllers, bounded observations, and coordinated ordinary orders.
//!
//! The host owns authority and supplies permitted immutable facts. This crate never receives
//! a mutable world or an unrestricted host callback.
pub mod budget;
pub mod controller;
pub mod controllers;
pub mod coordinator;
pub mod proposals;
pub mod query;
pub mod registry;
pub mod seed;
pub mod types;

pub use budget::{BudgetDimension, BudgetExceeded, BudgetUsage, DecisionBudget};
pub use seed::derive_seed;

pub use controller::*;
pub use query::*;
pub use types::*;

pub use proposals::*;
pub use registry::*;

pub use coordinator::*;
