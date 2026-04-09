//! Dedicated Shank IDL module for the Minter program.
//!
//! This module contains IDL-only struct/enum definitions that mirror the real state layouts
//! and instruction accounts. These are used exclusively by `shank-cli` to generate the IDL
//! JSON and have no runtime impact.

use shank::{ShankAccount, ShankInstruction};

// ---------------------------------------------------------------------------
// Account structs (mirror real state layouts with Shank-friendly types)
// ---------------------------------------------------------------------------

/// IDL mirror of [`crate::state::MinterConfig`].
///
/// Layout (42 bytes):
///   [0]       discriminator
///   [1]       bump
///   [2..10]   max_delay (i64 LE)
///   [10..42]  permission_manager (publicKey)
#[derive(ShankAccount)]
#[repr(C)]
pub struct MinterConfig {
    pub discriminator: u8,
    pub bump: u8,
    #[idl_type("i64")]
    pub max_delay: [u8; 8],
    #[idl_type("publicKey")]
    pub permission_manager: [u8; 32],
}

/// IDL mirror of [`crate::state::DailyLimit`].
///
/// Layout (26 bytes):
///   [0]       discriminator
///   [1]       bump
///   [2..10]   limit (u64 LE)
///   [10..18]  used_amount (u64 LE)
///   [18..26]  last_day (i64 LE)
#[derive(ShankAccount)]
#[repr(C)]
pub struct DailyLimit {
    pub discriminator: u8,
    pub bump: u8,
    #[idl_type("u64")]
    pub limit: [u8; 8],
    #[idl_type("u64")]
    pub used_amount: [u8; 8],
    #[idl_type("i64")]
    pub last_day: [u8; 8],
}

/// IDL mirror of [`crate::state::MintOperation`].
///
/// Layout (11 bytes):
///   [0]       discriminator
///   [1]       bump
///   [2]       status
///   [3..11]   deadline (i64 LE)
#[derive(ShankAccount)]
#[repr(C, packed)]
pub struct MintOperation {
    pub discriminator: u8,
    pub bump: u8,
    pub status: u8,
    #[idl_type("i64")]
    pub deadline: [u8; 8],
}

// ---------------------------------------------------------------------------
// Instruction enum
// ---------------------------------------------------------------------------

