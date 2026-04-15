use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the InitializeRedemption instruction.
///
///   0. `[signer, writable]` Admin / deployer (payer for PDA creation)
///   1. `[writable]`         RedemptionConfig PDA (to be created)
///   2. `[]`                 System program
///   3. `[]`                 Event authority PDA
///   4. `[]`                 Self program
pub struct InitializeRedemptionAccounts<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub system_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for InitializeRedemptionAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [admin, config, system_program, event_authority, self_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            admin,
            config,
            system_program,
            event_authority,
            self_program,
        })
    }
}
