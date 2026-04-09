//! Dedicated Shank IDL module for the PermissionManager program.
//!
//! This module contains IDL-only struct/enum definitions that mirror the real state layouts
//! and instruction accounts. These are used exclusively by `shank-cli` to generate the IDL
//! JSON and have no runtime impact.

use shank::{ShankAccount, ShankInstruction};

// ---------------------------------------------------------------------------
// Account structs (mirror real state layouts with Shank-friendly types)
// ---------------------------------------------------------------------------

/// IDL mirror of [`crate::state::PermissionConfig`].
///
/// Layout (66 bytes):
///   [0]       discriminator
///   [1]       bump
///   [2..34]   admin (publicKey)
///   [34..66]  pending_admin (publicKey)
#[derive(ShankAccount)]
#[repr(C)]
pub struct PermissionConfig {
    pub discriminator: u8,
    pub bump: u8,
    #[idl_type("publicKey")]
    pub admin: [u8; 32],
    #[idl_type("publicKey")]
    pub pending_admin: [u8; 32],
}

/// IDL mirror of [`crate::state::UserPermissions`].
///
/// Layout (34 bytes):
///   [0]       discriminator
///   [1]       bump
///   [2..34]   roles bitmask (32 bytes = 256 bits)
#[derive(ShankAccount)]
#[repr(C)]
pub struct UserPermissions {
    pub discriminator: u8,
    pub bump: u8,
    pub roles: [u8; 32],
}

// ---------------------------------------------------------------------------
// Instruction enum
// ---------------------------------------------------------------------------

#[derive(ShankInstruction)]
pub enum PermissionManagerInstruction {
    /// Initialize the PermissionManager program.
    /// Creates the PermissionConfig PDA and the admin's UserPermissions PDA.
    #[account(
        0,
        signer,
        writable,
        name = "admin",
        desc = "Admin / deployer (payer, becomes initial admin)"
    )]
    #[account(
        1,
        writable,
        name = "permission_config",
        desc = "PermissionConfig PDA (to be created). Seeds: [\"permission_config\"]"
    )]
    #[account(
        2,
        writable,
        name = "admin_permissions",
        desc = "Admin's UserPermissions PDA (to be created). Seeds: [\"user_perm\", admin]"
    )]
    #[account(3, name = "system_program", desc = "System program")]
    Initialize,

    /// Grant a role to a user.
    /// Caller must be admin or have the required grantor role.
    #[account(
        0,
        signer,
        writable,
        name = "caller",
        desc = "Caller (payer if target PDA needs creation)"
    )]
    #[account(1, name = "permission_config", desc = "PermissionConfig PDA")]
    #[account(
        2,
        writable,
        name = "user_permissions",
        desc = "Target user's UserPermissions PDA (created if needed). Seeds: [\"user_perm\", target_user]"
    )]
    #[account(3, name = "system_program", desc = "System program")]
    #[account(
        4,
        name = "target_user",
        desc = "Target user address (for PDA derivation)"
    )]
    #[account(
        5,
        name = "caller_permissions",
        desc = "Caller's UserPermissions PDA (for role-hierarchy check)"
    )]
    GrantRole { role_id: u8 },

    /// Revoke a role from a user.
    /// Caller must be admin or have the required grantor role.
    #[account(0, signer, name = "caller", desc = "Caller")]
    #[account(1, name = "permission_config", desc = "PermissionConfig PDA")]
    #[account(
        2,
        writable,
        name = "user_permissions",
        desc = "Target user's UserPermissions PDA"
    )]
    #[account(
        3,
        name = "target_user",
        desc = "Target user address (for PDA derivation)"
    )]
    #[account(
        4,
        name = "caller_permissions",
        desc = "Caller's UserPermissions PDA (for role-hierarchy check)"
    )]
    RevokeRole { role_id: u8 },

    /// Transfer admin ownership to a new address (two-step).
    /// Sets the pending_admin field. The new admin must call AcceptOwnership.
    #[account(0, signer, name = "admin", desc = "Current admin")]
    #[account(1, writable, name = "permission_config", desc = "PermissionConfig PDA")]
    TransferOwnership { new_admin: [u8; 32] },

    /// Accept admin ownership.
    /// Caller must be the pending_admin.
    #[account(
        0,
        signer,
        name = "new_admin",
        desc = "New admin (must be the pending_admin)"
    )]
    #[account(1, writable, name = "permission_config", desc = "PermissionConfig PDA")]
    AcceptOwnership,
}
