use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the InitializeToken instruction.
///
///   0. `[signer, writable]` Admin / deployer (payer)
///   1. `[writable]`         TokenConfig PDA (to be created)
///   2. `[writable]`         Token-2022 Mint account (must be pre-allocated by the caller
///                            with enough space for base mint + extensions + metadata)
///   3. `[]`                 Mint authority PDA (derived from ["mint_authority", mint])
///   4. `[]`                 PermissionManager program ID (stored in config)
///   5. `[]`                 Token-2022 program
///   6. `[]`                 System program
///   7. `[]`                 Transfer Hook program (spiko-transfer-hook)
///   8. `[]`                 Event authority PDA
///   9. `[]`                 Self program (this program)
pub struct InitializeTokenAccounts<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub mint: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub permission_manager: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub system_program: &'a AccountView,
    pub transfer_hook_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for InitializeTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [admin, config, mint, mint_authority, permission_manager, token_2022_program, system_program, transfer_hook_program, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            admin,
            config,
            mint,
            mint_authority,
            permission_manager,
            token_2022_program,
            system_program,
            transfer_hook_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for InitializeTokenAccounts<'a> {}
