use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the TransferHookExecute instruction.
///
/// Token-2022 passes 5 fixed + 7 extra = 12 accounts minimum.
///
///   0. `[]` Source token account
///   1. `[]` Mint
///   2. `[]` Destination token account
///   3. `[]` Source owner / delegate (authority)
///   4. `[]` ExtraAccountMetaList PDA (owned by this program)
///   5. `[]` PermissionManager program (literal)
///   6. `[]` SpikoToken program (literal)
///   7. `[]` TokenConfig PDA (derived from SpikoToken program)
///   8. `[]` Sender's UserPermissions PDA (derived from PermissionManager)
///   9. `[]` Recipient's UserPermissions PDA (derived from PermissionManager)
///  10. `[]` Event authority PDA
///  11. `[]` Self program (this program)
pub struct TransferHookExecuteAccounts<'a> {
    pub source: &'a AccountView,
    pub mint: &'a AccountView,
    pub destination: &'a AccountView,
    pub authority: &'a AccountView,
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub sender_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for TransferHookExecuteAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [source, mint, destination, authority, extra_account_meta_list, permission_manager_program, spiko_token_program, token_config, sender_perms, recipient_perms, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        Ok(Self {
            source,
            mint,
            destination,
            authority,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            token_config,
            sender_perms,
            recipient_perms,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for TransferHookExecuteAccounts<'a> {}
