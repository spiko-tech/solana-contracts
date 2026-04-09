//! Dedicated Shank IDL module for the Redemption program.
//!
//! This module contains IDL-only struct/enum definitions that mirror the real state layouts
//! and instruction accounts. These are used exclusively by `shank-cli` to generate the IDL
//! JSON and have no runtime impact.

use shank::{ShankAccount, ShankInstruction};

// ---------------------------------------------------------------------------
// Account structs (mirror real state layouts with Shank-friendly types)
// ---------------------------------------------------------------------------

/// IDL mirror of [`crate::state::RedemptionConfig`].
///
/// Layout (34 bytes):
///   [0]       discriminator
///   [1]       bump
///   [2..34]   permission_manager (publicKey)
#[derive(ShankAccount)]
#[repr(C)]
pub struct RedemptionConfig {
    pub discriminator: u8,
    pub bump: u8,
    #[idl_type("publicKey")]
    pub permission_manager: [u8; 32],
}

/// IDL mirror of [`crate::state::TokenMinimum`].
///
/// Layout (10 bytes):
///   [0]       discriminator
///   [1]       bump
///   [2..10]   minimum_amount (u64 LE)
#[derive(ShankAccount)]
#[repr(C)]
pub struct TokenMinimum {
    pub discriminator: u8,
    pub bump: u8,
    #[idl_type("u64")]
    pub minimum_amount: [u8; 8],
}

/// IDL mirror of [`crate::state::RedemptionOperation`].
///
/// Layout (44 bytes):
///   [0]       discriminator
///   [1]       bump
///   [2]       status
///   [3]       _padding
///   [4..12]   deadline (i64 LE)
///   [12..44]  user (publicKey)
#[derive(ShankAccount)]
#[repr(C)]
pub struct RedemptionOperation {
    pub discriminator: u8,
    pub bump: u8,
    pub status: u8,
    #[skip]
    _padding: u8,
    #[idl_type("i64")]
    pub deadline: [u8; 8],
    #[idl_type("publicKey")]
    pub user: [u8; 32],
}

// ---------------------------------------------------------------------------
// Instruction enum
// ---------------------------------------------------------------------------

