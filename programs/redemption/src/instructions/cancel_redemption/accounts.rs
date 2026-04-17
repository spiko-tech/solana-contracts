use pinocchio::{account::AccountView, error::ProgramError};

use spiko_common::InstructionAccounts;

/// Accounts for the CancelRedemption instruction.
///
///   0. `[signer]`   Anyone (no permission check)
///   1. `[]`         RedemptionConfig PDA
///   2. `[writable]` RedemptionOperation PDA
///   3. `[writable]` Vault token account (source for refund)
///   4. `[writable]` User's token account (destination for refund)
///   5. `[]`         Vault authority PDA
///   6. `[]`         Token-2022 Mint
///   7. `[]`         Token-2022 program
///   --- Transfer Hook extra accounts ---
///   8. `[]`         ExtraAccountMetaList PDA
///   9. `[]`         PermissionManager program
///  10. `[]`         SpikoToken program
///  11. `[]`         TokenConfig PDA (spiko-token)
///  12. `[]`         Vault authority's UserPermissions PDA (sender perms)
///  13. `[]`         User's UserPermissions PDA (recipient perms)
///  14. `[]`         Hook event authority PDA
///  15. `[]`         Hook program (spiko-transfer-hook)
///  --- Self-CPI event emission ---
///  16. `[]`         Event authority PDA
///  17. `[]`         Self program
pub struct CancelRedemptionAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub redemption_op: &'a AccountView,
    pub vault: &'a AccountView,
    pub user_token_account: &'a AccountView,
    pub vault_authority: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    // Transfer Hook extra accounts
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub vault_authority_perms: &'a AccountView,
    pub user_perms: &'a AccountView,
    pub hook_event_authority: &'a AccountView,
    pub hook_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CancelRedemptionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, redemption_op, vault, user_token_account, vault_authority, token_mint, token_2022_program, extra_account_meta_list, permission_manager_program, spiko_token_program, token_config, vault_authority_perms, user_perms, hook_event_authority, hook_program, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            caller,
            config,
            redemption_op,
            vault,
            user_token_account,
            vault_authority,
            token_mint,
            token_2022_program,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            token_config,
            vault_authority_perms,
            user_perms,
            hook_event_authority,
            hook_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> InstructionAccounts<'a> for CancelRedemptionAccounts<'a> {}
