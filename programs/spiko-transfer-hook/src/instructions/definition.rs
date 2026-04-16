//! Codama instruction definitions for the Spiko Transfer Hook program.

use codama::CodamaInstructions;

/// Instructions for the Spiko Transfer Hook program.
#[allow(clippy::large_enum_variant)]
#[repr(C, u8)]
#[derive(Clone, Debug, PartialEq, CodamaInstructions)]
pub enum SpikoTransferHookInstruction {
    /// Initialize the extra account metas for the transfer hook.
    #[codama(account(name = "admin", docs = "Admin / deployer (payer)", signer, writable))]
    #[codama(account(
        name = "extra_account_meta_list",
        docs = "ExtraAccountMetaList PDA to be created",
        writable
    ))]
    #[codama(account(name = "mint", docs = "Token-2022 Mint"))]
    #[codama(account(name = "token_config", docs = "TokenConfig PDA (owned by spiko-token)"))]
    #[codama(account(name = "spiko_token_program", docs = "SpikoToken program", default_value = public_key("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd")))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    InitExtraAccountMetas {} = 0,

    /// Invoked via CPI to emit event data in instruction args.
    #[codama(skip)]
    EmitEvent {} = 255,
}
