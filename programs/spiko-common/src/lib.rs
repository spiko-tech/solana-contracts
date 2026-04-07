//! Shared helpers for Spiko on-chain programs.
//!
//! Provides common utilities that were previously duplicated across
//! multiple programs: PDA verification, PDA account creation, and
//! deterministic operation-ID computation.
//!
//! This crate has NO dependency on any program crate, so it can be
//! used by every program in the workspace (including permission-manager)
//! without introducing cycles.

mod operation_id;
mod pda;

pub use operation_id::compute_operation_id;
pub use pda::{create_pda_account, verify_pda};
