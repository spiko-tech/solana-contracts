//! Codama instruction definitions for the Redemption program.
//!
//! This file is used exclusively for IDL generation via the `codama` crate.
//! It does NOT replace the actual dispatch logic in `lib.rs`.

use codama::CodamaInstructions;
use pinocchio::Address;

/// Instructions for the Redemption program.
#[allow(clippy::large_enum_variant)]
#[repr(C, u8)]
#[derive(Clone, Debug, PartialEq, CodamaInstructions)]
pub enum RedemptionInstruction {
    /// Initialize the redemption program with an admin and permission manager.
    #[codama(account(
        name = "admin",
        docs = "Admin / deployer (payer for PDA creation)",
        signer,
        writable
    ))]
    #[codama(account(
        name = "config",
        docs = "RedemptionConfig PDA (to be created)",
        writable,
        default_value = pda("redemption_config", [])
    ))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission"
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Redemption program (self) for CPI event emission",
        default_value = public_key("8opABJP3fzXuCVUnbzDZqYpnfxmCmeiXUQ49txf6BFWX")
    ))]
    InitializeRedemption {
        /// Permission manager program ID
        permission_manager: Address,
    } = 0,

    /// Execute a pending redemption operation (burn tokens from vault).
    #[codama(account(name = "operator", docs = "Must have ROLE_REDEMPTION_EXECUTOR", signer))]
    #[codama(account(name = "config", docs = "RedemptionConfig PDA"))]
    #[codama(account(name = "redemption_op", docs = "RedemptionOperation PDA", writable))]
    #[codama(account(
        name = "operator_perms",
        docs = "Operator's UserPermissions PDA (from permission_manager)"
    ))]
    #[codama(account(
        name = "spiko_token_program",
        docs = "Spiko Token program (for CPI: burn)"
    ))]
    #[codama(account(name = "token_config", docs = "TokenConfig PDA (on spiko-token)"))]
    #[codama(account(name = "token_mint", docs = "Token-2022 Mint", writable))]
    #[codama(account(
        name = "vault",
        docs = "Vault token account (source for burn)",
        writable
    ))]
    #[codama(account(name = "mint_authority", docs = "Mint authority PDA (spiko-token)"))]
    #[codama(account(name = "vault_authority", docs = "Vault authority PDA", writable))]
    #[codama(account(
        name = "vault_authority_perms",
        docs = "Vault authority's UserPermissions PDA (ROLE_BURNER)"
    ))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(
        name = "st_event_authority",
        docs = "Spiko Token event authority PDA (for CPI)"
    ))]
    #[codama(account(name = "st_self_program", docs = "Spiko Token self program (for CPI)"))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(
        name = "self_program",
        docs = "Redemption program (self)",
        default_value = public_key("8opABJP3fzXuCVUnbzDZqYpnfxmCmeiXUQ49txf6BFWX")
    ))]
    ExecuteRedemption {
        /// User address
        user: Address,
        /// Amount of tokens
        amount: u64,
        /// Unique salt for operation ID
        salt: u64,
    } = 1,

    /// Cancel a pending redemption operation (refund tokens to user).
    #[codama(account(name = "caller", docs = "Anyone (no permission check)", signer))]
    #[codama(account(name = "config", docs = "RedemptionConfig PDA"))]
    #[codama(account(name = "redemption_op", docs = "RedemptionOperation PDA", writable))]
    #[codama(account(
        name = "vault",
        docs = "Vault token account (source for refund)",
        writable
    ))]
    #[codama(account(
        name = "user_token_account",
        docs = "User's token account (destination for refund)",
        writable
    ))]
    #[codama(account(name = "vault_authority", docs = "Vault authority PDA", writable))]
    #[codama(account(name = "token_mint", docs = "Token-2022 Mint"))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(name = "extra_account_meta_list", docs = "ExtraAccountMetaList PDA"))]
    #[codama(account(
        name = "permission_manager_program",
        docs = "PermissionManager program"
    ))]
    #[codama(account(name = "spiko_token_program", docs = "SpikoToken program"))]
    #[codama(account(name = "token_config", docs = "TokenConfig PDA (spiko-token)"))]
    #[codama(account(
        name = "vault_authority_perms",
        docs = "Vault authority's UserPermissions PDA (sender perms)"
    ))]
    #[codama(account(
        name = "user_perms",
        docs = "User's UserPermissions PDA (recipient perms)"
    ))]
    #[codama(account(name = "hook_event_authority", docs = "Hook event authority PDA"))]
    #[codama(account(name = "hook_program", docs = "Hook program (spiko-transfer-hook)"))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(
        name = "self_program",
        docs = "Redemption program (self)",
        default_value = public_key("8opABJP3fzXuCVUnbzDZqYpnfxmCmeiXUQ49txf6BFWX")
    ))]
    CancelRedemption {
        /// User address
        user: Address,
        /// Amount of tokens
        amount: u64,
        /// Unique salt for operation ID
        salt: u64,
    } = 2,

    /// Set the minimum redemption amount for a token.
    #[codama(account(
        name = "caller",
        docs = "Admin caller (payer if creating TokenMinimum PDA)",
        signer,
        writable
    ))]
    #[codama(account(
        name = "config",
        docs = "RedemptionConfig PDA (to read permission_manager)"
    ))]
    #[codama(account(
        name = "perm_config",
        docs = "PermissionConfig PDA (from permission_manager, for admin check)"
    ))]
    #[codama(account(
        name = "token_minimum",
        docs = "TokenMinimum PDA (created if needed)",
        writable
    ))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(
        name = "self_program",
        docs = "Redemption program (self)",
        default_value = public_key("8opABJP3fzXuCVUnbzDZqYpnfxmCmeiXUQ49txf6BFWX")
    ))]
    SetMinimum {
        /// Token mint address
        token_mint: Address,
        /// Minimum redemption amount
        minimum: u64,
    } = 3,

    /// Called via CPI from spiko-token to initiate a redemption operation.
    #[codama(account(
        name = "token_config",
        docs = "TokenConfig PDA (from spiko_token — proves CPI origin)",
        signer
    ))]
    #[codama(account(
        name = "user",
        docs = "User (payer for PDA creation)",
        signer,
        writable
    ))]
    #[codama(account(name = "config", docs = "RedemptionConfig PDA"))]
    #[codama(account(
        name = "redemption_op",
        docs = "RedemptionOperation PDA (to be created)",
        writable
    ))]
    #[codama(account(name = "token_minimum", docs = "TokenMinimum PDA"))]
    #[codama(account(name = "token_mint", docs = "Token-2022 Mint"))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(
        name = "self_program",
        docs = "Redemption program (self)",
        default_value = public_key("8opABJP3fzXuCVUnbzDZqYpnfxmCmeiXUQ49txf6BFWX")
    ))]
    OnRedeem {
        /// User address
        user_address: Address,
        /// Amount of tokens
        amount: u64,
        /// Unique salt for operation ID
        salt: u64,
    } = 4,

    /// Invoked via CPI to emit event data in instruction args.
    #[codama(skip)]
    EmitEvent {} = 255,
}
