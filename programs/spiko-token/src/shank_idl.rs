//! Dedicated Shank IDL module for the SpikoToken program.
//!
//! This module contains IDL-only struct/enum definitions that mirror the real state layouts
//! and instruction accounts. These are used exclusively by `shank-cli` to generate the IDL
//! JSON and have no runtime impact.

use shank::{ShankAccount, ShankInstruction};

// ---------------------------------------------------------------------------
// Account structs (mirror real state layouts with Shank-friendly types)
// ---------------------------------------------------------------------------

/// IDL mirror of [`crate::state::TokenConfig`].
///
/// Layout (100 bytes):
///   [0]       discriminator
///   [1]       bump
///   [2]       paused
///   [3]       mint_authority_bump
///   [4..36]   permission_manager (publicKey)
///   [36..68]  spl_mint (publicKey)
///   [68..100] redemption_contract (publicKey)
#[derive(ShankAccount)]
#[repr(C)]
pub struct TokenConfig {
    pub discriminator: u8,
    pub bump: u8,
    pub paused: u8,
    pub mint_authority_bump: u8,
    #[idl_type("publicKey")]
    pub permission_manager: [u8; 32],
    #[idl_type("publicKey")]
    pub spl_mint: [u8; 32],
    #[idl_type("publicKey")]
    pub redemption_contract: [u8; 32],
}

// ---------------------------------------------------------------------------
// Instruction enum
// ---------------------------------------------------------------------------

