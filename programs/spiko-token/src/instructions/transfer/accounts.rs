use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the TransferToken instruction.
///
///   0. `[signer]`   Sender (owner of source token account)
///   1. `[]`         TokenConfig PDA
///   2. `[writable]` Source token account
///   3. `[writable]` Destination token account
///   4. `[]`         Token-2022 Mint (needed for TransferChecked)
///   5. `[]`         Sender's UserPermissions PDA (whitelist check)
///   6. `[]`         Recipient's UserPermissions PDA (whitelist check)
///   7. `[]`         Token-2022 program
///   --- Transfer Hook extra accounts ---
///   8. `[]`         ExtraAccountMetaList PDA
///   9. `[]`         PermissionManager program
///  10. `[]`         SpikoToken program (this program, for Token-2022 hook resolution)
///  11. `[]`         Transfer Hook event authority PDA
///  12. `[]`         Transfer Hook program (spiko-transfer-hook)
pub struct TransferTokenAccounts<'a> {
    pub sender: &'a AccountView,
    pub config: &'a AccountView,
    pub source: &'a AccountView,
    pub destination: &'a AccountView,
    pub mint: &'a AccountView,
    pub sender_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    // Transfer Hook extra accounts
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub hook_event_authority: &'a AccountView,
    pub hook_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for TransferTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [sender, config, source, destination, mint, sender_perms, recipient_perms, token_2022_program, extra_account_meta_list, permission_manager_program, spiko_token_program, hook_event_authority, hook_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !sender.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            sender,
            config,
            source,
            destination,
            mint,
            sender_perms,
            recipient_perms,
            token_2022_program,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            hook_event_authority,
            hook_program,
        })
    }
}
