use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the Initialize instruction.
///
///   0. `[signer, writable]` Admin / deployer (payer, becomes initial admin)
///   1. `[writable]`         PermissionConfig PDA (to be created)
///   2. `[writable]`         Admin's UserPermissions PDA (to be created)
///   3. `[]`                 System program
///   4. `[]`                 Event authority PDA
///   5. `[]`                 Self program (this program)
pub struct InitializeAccounts<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub admin_perms: &'a AccountView,
    pub system_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for InitializeAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [admin, config, admin_perms, system_program, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if system_program.address() != &pinocchio_system::ID {
            return Err(ProgramError::IncorrectProgramId);
        }

        Ok(Self {
            admin,
            config,
            admin_perms,
            system_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for InitializeAccounts<'a> {}
