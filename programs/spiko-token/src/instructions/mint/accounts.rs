use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the MintToken instruction.
///
///   0. `[signer]`   Caller (must have MINTER role)
///   1. `[]`         TokenConfig PDA
///   2. `[writable]` Token-2022 Mint
///   3. `[writable]` Recipient's token account
///   4. `[]`         Mint authority PDA
///   5. `[]`         Caller's UserPermissions PDA (from permission_manager)
///   6. `[]`         Recipient's UserPermissions PDA (whitelist check)
///   7. `[]`         Token-2022 program
///   8. `[]`         Event authority PDA
///   9. `[]`         Self program (this program)
pub struct MintTokenAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub mint: &'a AccountView,
    pub recipient_token_account: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for MintTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, mint, recipient_token_account, mint_authority, caller_perms, recipient_perms, token_2022_program, event_authority, self_program, ..] =
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
            mint,
            recipient_token_account,
            mint_authority,
            caller_perms,
            recipient_perms,
            token_2022_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for MintTokenAccounts<'a> {}
