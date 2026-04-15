use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the BurnFrom instruction.
///
///   0. `[signer]`   Caller (must be admin)
///   1. `[]`         TokenConfig PDA
///   2. `[writable]` Token-2022 Mint
///   3. `[writable]` Source token account (any account — admin can burn from anyone)
///   4. `[]`         Mint authority PDA (acts as authority for burn via PermanentDelegate)
///   5. `[]`         PermissionConfig PDA (from permission_manager, proves admin identity)
///   6. `[]`         Token-2022 program
///   7. `[]`         Event authority PDA
///   8. `[]`         Self program (this program)
pub struct BurnFromAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub mint: &'a AccountView,
    pub source_token_account: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub perm_config: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for BurnFromAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, mint, source_token_account, mint_authority, perm_config, token_2022_program, event_authority, self_program, ..] =
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
            source_token_account,
            mint_authority,
            perm_config,
            token_2022_program,
            event_authority,
            self_program,
        })
    }
}
