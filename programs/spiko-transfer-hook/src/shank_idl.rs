//! Dedicated Shank IDL module for the SpikoTransferHook program.
//!
//! This module contains IDL-only enum definitions for instruction accounts.
//! These are used exclusively by `shank-cli` to generate the IDL JSON
//! and have no runtime impact.
//!
//! Note: This program has no account structs (no ShankAccount types).
//! The only state it owns is the ExtraAccountMetaList PDA, which uses the
//! SPL TLV format and is not described via ShankAccount.

use shank::ShankInstruction;

// ---------------------------------------------------------------------------
// Instruction enum
// ---------------------------------------------------------------------------

/// Note: TransferHookExecute uses an 8-byte SPL Transfer Hook sighash discriminator
/// (`SHA256("spl-transfer-hook-interface:execute")[0..8]`), not the standard 1-byte discriminator.
/// Shank does not natively support this; the IDL will describe the logical structure and the
/// sighash should be handled by the client.
#[derive(ShankInstruction)]
pub enum SpikoTransferHookInstruction {
    /// Initialize the ExtraAccountMetaList PDA for a given mint.
    /// Must be called once per mint after InitializeToken.
    /// Writes 5 extra account metas that Token-2022 resolves during TransferChecked.
    #[account(0, signer, writable, name = "admin", desc = "Admin / deployer (payer)")]
    #[account(
        1,
        writable,
        name = "extra_account_meta_list",
        desc = "ExtraAccountMetaList PDA (to be created). Seeds: [\"extra-account-metas\", mint]"
    )]
    #[account(2, name = "mint", desc = "Token-2022 Mint")]
    #[account(
        3,
        name = "token_config",
        desc = "TokenConfig PDA (must be initialized, owned by spiko-token)"
    )]
    #[account(4, name = "spiko_token_program", desc = "SpikoToken program")]
    #[account(5, name = "system_program", desc = "System program")]
    InitExtraAccountMetas,

    /// Transfer Hook Execute handler.
    /// Called automatically by Token-2022 on every TransferChecked for mints with
    /// this program as the transfer hook. Enforces: not paused, sender WHITELISTED,
    /// recipient WHITELISTED.
    ///
    /// Discriminator: 8-byte sighash `SHA256("spl-transfer-hook-interface:execute")[0..8]`
    /// = `[105, 37, 101, 197, 75, 251, 102, 26]`
    #[account(0, name = "source", desc = "Source token account")]
    #[account(1, name = "mint", desc = "Token-2022 Mint")]
    #[account(2, name = "destination", desc = "Destination token account")]
    #[account(3, name = "authority", desc = "Source owner / delegate")]
    #[account(
        4,
        name = "extra_account_meta_list",
        desc = "ExtraAccountMetaList PDA (owned by this program)"
    )]
    #[account(
        5,
        name = "permission_manager_program",
        desc = "PermissionManager program (extra account)"
    )]
    #[account(
        6,
        name = "spiko_token_program",
        desc = "SpikoToken program (extra account)"
    )]
    #[account(
        7,
        name = "token_config",
        desc = "TokenConfig PDA (extra account, derived from SpikoToken)"
    )]
    #[account(
        8,
        name = "sender_permissions",
        desc = "Sender's UserPermissions PDA (extra account)"
    )]
    #[account(
        9,
        name = "recipient_permissions",
        desc = "Recipient's UserPermissions PDA (extra account)"
    )]
    TransferHookExecute { amount: u64 },
}
