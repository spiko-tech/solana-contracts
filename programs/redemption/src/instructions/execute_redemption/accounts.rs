use pinocchio::{account::AccountView, error::ProgramError};

use spiko_common::InstructionAccounts;

/// Accounts for the ExecuteRedemption instruction.
///
///   0.  `[signer]`   Operator (must have ROLE_REDEMPTION_EXECUTOR)
///   1.  `[]`         RedemptionConfig PDA
///   2.  `[writable]` RedemptionOperation PDA
///   3.  `[]`         Operator's UserPermissions PDA (from permission_manager)
///   4.  `[]`         Spiko Token program (for CPI: burn)
///   5.  `[]`         TokenConfig PDA (on spiko-token)
///   6.  `[writable]` Token-2022 Mint
///   7.  `[writable]` Vault token account (source for burn)
///   8.  `[]`         Mint authority PDA (spiko-token)
///   9.  `[]`         Vault authority PDA
///  10.  `[]`         Vault authority's UserPermissions PDA (ROLE_BURNER)
///  11.  `[]`         Token-2022 program
///  12.  `[]`         Spiko Token event authority PDA (for CPI)
///  13.  `[]`         Spiko Token self program (for CPI)
///  14.  `[]`         Event authority PDA
///  15.  `[]`         Self program
pub struct ExecuteRedemptionAccounts<'a> {
    pub operator: &'a AccountView,
    pub config: &'a AccountView,
    pub redemption_op: &'a AccountView,
    pub operator_perms: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub vault: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub vault_authority: &'a AccountView,
    pub vault_authority_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub st_event_authority: &'a AccountView,
    pub st_self_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ExecuteRedemptionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [operator, config, redemption_op, operator_perms, spiko_token_program, token_config, token_mint, vault, mint_authority, vault_authority, vault_authority_perms, token_2022_program, st_event_authority, st_self_program, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !operator.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            operator,
            config,
            redemption_op,
            operator_perms,
            spiko_token_program,
            token_config,
            token_mint,
            vault,
            mint_authority,
            vault_authority,
            vault_authority_perms,
            token_2022_program,
            st_event_authority,
            st_self_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> InstructionAccounts<'a> for ExecuteRedemptionAccounts<'a> {}