#[derive(ShankInstruction)]
pub enum SpikoTokenInstruction {
    /// Initialize a new Spiko fund token.
    /// Creates the TokenConfig PDA and initializes the Token-2022 mint with
    /// TransferHook, PermanentDelegate, and MetadataPointer extensions.
    #[account(0, signer, writable, name = "admin", desc = "Admin / deployer (payer)")]
    #[account(
        1,
        writable,
        name = "token_config",
        desc = "TokenConfig PDA (to be created). Seeds: [\"token_config\", mint]"
    )]
    #[account(
        2,
        writable,
        name = "mint",
        desc = "Token-2022 Mint account (pre-allocated with extension space)"
    )]
    #[account(
        3,
        name = "mint_authority",
        desc = "Mint authority PDA. Seeds: [\"mint_authority\", mint]"
    )]
    #[account(4, name = "permission_manager", desc = "PermissionManager program ID")]
    #[account(5, name = "token_2022_program", desc = "Token-2022 program")]
    #[account(6, name = "system_program", desc = "System program")]
    #[account(
        7,
        name = "transfer_hook_program",
        desc = "Transfer Hook program (spiko-transfer-hook)"
    )]
    InitializeToken { decimals: u8, name_len: u32 },

    /// Mint tokens to a recipient.
    /// Caller must have the MINTER role. Recipient must be WHITELISTED.
    #[account(0, signer, name = "caller", desc = "Caller (must have MINTER role)")]
    #[account(1, name = "token_config", desc = "TokenConfig PDA")]
    #[account(2, writable, name = "mint", desc = "Token-2022 Mint")]
    #[account(
        3,
        writable,
        name = "recipient_token_account",
        desc = "Recipient's token account"
    )]
    #[account(4, name = "mint_authority", desc = "Mint authority PDA")]
    #[account(
        5,
        name = "caller_permissions",
        desc = "Caller's UserPermissions PDA (from permission_manager)"
    )]
    #[account(
        6,
        name = "recipient_permissions",
        desc = "Recipient's UserPermissions PDA (whitelist check)"
    )]
    #[account(7, name = "token_2022_program", desc = "Token-2022 program")]
    MintToken { amount: u64 },

    /// Burn tokens from the caller's own token account.
    /// Caller must have the BURNER role and own the source token account.
    #[account(
        0,
        signer,
        name = "caller",
        desc = "Caller (must have BURNER role, must own source account)"
    )]
    #[account(1, name = "token_config", desc = "TokenConfig PDA")]
    #[account(2, writable, name = "mint", desc = "Token-2022 Mint")]
    #[account(
        3,
        writable,
        name = "source_token_account",
        desc = "Source token account (must be owned by caller)"
    )]
    #[account(
        4,
        name = "mint_authority",
        desc = "Mint authority PDA (acts as authority for burn)"
    )]
    #[account(
        5,
        name = "caller_permissions",
        desc = "Caller's UserPermissions PDA (from permission_manager)"
    )]
    #[account(6, name = "token_2022_program", desc = "Token-2022 program")]
    BurnToken { amount: u64 },

    /// Transfer tokens between whitelisted accounts.
    /// Both sender and recipient must be WHITELISTED. Uses TransferChecked with Transfer Hook.
    #[account(
        0,
        signer,
        name = "sender",
        desc = "Sender (owner of source token account)"
    )]
    #[account(1, name = "token_config", desc = "TokenConfig PDA")]
    #[account(2, writable, name = "source", desc = "Source token account")]
    #[account(3, writable, name = "destination", desc = "Destination token account")]
    #[account(4, name = "mint", desc = "Token-2022 Mint")]
    #[account(
        5,
        name = "sender_permissions",
        desc = "Sender's UserPermissions PDA (whitelist check)"
    )]
    #[account(
        6,
        name = "recipient_permissions",
        desc = "Recipient's UserPermissions PDA (whitelist check)"
    )]
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
    #[account(
        10,
        name = "spiko_token_program",
        desc = "SpikoToken program (this program, for hook resolution)"
    )]
    #[account(
        11,
        name = "hook_program",
        desc = "Transfer Hook program (spiko-transfer-hook)"
    )]
    TransferToken { amount: u64 },

    /// Pause the token. No transfers, mints, or burns will be allowed.
    /// Caller must have the PAUSER role.
    #[account(0, signer, name = "caller", desc = "Caller (must have PAUSER role)")]
    #[account(1, writable, name = "token_config", desc = "TokenConfig PDA")]
    #[account(
        2,
        name = "caller_permissions",
        desc = "Caller's UserPermissions PDA (from permission_manager)"
    )]
    Pause,

    /// Unpause the token. Resumes normal operations.
    /// Caller must have the PAUSER role.
    #[account(0, signer, name = "caller", desc = "Caller (must have PAUSER role)")]
    #[account(1, writable, name = "token_config", desc = "TokenConfig PDA")]
    #[account(
        2,
        name = "caller_permissions",
        desc = "Caller's UserPermissions PDA (from permission_manager)"
    )]
    Unpause,

    /// Redeem tokens — transfers tokens to vault and initiates a redemption operation.
    /// User must be WHITELISTED. TokenConfig PDA signs the CPI to the Redemption program.
    #[account(0, signer, name = "user", desc = "User (token holder)")]
    #[account(
        1,
        name = "token_config",
        desc = "TokenConfig PDA (will sign CPI to redemption)"
    )]
    #[account(
        2,
        writable,
        name = "user_source",
        desc = "User's source token account"
    )]
    #[account(
        3,
        writable,
        name = "vault",
        desc = "Vault token account (destination for transfer)"
    )]
    #[account(4, name = "mint", desc = "Token-2022 Mint")]
    #[account(
        5,
        name = "user_permissions",
        desc = "User's UserPermissions PDA (whitelist check)"
    )]
    #[account(
        6,
        name = "vault_authority_permissions",
        desc = "Vault authority's UserPermissions PDA (whitelist check)"
    )]
    #[account(7, name = "token_2022_program", desc = "Token-2022 program")]
    #[account(8, name = "redemption_program", desc = "Redemption program")]
    #[account(
        9,
        name = "redemption_config",
        desc = "RedemptionConfig PDA (from redemption program)"
    )]
    #[account(
        10,
        writable,
        name = "redemption_operation",
        desc = "RedemptionOperation PDA (to be created by redemption)"
    )]
    #[account(
        11,
        name = "token_minimum",
        desc = "TokenMinimum PDA (from redemption program)"
    )]
    #[account(12, name = "system_program", desc = "System program")]
    #[account(
        13,
        name = "extra_account_meta_list",
        desc = "ExtraAccountMetaList PDA (Transfer Hook)"
    )]
    #[account(
        14,
        name = "permission_manager_program",
        desc = "PermissionManager program"
    )]
    #[account(
        15,
        name = "spiko_token_program",
        desc = "SpikoToken program (this program, for hook resolution)"
    )]
    #[account(
        16,
        name = "hook_program",
        desc = "Transfer Hook program (spiko-transfer-hook)"
    )]
    RedeemToken { amount: u64, salt: u64 },

    /// Set the redemption contract address in the TokenConfig.
    /// Caller must be admin.
    #[account(0, signer, name = "caller", desc = "Admin caller")]
    #[account(1, writable, name = "token_config", desc = "TokenConfig PDA")]
    #[account(
        2,
        name = "permission_config",
        desc = "PermissionConfig PDA (from permission_manager, proves admin identity)"
    )]
    SetRedemptionContract { redemption_contract: [u8; 32] },

    /// Burn tokens from any account using the PermanentDelegate authority.
    /// Caller must be admin.
    #[account(0, signer, name = "caller", desc = "Caller (must be admin)")]
    #[account(1, name = "token_config", desc = "TokenConfig PDA")]
    #[account(2, writable, name = "mint", desc = "Token-2022 Mint")]
    #[account(
        3,
        writable,
        name = "source_token_account",
        desc = "Source token account (any account — admin can burn from anyone)"
    )]
    #[account(
        4,
        name = "mint_authority",
        desc = "Mint authority PDA (acts as authority for burn via PermanentDelegate)"
    )]
    #[account(
        5,
        name = "permission_config",
        desc = "PermissionConfig PDA (from permission_manager, proves admin identity)"
    )]
    #[account(6, name = "token_2022_program", desc = "Token-2022 program")]
    BurnFrom { amount: u64 },
}