#[derive(ShankInstruction)]
pub enum MinterInstruction {
    /// Initialize the Minter program.
    /// Creates the MinterConfig PDA with the max delay and permission manager address.
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
        name = "minter_config",
        desc = "MinterConfig PDA (to be created). Seeds: [\"minter_config\"]"
    )]
    #[account(2, name = "system_program", desc = "System program")]
    InitializeMinter {
        max_delay: i64,
        permission_manager: [u8; 32],
    },

    /// Initiate a mint operation.
    /// If the amount is within the daily limit, the mint is executed immediately via CPI.
    /// Otherwise, a pending MintOperation PDA is created requiring approval.
    #[account(
        0,
        signer,
        name = "caller",
        desc = "Caller (must have ROLE_MINT_INITIATOR)"
    )]
    #[account(1, name = "minter_config", desc = "MinterConfig PDA")]
    #[account(
        2,
        writable,
        name = "daily_limit",
        desc = "DailyLimit PDA for this token. Seeds: [\"daily_limit\", mint]"
    )]
    #[account(
        3,
        writable,
        name = "mint_operation",
        desc = "MintOperation PDA (created if blocked). Seeds: [\"mint_op\", operation_id]"
    )]
    #[account(
        4,
        name = "caller_permissions",
        desc = "Caller's UserPermissions PDA (from permission_manager)"
    )]
    #[account(
        5,
        name = "spiko_token_program",
        desc = "Spiko Token program (for CPI if immediate mint)"
    )]
    #[account(
        6,
        name = "token_config",
        desc = "TokenConfig PDA (on spiko-token, for CPI)"
    )]
    #[account(7, writable, name = "token_mint", desc = "Token-2022 Mint (for CPI)")]
    #[account(
        8,
        writable,
        name = "recipient_token_account",
        desc = "Recipient's token account (for CPI)"
    )]
    #[account(
        9,
        name = "mint_authority",
        desc = "Mint authority PDA (spiko-token, for CPI)"
    )]
    #[account(
        10,
        name = "minter_user_permissions",
        desc = "Minter's UserPermissions PDA (MinterConfig has ROLE_MINTER)"
    )]
    #[account(
        11,
        name = "recipient_permissions",
        desc = "Recipient's UserPermissions PDA (whitelist check, for CPI)"
    )]
    #[account(12, name = "token_2022_program", desc = "Token-2022 program")]
    #[account(
        13,
        name = "system_program",
        desc = "System program (for PDA creation if blocked)"
    )]
    InitiateMint {
        user: [u8; 32],
        amount: u64,
        salt: u64,
    },

    /// Approve a pending mint operation and execute the mint via CPI.
    /// Caller must have the ROLE_MINT_APPROVER.
    #[account(
        0,
        signer,
        name = "caller",
        desc = "Caller (must have ROLE_MINT_APPROVER)"
    )]
    #[account(1, name = "minter_config", desc = "MinterConfig PDA")]
    #[account(2, writable, name = "mint_operation", desc = "MintOperation PDA")]
    #[account(
        3,
        name = "caller_permissions",
        desc = "Caller's UserPermissions PDA (from permission_manager)"
    )]
    #[account(
        4,
        name = "spiko_token_program",
        desc = "Spiko Token program (for CPI)"
    )]
    #[account(
        5,
        name = "token_config",
        desc = "TokenConfig PDA (on spiko-token, for CPI)"
    )]
    #[account(6, writable, name = "token_mint", desc = "Token-2022 Mint (for CPI)")]
    #[account(
        7,
        writable,
        name = "recipient_token_account",
        desc = "Recipient's token account (for CPI)"
    )]
    #[account(
        8,
        name = "mint_authority",
        desc = "Mint authority PDA (spiko-token, for CPI)"
    )]
    #[account(
        9,
        name = "minter_user_permissions",
        desc = "Minter's UserPermissions PDA (MinterConfig has ROLE_MINTER)"
    )]
    #[account(
        10,
        name = "recipient_permissions",
        desc = "Recipient's UserPermissions PDA (whitelist check, for CPI)"
    )]
    #[account(11, name = "token_2022_program", desc = "Token-2022 program")]
    ApproveMint {
        user: [u8; 32],
        token_mint: [u8; 32],
        amount: u64,
        salt: u64,
    },

    /// Cancel a pending mint operation.
    /// Caller must have the ROLE_MINT_APPROVER. Marks the operation as DONE without minting.
    #[account(
        0,
        signer,
        name = "caller",
        desc = "Caller (must have ROLE_MINT_APPROVER)"
    )]
    #[account(1, name = "minter_config", desc = "MinterConfig PDA")]
    #[account(2, writable, name = "mint_operation", desc = "MintOperation PDA")]
    #[account(
        3,
        name = "caller_permissions",
        desc = "Caller's UserPermissions PDA (from permission_manager)"
    )]
    CancelMint {
        user: [u8; 32],
        token_mint: [u8; 32],
        amount: u64,
        salt: u64,
    },

    /// Set the daily mint limit for a specific token.
    /// Caller must be admin. Creates the DailyLimit PDA if it does not exist.
    #[account(
        0,
        signer,
        writable,
        name = "caller",
        desc = "Admin caller (payer if creating DailyLimit PDA)"
    )]
    #[account(
        1,
        name = "minter_config",
        desc = "MinterConfig PDA (to read permission_manager)"
    )]
    #[account(
        2,
        name = "permission_config",
        desc = "PermissionConfig PDA (from permission_manager, for admin check)"
    )]
    #[account(
        3,
        writable,
        name = "daily_limit",
        desc = "DailyLimit PDA for the token (created if needed). Seeds: [\"daily_limit\", mint]"
    )]
    #[account(4, name = "system_program", desc = "System program")]
    SetDailyLimit { token_mint: [u8; 32], limit: u64 },

    /// Update the max delay on the MinterConfig.
    /// Caller must be admin.
    #[account(0, signer, name = "caller", desc = "Admin caller")]
    #[account(1, writable, name = "minter_config", desc = "MinterConfig PDA")]
    #[account(
        2,
        name = "permission_config",
        desc = "PermissionConfig PDA (from permission_manager, for admin check)"
    )]
    SetMaxDelay { max_delay: i64 },
}