#[derive(ShankInstruction)]
pub enum RedemptionInstruction {
    /// Initialize the Redemption program.
    /// Creates the RedemptionConfig PDA with the permission manager address.
    #[account(
        0,
        signer,
        writable,
        name = "admin",
        desc = "Admin / deployer (payer for PDA creation)"
    )]
    #[account(
        1,
        writable,
        name = "redemption_config",
        desc = "RedemptionConfig PDA (to be created). Seeds: [\"redemption_config\"]"
    )]
    #[account(2, name = "system_program", desc = "System program")]
    InitializeRedemption { permission_manager: [u8; 32] },

    /// Execute a pending redemption operation.
    /// Burns the tokens from the vault. Caller must have ROLE_REDEMPTION_EXECUTOR.
    #[account(
        0,
        signer,
        name = "operator",
        desc = "Operator (must have ROLE_REDEMPTION_EXECUTOR)"
    )]
    #[account(1, name = "redemption_config", desc = "RedemptionConfig PDA")]
    #[account(
        2,
        writable,
        name = "redemption_operation",
        desc = "RedemptionOperation PDA"
    )]
    #[account(
        3,
        name = "operator_permissions",
        desc = "Operator's UserPermissions PDA (from permission_manager)"
    )]
    #[account(
        4,
        name = "spiko_token_program",
        desc = "Spiko Token program (for CPI: burn)"
    )]
    #[account(5, name = "token_config", desc = "TokenConfig PDA (on spiko-token)")]
    #[account(6, writable, name = "token_mint", desc = "Token-2022 Mint")]
    #[account(
        7,
        writable,
        name = "vault",
        desc = "Vault token account (source for burn)"
    )]
    #[account(8, name = "mint_authority", desc = "Mint authority PDA (spiko-token)")]
    #[account(
        9,
        name = "vault_authority",
        desc = "Vault authority PDA. Seeds: [\"vault\"]"
    )]
    #[account(
        10,
        name = "vault_authority_permissions",
        desc = "Vault authority's UserPermissions PDA (ROLE_BURNER)"
    )]
    #[account(11, name = "token_2022_program", desc = "Token-2022 program")]
    ExecuteRedemption {
        user: [u8; 32],
        amount: u64,
        salt: u64,
    },

    /// Cancel a pending redemption operation after deadline has passed.
    /// Anyone can call. Refunds tokens from vault to the user.
    #[account(
        0,
        signer,
        name = "caller",
        desc = "Anyone (no permission check, deadline must have passed)"
    )]
    #[account(1, name = "redemption_config", desc = "RedemptionConfig PDA")]
    #[account(
        2,
        writable,
        name = "redemption_operation",
        desc = "RedemptionOperation PDA"
    )]
    #[account(
        3,
        writable,
        name = "vault",
        desc = "Vault token account (source for refund)"
    )]
    #[account(
        4,
        writable,
        name = "user_token_account",
        desc = "User's token account (destination for refund)"
    )]
    #[account(
        5,
        name = "vault_authority",
        desc = "Vault authority PDA. Seeds: [\"vault\"]"
    )]
    #[account(6, name = "token_mint", desc = "Token-2022 Mint")]
    #[account(7, name = "token_2022_program", desc = "Token-2022 program")]
    #[account(
        8,
        name = "extra_account_meta_list",
        desc = "ExtraAccountMetaList PDA (Transfer Hook)"
    )]
    #[account(
        9,
        name = "permission_manager_program",
        desc = "PermissionManager program"
    )]
    #[account(10, name = "spiko_token_program", desc = "SpikoToken program")]
    #[account(11, name = "token_config", desc = "TokenConfig PDA (spiko-token)")]
    #[account(
        12,
        name = "vault_authority_permissions",
        desc = "Vault authority's UserPermissions PDA (sender perms)"
    )]
    #[account(
        13,
        name = "user_permissions",
        desc = "User's UserPermissions PDA (recipient perms)"
    )]
    #[account(14, name = "hook_program", desc = "Hook program (spiko-transfer-hook)")]
    CancelRedemption {
        user: [u8; 32],
        amount: u64,
        salt: u64,
    },

    /// Set the minimum redemption amount for a specific token.
    /// Caller must be admin. Creates the TokenMinimum PDA if it does not exist.
    #[account(
        0,
        signer,
        writable,
        name = "caller",
        desc = "Admin caller (payer if creating TokenMinimum PDA)"
    )]
    #[account(
        1,
        name = "redemption_config",
        desc = "RedemptionConfig PDA (to read permission_manager)"
    )]
    #[account(
        2,
        name = "permission_config",
        desc = "PermissionConfig PDA (from permission_manager, for admin check)"
    )]
    #[account(
        3,
        writable,
        name = "token_minimum",
        desc = "TokenMinimum PDA (created if needed). Seeds: [\"minimum\", mint]"
    )]
    #[account(4, name = "system_program", desc = "System program")]
    SetMinimum { token_mint: [u8; 32], minimum: u64 },

    /// Called via CPI from spiko_token::redeem only.
    /// Creates a pending RedemptionOperation PDA. The TokenConfig PDA must be a signer
    /// (proving CPI origin from spiko_token).
    #[account(
        0,
        signer,
        name = "token_config",
        desc = "TokenConfig PDA (from spiko_token, proves CPI origin)"
    )]
    #[account(
        1,
        signer,
        writable,
        name = "user",
        desc = "User (payer for PDA creation)"
    )]
    #[account(2, name = "redemption_config", desc = "RedemptionConfig PDA")]
    #[account(
        3,
        writable,
        name = "redemption_operation",
        desc = "RedemptionOperation PDA (to be created). Seeds: [\"redemption_op\", operation_id]"
    )]
    #[account(4, name = "token_minimum", desc = "TokenMinimum PDA")]
    #[account(5, name = "token_mint", desc = "Token-2022 Mint")]
    #[account(6, name = "system_program", desc = "System program")]
    OnRedeem {
        user_address: [u8; 32],
        amount: u64,
        salt: u64,
    },
}
