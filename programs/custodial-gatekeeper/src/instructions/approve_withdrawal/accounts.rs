use pinocchio::{account::AccountView, error::ProgramError};

use spiko_common::InstructionAccounts;

/// Accounts for the ApproveWithdrawal instruction.
///
///   0. `[signer]`   Approver (must have ROLE_CUSTODIAL_GATEKEEPER_APPROVER)
///   1. `[]`         GatekeeperConfig PDA
///   2. `[writable]` WithdrawalOperation PDA
///   3. `[]`         Approver's UserPermissions PDA
///   4. `[writable]` Vault token account (source)
///   5. `[writable]` Recipient's token account (destination)
///   6. `[]`         Vault authority PDA
///   7. `[]`         Token-2022 Mint
///   8. `[]`         Vault authority's UserPermissions PDA
///   9. `[]`         Recipient's UserPermissions PDA
///  10. `[]`         Token-2022 program
///  --- Transfer Hook extra accounts ---
///  11. `[]`         ExtraAccountMetaList PDA
///  12. `[]`         PermissionManager program
///  13. `[]`         SpikoToken program
///  14. `[]`         TokenConfig PDA (spiko-token)
///  15. `[]`         Hook event authority PDA
///  16. `[]`         Hook program (spiko-transfer-hook)
///  --- Self-CPI event emission ---
///  17. `[]`         Event authority PDA
///  18. `[]`         Self program
pub struct ApproveWithdrawalAccounts<'a> {
    pub approver: &'a AccountView,
    pub config: &'a AccountView,
    pub withdrawal_op: &'a AccountView,
    pub approver_perms: &'a AccountView,
    pub vault_token_account: &'a AccountView,
    pub recipient_token_account: &'a AccountView,
    pub vault_authority: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub vault_authority_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    // Transfer Hook extra accounts
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub hook_event_authority: &'a AccountView,
    pub hook_program: &'a AccountView,
    // Self-CPI event
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ApproveWithdrawalAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [approver, config, withdrawal_op, approver_perms, vault_token_account, recipient_token_account, vault_authority, token_mint, vault_authority_perms, recipient_perms, token_2022_program, extra_account_meta_list, permission_manager_program, spiko_token_program, token_config, hook_event_authority, hook_program, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !approver.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            approver,
            config,
            withdrawal_op,
            approver_perms,
            vault_token_account,
            recipient_token_account,
            vault_authority,
            token_mint,
            vault_authority_perms,
            recipient_perms,
            token_2022_program,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            token_config,
            hook_event_authority,
            hook_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> InstructionAccounts<'a> for ApproveWithdrawalAccounts<'a> {}
