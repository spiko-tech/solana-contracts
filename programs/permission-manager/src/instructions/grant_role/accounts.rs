use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the GrantRole instruction.
///
///   0. `[signer, writable]` Caller (payer if target PDA needs creation)
///   1. `[]`                 PermissionConfig PDA
///   2. `[writable]`         Target user's UserPermissions PDA (created if needed)
///   3. `[]`                 System program
///   4. `[]`                 Target user address (for PDA derivation, not a signer)
///   5. `[]`                 Caller's UserPermissions PDA (for role-hierarchy check)
///   6. `[]`                 Event authority PDA
///   7. `[]`                 Self program (this program)
pub struct GrantRoleAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub user_perms: &'a AccountView,
    pub system_program: &'a AccountView,
    pub target_user: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for GrantRoleAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, user_perms, system_program, target_user, caller_perms, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if system_program.address() != &pinocchio_system::ID {
            return Err(ProgramError::IncorrectProgramId);
        }

        Ok(Self {
            caller,
            config,
            user_perms,
            system_program,
            target_user,
            caller_perms,
            event_authority,
            self_program,
        })
    }
}
