use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the InitExtraAccountMetas instruction.
///
///   0. `[signer, writable]` Admin / deployer (payer)
///   1. `[writable]`         ExtraAccountMetaList PDA (to be created)
///   2. `[]`                 Token-2022 Mint
///   3. `[]`                 TokenConfig PDA (must be initialized, owned by spiko-token)
///   4. `[]`                 SpikoToken program
///   5. `[]`                 System program
pub struct InitExtraAccountMetasAccounts<'a> {
    pub admin: &'a AccountView,
    pub extra_account_meta_list: &'a AccountView,
    pub mint: &'a AccountView,
    pub token_config: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for InitExtraAccountMetasAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [admin, extra_account_meta_list, mint, token_config, spiko_token_program, system_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            admin,
            extra_account_meta_list,
            mint,
            token_config,
            spiko_token_program,
            system_program,
        })
    }
}
