//! Cross-program permission helpers for Spiko on-chain programs.
//!
//! Provides `require_permission` and `require_admin` which read
//! `permission-manager`'s on-chain state directly (no CPI) to verify
//! that a caller holds the required role or is the admin.
//!
//! This crate depends on `permission-manager` (for its state types)
//! and `spiko-common` (for `verify_pda`).  Programs that need
//! permission checks depend on this crate; `permission-manager` itself
//! does NOT depend on it, so there are no cycles.

mod permissions;

pub use permissions::{require_permission, require_admin};
