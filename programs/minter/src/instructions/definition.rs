//! Codama instruction definitions for the Minter program.
//!
//! This file is used exclusively for IDL generation via the `codama` crate.
//! It does NOT replace the actual dispatch logic in `lib.rs`.

use codama::CodamaInstructions;
use pinocchio::Address;

/// Instructions for the Minter program.
#[allow(clippy::large_enum_variant)]
#[repr(C, u8)]
#[derive(Clone, Debug, PartialEq, CodamaInstructions)]
pub enum MinterInstruction {
    /// Initialize the Minter program with an admin and configuration.
    #[codama(account(
        name = "admin",
        docs = "Admin / deployer (payer for PDA creation)",
        signer,
        writable
    ))]
    #[codama(account(
        name = "config",
        docs = "MinterConfig PDA to be created",
        writable,
        default_value = pda("minter_config", [])
    ))]
    #[codama(account(
        name = "system_program",
        docs = "System program",
        default_value = program("system")
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("9iz29VQ2wSuy56NKTz9qBv9mNpFP4bApHtbc2aPzURhh")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Minter program (self) for CPI event emission",
        default_value = public_key("3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2")
    ))]
    InitializeMinter {
        /// Maximum delay in seconds before a blocked mint expires
        max_delay: i64,
        /// Permission manager program address
        permission_manager: Address,
    } = 0,

    /// Initiate a mint operation (immediate or blocked depending on daily limit).
    #[codama(account(
        name = "caller",
        docs = "Caller (must have ROLE_MINT_INITIATOR)",
        signer
    ))]
    #[codama(account(name = "config", docs = "MinterConfig PDA", writable))]
    #[codama(account(name = "daily_limit", docs = "DailyLimit PDA for this token", writable))]
    #[codama(account(
        name = "mint_operation",
        docs = "MintOperation PDA (created if blocked; unused if immediate)",
        writable
    ))]
    #[codama(account(
        name = "caller_perms",
        docs = "Caller's UserPermissions PDA (from permission_manager)"
    ))]
    #[codama(account(
        name = "spiko_token_program",
        docs = "Spiko Token program (for CPI if immediate mint)",
        default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")
    ))]
    #[codama(account(
        name = "token_config",
        docs = "TokenConfig PDA (on spiko-token, for CPI)"
    ))]
    #[codama(account(name = "token_mint", docs = "Token-2022 Mint (for CPI)", writable))]
    #[codama(account(
        name = "recipient_token_account",
        docs = "Recipient's token account (for CPI)",
        writable
    ))]
    #[codama(account(
        name = "mint_authority",
        docs = "Mint authority PDA (spiko-token, for CPI)"
    ))]
    #[codama(account(
        name = "minter_user_perms",
        docs = "Minter's UserPermissions PDA (MinterConfig has ROLE_MINTER)"
    ))]
    #[codama(account(
        name = "recipient_perms",
        docs = "Recipient's UserPermissions PDA (whitelist check, for CPI)"
    ))]
    #[codama(account(
        name = "token_2022_program",
        docs = "Token-2022 program",
        default_value = program("token-2022")
    ))]
    #[codama(account(
        name = "system_program",
        docs = "System program (for PDA creation if blocked)",
        default_value = program("system")
    ))]
    #[codama(account(
        name = "st_event_authority",
        docs = "Spiko Token event authority PDA (for CPI)",
        default_value = public_key("rZRubXuysNdvEFgp7BZb6qkKCos3BocVq8pMKziDHzL")
    ))]
    #[codama(account(
        name = "st_self_program",
        docs = "Spiko Token self program (for CPI)",
        default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("9iz29VQ2wSuy56NKTz9qBv9mNpFP4bApHtbc2aPzURhh")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Minter program (self) for CPI event emission",
        default_value = public_key("3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2")
    ))]
    InitiateMint {
        /// Recipient user address
        user: Address,
        /// Amount of tokens to mint
        amount: u64,
        /// Salt for operation uniqueness
        salt: u64,
    } = 1,

    /// Approve a blocked mint operation and execute the mint.
    #[codama(account(
        name = "caller",
        docs = "Caller (must have ROLE_MINT_APPROVER)",
        signer
    ))]
    #[codama(account(name = "config", docs = "MinterConfig PDA", writable))]
    #[codama(account(name = "mint_operation", docs = "MintOperation PDA", writable))]
    #[codama(account(
        name = "caller_perms",
        docs = "Caller's UserPermissions PDA (from permission_manager)"
    ))]
    #[codama(account(
        name = "spiko_token_program",
        docs = "Spiko Token program (for CPI)",
        default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")
    ))]
    #[codama(account(
        name = "token_config",
        docs = "TokenConfig PDA (on spiko-token, for CPI)"
    ))]
    #[codama(account(name = "token_mint", docs = "Token-2022 Mint (for CPI)", writable))]
    #[codama(account(
        name = "recipient_token_account",
        docs = "Recipient's token account (for CPI)",
        writable
    ))]
    #[codama(account(
        name = "mint_authority",
        docs = "Mint authority PDA (spiko-token, for CPI)"
    ))]
    #[codama(account(
        name = "minter_user_perms",
        docs = "Minter's UserPermissions PDA (MinterConfig has ROLE_MINTER)"
    ))]
    #[codama(account(
        name = "recipient_perms",
        docs = "Recipient's UserPermissions PDA (whitelist check, for CPI)"
    ))]
    #[codama(account(
        name = "token_2022_program",
        docs = "Token-2022 program",
        default_value = program("token-2022")
    ))]
    #[codama(account(
        name = "st_event_authority",
        docs = "Spiko Token event authority PDA (for CPI)",
        default_value = public_key("rZRubXuysNdvEFgp7BZb6qkKCos3BocVq8pMKziDHzL")
    ))]
    #[codama(account(
        name = "st_self_program",
        docs = "Spiko Token self program (for CPI)",
        default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("9iz29VQ2wSuy56NKTz9qBv9mNpFP4bApHtbc2aPzURhh")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Minter program (self) for CPI event emission",
        default_value = public_key("3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2")
    ))]
    ApproveMint {
        /// Recipient user address
        user: Address,
        /// Token mint address (needed to recompute operation_id)
        token_mint_key: Address,
        /// Amount of tokens to mint
        amount: u64,
        /// Salt for operation uniqueness
        salt: u64,
    } = 2,

    /// Cancel a blocked mint operation.
    #[codama(account(
        name = "caller",
        docs = "Caller (must have ROLE_MINT_APPROVER)",
        signer
    ))]
    #[codama(account(name = "config", docs = "MinterConfig PDA"))]
    #[codama(account(name = "mint_operation", docs = "MintOperation PDA", writable))]
    #[codama(account(
        name = "caller_perms",
        docs = "Caller's UserPermissions PDA (from permission_manager)"
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("9iz29VQ2wSuy56NKTz9qBv9mNpFP4bApHtbc2aPzURhh")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Minter program (self) for CPI event emission",
        default_value = public_key("3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2")
    ))]
    CancelMint {
        /// Recipient user address
        user: Address,
        /// Token mint address
        token_mint_key: Address,
        /// Amount of tokens
        amount: u64,
        /// Salt for operation uniqueness
        salt: u64,
    } = 3,

    /// Set or update the daily mint limit for a token.
    #[codama(account(
        name = "caller",
        docs = "Admin caller (payer if creating DailyLimit PDA)",
        signer,
        writable
    ))]
    #[codama(account(
        name = "config",
        docs = "MinterConfig PDA (to read permission_manager)"
    ))]
    #[codama(account(
        name = "perm_config",
        docs = "PermissionConfig PDA (from permission_manager, for admin check)"
    ))]
    #[codama(account(
        name = "daily_limit",
        docs = "DailyLimit PDA for the token (created if needed)",
        writable
    ))]
    #[codama(account(
        name = "system_program",
        docs = "System program",
        default_value = program("system")
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("9iz29VQ2wSuy56NKTz9qBv9mNpFP4bApHtbc2aPzURhh")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Minter program (self) for CPI event emission",
        default_value = public_key("3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2")
    ))]
    SetDailyLimit {
        /// Token mint address (used for PDA derivation)
        token_mint: Address,
        /// Daily mint limit amount
        limit: u64,
    } = 4,

    /// Update the maximum delay for blocked mint operations.
    #[codama(account(name = "caller", docs = "Admin caller", signer))]
    #[codama(account(name = "config", docs = "MinterConfig PDA", writable))]
    #[codama(account(
        name = "perm_config",
        docs = "PermissionConfig PDA (from permission_manager, for admin check)"
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("9iz29VQ2wSuy56NKTz9qBv9mNpFP4bApHtbc2aPzURhh")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Minter program (self) for CPI event emission",
        default_value = public_key("3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2")
    ))]
    SetMaxDelay {
        /// New maximum delay in seconds
        max_delay: i64,
    } = 5,

    /// Invoked via CPI to emit event data in instruction args.
    #[codama(skip)]
    EmitEvent {} = 255,
}
