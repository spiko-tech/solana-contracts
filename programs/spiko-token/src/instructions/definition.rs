//! Codama instruction definitions for the Spiko Token program.
//!
//! This file is used exclusively for IDL generation via the `codama` crate.
//! It does NOT replace the actual dispatch logic in `lib.rs`.

use codama::CodamaInstructions;
use pinocchio::Address;

/// Instructions for the Spiko Token program.
#[allow(clippy::large_enum_variant)]
#[repr(C, u8)]
#[derive(Clone, Debug, PartialEq, CodamaInstructions)]
pub enum SpikoTokenInstruction {
    /// Initialize a new Spiko Token with Token-2022 extensions.
    #[codama(account(name = "admin", docs = "Admin / deployer (payer)", signer, writable))]
    #[codama(account(
        name = "config",
        docs = "TokenConfig PDA to be created",
        writable,
        default_value = pda("tokenConfig", [seed("splMint", account("mint"))])
    ))]
    #[codama(account(
        name = "mint",
        docs = "Token-2022 Mint (pre-allocated by caller)",
        writable
    ))]
    #[codama(account(
        name = "mint_authority",
        docs = "Mint authority PDA",
        default_value = pda("mintAuthority", [seed("mint", account("mint"))])
    ))]
    #[codama(account(name = "permission_manager", docs = "PermissionManager program ID"))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(name = "transfer_hook_program", docs = "Transfer hook program"))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission"
    ))]
    #[codama(account(name = "self_program", docs = "This program for CPI event emission", default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")))]
    InitializeToken {
        /// Number of decimals for the token
        decimals: u8,
        /// Token name
        #[codama(type = string(utf8))]
        #[codama(size_prefix = number(u32))]
        name: String,
        /// Token symbol
        #[codama(type = string(utf8))]
        #[codama(size_prefix = number(u32))]
        symbol: String,
        /// Token URI
        #[codama(type = string(utf8))]
        #[codama(size_prefix = number(u32))]
        uri: String,
    } = 0,

    /// Mint tokens to a recipient.
    #[codama(account(name = "caller", docs = "Must have MINTER role", signer))]
    #[codama(account(name = "config", docs = "TokenConfig PDA"))]
    #[codama(account(name = "mint", docs = "Token-2022 Mint", writable))]
    #[codama(account(
        name = "recipient_token_account",
        docs = "Recipient's token account",
        writable
    ))]
    #[codama(account(
        name = "mint_authority",
        docs = "Mint authority PDA",
        default_value = pda("mintAuthority", [seed("mint", account("mint"))])
    ))]
    #[codama(account(name = "caller_perms", docs = "Caller's UserPermissions PDA"))]
    #[codama(account(
        name = "recipient_perms",
        docs = "Recipient's UserPermissions PDA (whitelist)"
    ))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(name = "self_program", docs = "This program", default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")))]
    MintToken {
        /// Amount of tokens to mint
        amount: u64,
    } = 1,

    /// Burn tokens from caller's own account.
    #[codama(account(name = "caller", docs = "Must have BURNER role", signer))]
    #[codama(account(name = "config", docs = "TokenConfig PDA"))]
    #[codama(account(name = "mint", docs = "Token-2022 Mint", writable))]
    #[codama(account(
        name = "source_token_account",
        docs = "Must be owned by caller",
        writable
    ))]
    #[codama(account(
        name = "mint_authority",
        docs = "Mint authority PDA (authority for burn)",
        default_value = pda("mintAuthority", [seed("mint", account("mint"))])
    ))]
    #[codama(account(name = "caller_perms", docs = "Caller's UserPermissions PDA"))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(name = "self_program", docs = "This program", default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")))]
    BurnToken {
        /// Amount of tokens to burn
        amount: u64,
    } = 2,

    /// Transfer tokens between accounts.
    #[codama(account(name = "sender", docs = "Owner of source token account", signer))]
    #[codama(account(name = "config", docs = "TokenConfig PDA"))]
    #[codama(account(name = "source", docs = "Source token account", writable))]
    #[codama(account(name = "destination", docs = "Destination token account", writable))]
    #[codama(account(name = "mint", docs = "Token-2022 Mint (for TransferChecked)"))]
    #[codama(account(name = "sender_perms", docs = "Sender's UserPermissions PDA"))]
    #[codama(account(name = "recipient_perms", docs = "Recipient's UserPermissions PDA"))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(
        name = "extra_account_meta_list",
        docs = "ExtraAccountMetaList PDA (transfer hook)"
    ))]
    #[codama(account(
        name = "permission_manager_program",
        docs = "PermissionManager program"
    ))]
    #[codama(account(name = "spiko_token_program", docs = "This program (hook resolution)"))]
    #[codama(account(
        name = "hook_event_authority",
        docs = "Transfer Hook event authority PDA"
    ))]
    #[codama(account(name = "hook_program", docs = "spiko-transfer-hook program"))]
    TransferToken {
        /// Amount of tokens to transfer
        amount: u64,
    } = 3,

    /// Pause token operations.
    #[codama(account(name = "caller", docs = "Must have PAUSER role", signer))]
    #[codama(account(name = "config", docs = "TokenConfig PDA", writable))]
    #[codama(account(name = "caller_perms", docs = "Caller's UserPermissions PDA"))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(name = "self_program", docs = "This program", default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")))]
    Pause {} = 4,

    /// Unpause token operations.
    #[codama(account(name = "caller", docs = "Must have PAUSER role", signer))]
    #[codama(account(name = "config", docs = "TokenConfig PDA", writable))]
    #[codama(account(name = "caller_perms", docs = "Caller's UserPermissions PDA"))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(name = "self_program", docs = "This program", default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")))]
    Unpause {} = 5,

    /// Redeem tokens (transfer to vault + create redemption operation via CPI).
    #[codama(account(name = "user", docs = "Token holder", signer, writable))]
    #[codama(account(name = "config", docs = "TokenConfig PDA (signs CPI to redemption)"))]
    #[codama(account(name = "user_source", docs = "User's source token account", writable))]
    #[codama(account(name = "vault", docs = "Vault token account (destination)", writable))]
    #[codama(account(name = "mint", docs = "Token-2022 Mint"))]
    #[codama(account(name = "user_perms", docs = "User's UserPermissions PDA"))]
    #[codama(account(
        name = "vault_authority_perms",
        docs = "Vault authority's UserPermissions PDA"
    ))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(name = "redemption_program", docs = "Redemption program"))]
    #[codama(account(name = "redemption_config", docs = "RedemptionConfig PDA"))]
    #[codama(account(
        name = "redemption_op",
        docs = "RedemptionOperation PDA (to be created)",
        writable
    ))]
    #[codama(account(name = "token_minimum", docs = "TokenMinimum PDA"))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(
        name = "extra_account_meta_list",
        docs = "ExtraAccountMetaList PDA (transfer hook)"
    ))]
    #[codama(account(
        name = "permission_manager_program",
        docs = "PermissionManager program"
    ))]
    #[codama(account(name = "spiko_token_program", docs = "This program (hook resolution)"))]
    #[codama(account(
        name = "hook_event_authority",
        docs = "Transfer Hook event authority PDA"
    ))]
    #[codama(account(name = "hook_program", docs = "spiko-transfer-hook program"))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(name = "self_program", docs = "This program", default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")))]
    #[codama(account(name = "rd_event_authority", docs = "Redemption event authority PDA"))]
    #[codama(account(name = "rd_self_program", docs = "Redemption self program"))]
    RedeemToken {
        /// Amount of tokens to redeem
        amount: u64,
        /// Unique salt for operation ID computation
        salt: u64,
    } = 6,

    /// Set the redemption contract address on the token config.
    #[codama(account(name = "caller", docs = "Admin caller", signer))]
    #[codama(account(name = "config", docs = "TokenConfig PDA", writable))]
    #[codama(account(name = "perm_config", docs = "PermissionConfig PDA (proves admin)"))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(name = "self_program", docs = "This program", default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")))]
    SetRedemptionContract {
        /// Redemption contract address (all zeros to clear)
        redemption_contract: Address,
    } = 7,

    /// Burn tokens from any account (admin-only via PermanentDelegate).
    #[codama(account(name = "caller", docs = "Must be admin", signer))]
    #[codama(account(name = "config", docs = "TokenConfig PDA"))]
    #[codama(account(name = "mint", docs = "Token-2022 Mint", writable))]
    #[codama(account(
        name = "source_token_account",
        docs = "Any account (admin can burn from anyone via PermanentDelegate)",
        writable
    ))]
    #[codama(account(
        name = "mint_authority",
        docs = "Mint authority PDA",
        default_value = pda("mintAuthority", [seed("mint", account("mint"))])
    ))]
    #[codama(account(name = "perm_config", docs = "PermissionConfig PDA (proves admin)"))]
    #[codama(account(name = "token_2022_program", docs = "Token-2022 program", default_value = program("token-2022")))]
    #[codama(account(name = "event_authority", docs = "Event authority PDA"))]
    #[codama(account(name = "self_program", docs = "This program", default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")))]
    BurnFrom {
        /// Amount of tokens to burn
        amount: u64,
    } = 8,

    /// Invoked via CPI to emit event data in instruction args.
    #[codama(skip)]
    EmitEvent {} = 255,
}
