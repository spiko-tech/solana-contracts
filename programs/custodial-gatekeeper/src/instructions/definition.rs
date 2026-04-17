//! Codama instruction definitions for the CustodialGatekeeper program.
//!
//! This file is used exclusively for IDL generation via the `codama` crate.
//! It does NOT replace the actual dispatch logic in `entrypoint.rs`.

use codama::CodamaInstructions;
use pinocchio::Address;

/// Instructions for the CustodialGatekeeper program.
#[allow(clippy::large_enum_variant)]
#[repr(C, u8)]
#[derive(Clone, Debug, PartialEq, CodamaInstructions)]
pub enum GatekeeperInstruction {
    /// Initialize the gatekeeper program with config.
    #[codama(account(
        name = "admin",
        docs = "Admin / deployer (payer for PDA creation)",
        signer,
        writable
    ))]
    #[codama(account(
        name = "config",
        docs = "GatekeeperConfig PDA (to be created)",
        writable,
        default_value = pda("gatekeeper_config", [])
    ))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission"
    ))]
    #[codama(account(
        name = "self_program",
        docs = "CustodialGatekeeper program (self)",
        default_value = public_key("CGK1111111111111111111111111111111111111111")
    ))]
    Initialize {
        /// Permission manager program ID
        permission_manager: Address,
        /// Maximum delay for pending operations (seconds)
        max_delay: i64,
    } = 0,

    /// Set the daily withdrawal limit for a token. Admin only.
    #[codama(account(
        name = "caller",
        docs = "Admin caller (payer if creating PDA)",
        signer,
        writable
    ))]
    #[codama(account(name = "config", docs = "GatekeeperConfig PDA"))]
    #[codama(account(
        name = "perm_config",
        docs = "PermissionConfig PDA (from permission_manager, for admin check)"
    ))]
    #[codama(account(
        name = "daily_limit",
        docs = "WithdrawalDailyLimit PDA (created if needed)",
        writable
    ))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(
        name = "self_program",
        docs = "CustodialGatekeeper program (self)",
        default_value = public_key("CGK1111111111111111111111111111111111111111")
    ))]
    SetDailyLimit {
        /// Token mint address
        token_mint: Address,
        /// Daily limit amount
        limit: u64,
    } = 1,

    /// Initiate a custodial withdrawal. Tokens go to vault, then either
    /// forwarded instantly (within limit) or held pending approval.
    #[codama(account(
        name = "sender",
        docs = "Custodial wallet (WHITELISTED)",
        signer,
        writable
    ))]
    #[codama(account(name = "config", docs = "GatekeeperConfig PDA"))]
    #[codama(account(name = "daily_limit", docs = "WithdrawalDailyLimit PDA", writable))]
    #[codama(account(
        name = "withdrawal_op",
        docs = "WithdrawalOperation PDA (created if over limit)",
        writable
    ))]
    #[codama(account(
        name = "sender_token_account",
        docs = "Sender's token account",
        writable
    ))]
    #[codama(account(name = "vault_token_account", docs = "Vault token account", writable))]
    #[codama(account(
        name = "recipient_token_account",
        docs = "Recipient's token account",
        writable
    ))]
    #[codama(account(name = "vault_authority", docs = "Vault authority PDA"))]
    #[codama(account(name = "token_mint", docs = "Token-2022 Mint"))]
    #[codama(account(name = "sender_perms", docs = "Sender's UserPermissions PDA"))]
    #[codama(account(name = "recipient_perms", docs = "Recipient's UserPermissions PDA"))]
    #[codama(account(
        name = "vault_authority_perms",
        docs = "Vault authority's UserPermissions PDA"
    ))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(name = "extra_account_meta_list", docs = "ExtraAccountMetaList PDA"))]
    #[codama(account(
        name = "permission_manager_program",
        docs = "PermissionManager program"
    ))]
    #[codama(account(name = "spiko_token_program", docs = "SpikoToken program"))]
    #[codama(account(name = "token_config", docs = "TokenConfig PDA (spiko-token)"))]
    #[codama(account(name = "hook_event_authority", docs = "Hook event authority PDA"))]
    #[codama(account(name = "hook_program", docs = "Hook program (spiko-transfer-hook)"))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(
        name = "self_program",
        docs = "CustodialGatekeeper program (self)",
        default_value = public_key("CGK1111111111111111111111111111111111111111")
    ))]
    CustodialWithdraw {
        /// Recipient address (self-custodial wallet owner)
        recipient: Address,
        /// Amount of tokens
        amount: u64,
        /// Unique salt for operation ID
        salt: u64,
    } = 2,

    /// Approve a pending withdrawal. Requires ROLE_CUSTODIAL_GATEKEEPER_APPROVER.
    #[codama(account(
        name = "approver",
        docs = "Must have ROLE_CUSTODIAL_GATEKEEPER_APPROVER",
        signer
    ))]
    #[codama(account(name = "config", docs = "GatekeeperConfig PDA"))]
    #[codama(account(name = "withdrawal_op", docs = "WithdrawalOperation PDA", writable))]
    #[codama(account(name = "approver_perms", docs = "Approver's UserPermissions PDA"))]
    #[codama(account(name = "vault_token_account", docs = "Vault token account", writable))]
    #[codama(account(
        name = "recipient_token_account",
        docs = "Recipient's token account",
        writable
    ))]
    #[codama(account(name = "vault_authority", docs = "Vault authority PDA"))]
    #[codama(account(name = "token_mint", docs = "Token-2022 Mint"))]
    #[codama(account(
        name = "vault_authority_perms",
        docs = "Vault authority's UserPermissions PDA"
    ))]
    #[codama(account(name = "recipient_perms", docs = "Recipient's UserPermissions PDA"))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(name = "extra_account_meta_list", docs = "ExtraAccountMetaList PDA"))]
    #[codama(account(
        name = "permission_manager_program",
        docs = "PermissionManager program"
    ))]
    #[codama(account(name = "spiko_token_program", docs = "SpikoToken program"))]
    #[codama(account(name = "token_config", docs = "TokenConfig PDA (spiko-token)"))]
    #[codama(account(name = "hook_event_authority", docs = "Hook event authority PDA"))]
    #[codama(account(name = "hook_program", docs = "Hook program (spiko-transfer-hook)"))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(
        name = "self_program",
        docs = "CustodialGatekeeper program (self)",
        default_value = public_key("CGK1111111111111111111111111111111111111111")
    ))]
    ApproveWithdrawal {
        /// Recipient address
        recipient: Address,
        /// Amount of tokens
        amount: u64,
        /// Unique salt for operation ID
        salt: u64,
    } = 3,

    /// Cancel a pending withdrawal after deadline. Anyone can call.
    /// Tokens refunded to sender (custodial wallet).
    #[codama(account(name = "caller", docs = "Anyone (no permission check)", signer))]
    #[codama(account(name = "config", docs = "GatekeeperConfig PDA"))]
    #[codama(account(name = "withdrawal_op", docs = "WithdrawalOperation PDA", writable))]
    #[codama(account(name = "vault_token_account", docs = "Vault token account", writable))]
    #[codama(account(
        name = "sender_token_account",
        docs = "Sender's token account (refund destination)",
        writable
    ))]
    #[codama(account(name = "vault_authority", docs = "Vault authority PDA"))]
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
        docs = "Vault authority's UserPermissions PDA"
    ))]
    #[codama(account(name = "sender_perms", docs = "Sender's UserPermissions PDA"))]
    #[codama(account(name = "hook_event_authority", docs = "Hook event authority PDA"))]
    #[codama(account(name = "hook_program", docs = "Hook program (spiko-transfer-hook)"))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(
        name = "self_program",
        docs = "CustodialGatekeeper program (self)",
        default_value = public_key("CGK1111111111111111111111111111111111111111")
    ))]
    CancelWithdrawal {
        /// Recipient address (used for operation_id derivation)
        recipient: Address,
        /// Amount of tokens
        amount: u64,
        /// Unique salt for operation ID
        salt: u64,
    } = 4,

    /// Invoked via CPI to emit event data in instruction args.
    #[codama(skip)]
    EmitEvent {} = 255,
}
